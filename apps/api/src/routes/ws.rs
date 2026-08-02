use std::collections::HashSet;
use std::net::SocketAddr;
use std::time::{Duration, Instant};

use axum::{
    extract::{
        ws::{Message, WebSocket},
        ConnectInfo, Query, State, WebSocketUpgrade,
    },
    response::IntoResponse,
};
use futures_util::{SinkExt, StreamExt};
use redis::AsyncCommands;
use serde::Deserialize;
use tokio::sync::broadcast;
use tokio::time::interval;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::realtime::{ClientMessage, ServerMessage};
use crate::state::AppState;

const CONNECT_RATE_WINDOW_SECS: u64 = 60;

#[derive(Debug, Deserialize)]
pub struct WsConnectQuery {
    pub access_token: Option<String>,
}

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Query(query): Query<WsConnectQuery>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> AppResult<impl IntoResponse> {
    let token = query
        .access_token
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or(AppError::Unauthorized)?;

    enforce_connect_rate(&state, addr).await?;

    let claims = state
        .auth()
        .token_service()
        .decode_access_token(token)
        .map_err(|_| AppError::Unauthorized)?;
    let principal = state
        .authorization()
        .authenticate(claims)
        .await
        .map_err(|_| AppError::Unauthorized)?;

    let user_id = principal.user_id;
    let (_id, event_rx) = state
        .realtime()
        .hub()
        .subscribe(user_id)
        .await
        .map_err(|_| AppError::RateLimited)?;

    state.presence().touch(user_id).await;

    Ok(ws.on_upgrade(move |socket| handle_socket(socket, state, user_id, event_rx)))
}

