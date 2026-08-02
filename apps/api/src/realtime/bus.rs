use redis::aio::ConnectionManager;
use redis::AsyncCommands;
use futures_util::StreamExt;
use serde_json::{json, Value};
use tokio::sync::broadcast;
use tracing::{error, info, warn};
use uuid::Uuid;

use super::hub::RealtimeHub;
use super::protocol::{RealtimeEnvelope, ServerMessage};
use super::{POLL_EVENTS_CHANNEL, USER_EVENTS_CHANNEL};

#[derive(Clone)]
pub struct RealtimeBus {
    redis: ConnectionManager,
    redis_url: String,
    hub: RealtimeHub,
    local_tx: broadcast::Sender<RealtimeEnvelope>,
    /// Fan-out for poll updates: (poll_id, message).
    poll_tx: broadcast::Sender<(Uuid, ServerMessage)>,
}

impl RealtimeBus {
    pub fn new(redis: ConnectionManager, redis_url: String, hub: RealtimeHub) -> Self {
        let (local_tx, _) = broadcast::channel(256);
        let (poll_tx, _) = broadcast::channel(256);
        Self {
            redis,
            redis_url,
            hub,
            local_tx,
            poll_tx,
        }
    }

    pub fn hub(&self) -> &RealtimeHub {
        &self.hub
    }

    pub fn subscribe_local(&self) -> broadcast::Receiver<RealtimeEnvelope> {
        self.local_tx.subscribe()
    }

    /// Subscribe to poll update events for all polls; the caller filters by id.
    pub fn subscribe_poll_events(&self) -> broadcast::Receiver<(Uuid, ServerMessage)> {
        self.poll_tx.subscribe()
    }

    /// Broadcast a poll update to every client subscribed to this poll.
    /// Publishes on Redis (multi-instance) and fans out locally.
    pub async fn publish_poll_update(&self, poll_id: Uuid, data: Value) {
        let message = ServerMessage {
            type_: "poll.updated".into(),
            timestamp: chrono::Utc::now(),
            data,
        };
        let envelope = json!({ "poll_id": poll_id, "message": message });
        let payload = serde_json::to_string(&envelope).unwrap_or_default();
        let mut redis = self.redis.clone();
        if let Err(error) = redis
            .publish::<_, _, ()>(POLL_EVENTS_CHANNEL, payload)
            .await
        {
            // Fall back to local fan-out when Redis publish is unavailable.
            tracing::warn!(%error, %poll_id, "poll update publish failed; delivering locally");
            let _ = self.poll_tx.send((poll_id, message));
        }
    }

    pub fn spawn_poll_subscriber(self) {
        tokio::spawn(async move {
            loop {
                if let Err(error) = self.run_poll_subscriber_once().await {
                    error!(%error, "realtime poll subscriber failed; retrying");
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                }
            }
        });
    }

    async fn run_poll_subscriber_once(&self) -> anyhow::Result<()> {
        let client = redis::Client::open(self.redis_url.as_str())?;
        let mut pubsub = client.get_async_pubsub().await?;
        pubsub.subscribe(POLL_EVENTS_CHANNEL).await?;
        info!(
            channel = POLL_EVENTS_CHANNEL,
            "realtime poll subscriber ready"
        );
        let mut stream = pubsub.on_message();
        while let Some(message) = stream.next().await {
            let payload: String = message.get_payload()?;
            let envelope: Value = match serde_json::from_str(&payload) {
                Ok(value) => value,
                Err(error) => {
                    tracing::warn!(%error, "malformed poll update payload");
                    continue;
                }
            };
            let (Some(poll_id), Some(message)) = (
                envelope.get("poll_id").and_then(Value::as_str).and_then(|raw| Uuid::parse_str(raw).ok()),
                envelope.get("message"),
            ) else {
                continue;
            };
            let message: ServerMessage = match serde_json::from_value(message.clone()) {
                Ok(value) => value,
                Err(error) => {
                    tracing::warn!(%error, "malformed poll update message");
                    continue;
                }
            };
            if self.poll_tx.send((poll_id, message)).is_err() {
                // No local subscribers; fine.
            }
        }
        Ok(())
    }

    pub async fn publish_to_user(
        &self,
        user_id: Uuid,
        type_: &str,
        data: serde_json::Value,
    ) -> anyhow::Result<()> {
        let envelope = RealtimeEnvelope::new(type_, user_id, data);
        let payload = serde_json::to_string(&envelope)?;
        let mut redis = self.redis.clone();
        match redis
            .publish::<_, _, ()>(USER_EVENTS_CHANNEL, payload)
            .await
        {
            Ok(()) => Ok(()),
            Err(error) => {
                // Fall back to local fan-out when Redis publish is unavailable.
                tracing::warn!(%error, %user_id, "realtime publish failed; delivering locally");
                self.hub
                    .send_to_user(user_id, envelope.to_server_message())
                    .await;
                let _ = self.local_tx.send(envelope);
                Err(error.into())
            }
        }
    }

    pub fn spawn_subscriber(self) {
        tokio::spawn(async move {
            loop {
                if let Err(error) = self.run_subscriber_once().await {
                    error!(%error, "realtime redis subscriber failed; retrying");
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                }
            }
        });
    }

    async fn run_subscriber_once(&self) -> anyhow::Result<()> {
        let client = redis::Client::open(self.redis_url.as_str())?;
        let mut pubsub = client.get_async_pubsub().await?;
        pubsub.subscribe(USER_EVENTS_CHANNEL).await?;
        info!(
            channel = USER_EVENTS_CHANNEL,
            "realtime redis subscriber ready"
        );

        let mut stream = pubsub.on_message();
        use futures_util::StreamExt;
        while let Some(message) = stream.next().await {
            let payload: String = match message.get_payload() {
                Ok(value) => value,
                Err(error) => {
                    warn!(%error, "invalid realtime redis payload");
                    continue;
                }
            };
            let envelope: RealtimeEnvelope = match serde_json::from_str(&payload) {
                Ok(value) => value,
                Err(error) => {
                    warn!(%error, "failed to decode realtime envelope");
                    continue;
                }
            };
            // Local sockets already received via publish_to_user; still deliver for remote
            // publishers. Duplicate local delivery is acceptable for notifications (idempotent UI).
            self.hub
                .send_to_user(envelope.user_id, envelope.to_server_message())
                .await;
            let _ = self.local_tx.send(envelope);
        }
        anyhow::bail!("realtime redis subscription ended")
    }
}
