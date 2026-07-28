use lumiforum_api::realtime::{
    ClientMessage, RealtimeEnvelope, RealtimeHub, ServerMessage, USER_EVENTS_CHANNEL,
};
use uuid::Uuid;

#[test]
fn server_and_client_message_contracts() {
    let user = Uuid::new_v4();
    let connected = ServerMessage::connected(user);
    let json = serde_json::to_value(&connected).unwrap();
    assert_eq!(json["type"], "connected");
    assert_eq!(json["data"]["user_id"], user.to_string());

    let ping: ClientMessage = serde_json::from_str(r#"{"type":"ping","data":{}}"#).unwrap();
    assert_eq!(ping.type_, "ping");
}

#[test]
fn envelope_roundtrip_for_redis_payload() {
    let user = Uuid::new_v4();
    let envelope = RealtimeEnvelope::new(
        "notification.created",
        user,
        serde_json::json!({ "title": "hello" }),
    );
    let encoded = serde_json::to_string(&envelope).unwrap();
    let decoded: RealtimeEnvelope = serde_json::from_str(&encoded).unwrap();
    assert_eq!(decoded.user_id, user);
    assert_eq!(decoded.type_, "notification.created");
    assert_eq!(decoded.data["title"], "hello");
    assert_eq!(USER_EVENTS_CHANNEL, "realtime:user-events");
}

#[tokio::test]
async fn hub_limits_and_delivery() {
    let hub = RealtimeHub::new(1);
    let user = Uuid::new_v4();
    let (_id, mut rx) = hub.subscribe(user).await.unwrap();
    assert!(hub.subscribe(user).await.is_err());
    hub.send_to_user(user, ServerMessage::pong()).await;
    let message = rx.recv().await.unwrap();
    assert_eq!(message.type_, "pong");
    hub.unsubscribe(user).await;
    assert_eq!(hub.user_connection_count(user).await, 0);
}