async fn handle_socket(
    socket: WebSocket,
    state: AppState,
    user_id: Uuid,
    mut event_rx: broadcast::Receiver<ServerMessage>,
) {
    let (mut sender, mut receiver) = socket.split();
    let heartbeat = Duration::from_secs(state.config().ws_heartbeat_secs);
    let idle_timeout = Duration::from_secs(state.config().ws_idle_timeout_secs);
    let mut heartbeat_tick = interval(heartbeat);
    let mut last_client_activity = Instant::now();
    // Polls this connection is watching (limit guards against abuse).
    let mut poll_subscriptions: HashSet<Uuid> = HashSet::new();
    let mut poll_rx = state.realtime().subscribe_poll_events();

    if sender
        .send(json_message(&ServerMessage::connected(user_id)))
        .await
        .is_err()
    {
        state.realtime().hub().unsubscribe(user_id).await;
        return;
    }

    loop {
        tokio::select! {
            message = receiver.next() => {
                match message {
                    Some(Ok(Message::Text(text))) => {
                        last_client_activity = Instant::now();
                        match serde_json::from_str::<ClientMessage>(text.as_ref()) {
                            Ok(client) if client.type_ == "ping" => {
                                state.presence().touch(user_id).await;
                                if sender.send(json_message(&ServerMessage::pong())).await.is_err() {
                                    break;
                                }
                            }
                            Ok(client) if client.type_ == "subscribe.presence" => {
                                match presence_updates(&state, &client).await {
                                    Ok(updates) => {
                                        for update in updates {
                                            if sender.send(json_message(&update)).await.is_err() {
                                                break;
                                            }
                                        }
                                    }
                                    Err(message) => {
                                        let _ = sender
                                            .send(json_message(&ServerMessage::error(message)))
                                            .await;
                                    }
                                }
                            }
                            Ok(client) if client.type_ == "subscribe.poll" => {
                                match parse_single_uuid(&client, "poll_id") {
                                    Ok(poll_id) => {
                                        if poll_subscriptions.len() >= 64 {
                                            let _ = sender
                                                .send(json_message(&ServerMessage::error(
                                                    "too many poll subscriptions",
                                                )))
                                                .await;
                                        } else {
                                            poll_subscriptions.insert(poll_id);
                                        }
                                    }
                                    Err(message) => {
                                        let _ = sender
                                            .send(json_message(&ServerMessage::error(message)))
                                            .await;
                                    }
                                }
                            }
                            Ok(client) if client.type_ == "unsubscribe.poll" => {
                                if let Ok(poll_id) = parse_single_uuid(&client, "poll_id") {
                                    poll_subscriptions.remove(&poll_id);
                                }
                            }
                            Ok(_) => {
                                let _ = sender
                                    .send(json_message(&ServerMessage::error("unknown message type")))
                                    .await;
                            }
                            Err(_) => {
                                let _ = sender
                                    .send(json_message(&ServerMessage::error("invalid message")))
                                    .await;
                            }
                        }
                    }
                    Some(Ok(Message::Ping(payload))) => {
                        last_client_activity = Instant::now();
                        if sender.send(Message::Pong(payload)).await.is_err() {
                            break;
                        }
                    }
                    Some(Ok(Message::Pong(_))) => {
                        last_client_activity = Instant::now();
                        state.presence().touch(user_id).await;
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(Message::Binary(_))) => {
                        let _ = sender
                            .send(json_message(&ServerMessage::error(
                                "binary frames are not supported",
                            )))
                            .await;
                    }
                    Some(Err(_)) => break,
                }
            }
            event = event_rx.recv() => {
                match event {
                    Ok(message) => {
                        if sender.send(json_message(&message)).await.is_err() {
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
            poll_event = poll_rx.recv() => {
                match poll_event {
                    Ok((poll_id, message)) => {
                        if poll_subscriptions.contains(&poll_id)
                            && sender.send(json_message(&message)).await.is_err()
                        {
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
            _ = heartbeat_tick.tick() => {
                if last_client_activity.elapsed() > idle_timeout {
                    break;
                }
                state.presence().touch(user_id).await;
                if sender.send(Message::Ping(Vec::new().into())).await.is_err() {
                    break;
                }
            }
        }
    }

    state.realtime().hub().unsubscribe(user_id).await;
}

async fn presence_updates(
    state: &AppState,
    client: &ClientMessage,
) -> Result<Vec<ServerMessage>, &'static str> {
    let user_ids = client
        .data
        .get("user_ids")
        .and_then(|value| value.as_array())
        .ok_or("user_ids required")?;
    if user_ids.len() > 20 {
        return Err("too many user ids");
    }
    let mut parsed = Vec::new();
    for value in user_ids {
        let id = value
            .as_str()
            .and_then(|raw| Uuid::parse_str(raw).ok())
            .ok_or("invalid user id")?;
        parsed.push(id);
    }
    Ok(state
        .presence()
        .get_many(&parsed)
        .await
        .into_iter()
        .map(|status| ServerMessage::presence_updated(status.user_id, status.online))
        .collect())
}

fn parse_single_uuid(client: &ClientMessage, field: &str) -> Result<Uuid, &'static str> {
    let raw = client
        .data
        .get(field)
        .and_then(|value| value.as_str())
        .ok_or("poll_id required")?;
    Uuid::parse_str(raw).map_err(|_| "invalid poll_id")
}

fn json_message(message: &ServerMessage) -> Message {
    let body = serde_json::to_string(message).unwrap_or_else(|_| {
        r#"{"type":"error","timestamp":"1970-01-01T00:00:00Z","data":{"message":"encode failed"}}"#
            .into()
    });
    Message::Text(body.into())
}

async fn enforce_connect_rate(state: &AppState, addr: SocketAddr) -> AppResult<()> {
    let key = format!("realtime:connect-rate:{}", addr.ip());
    let mut redis = state.redis().clone();
    let count: u64 = match redis.incr::<_, _, u64>(&key, 1_u64).await {
        Ok(value) => value,
        Err(error) => {
            tracing::warn!(%error, "realtime connect rate limit unavailable");
            return Ok(());
        }
    };
    if count == 1 {
        let _: redis::RedisResult<()> = redis.expire(&key, CONNECT_RATE_WINDOW_SECS as i64).await;
    }
    if count > state.config().ws_connect_rate_limit {
        return Err(AppError::RateLimited);
    }
    Ok(())
}
