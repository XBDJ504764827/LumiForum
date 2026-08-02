use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RealtimeEnvelope {
    #[serde(rename = "type")]
    pub type_: String,
    pub timestamp: DateTime<Utc>,
    pub user_id: Uuid,
    pub data: Value,
}

impl RealtimeEnvelope {
    pub fn new(type_: impl Into<String>, user_id: Uuid, data: Value) -> Self {
        Self {
            type_: type_.into(),
            timestamp: Utc::now(),
            user_id,
            data,
        }
    }

    pub fn to_server_message(&self) -> ServerMessage {
        ServerMessage {
            type_: self.type_.clone(),
            timestamp: self.timestamp,
            data: self.data.clone(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServerMessage {
    #[serde(rename = "type")]
    pub type_: String,
    pub timestamp: DateTime<Utc>,
    pub data: Value,
}

impl ServerMessage {
    pub fn connected(user_id: Uuid) -> Self {
        Self {
            type_: "connected".into(),
            timestamp: Utc::now(),
            data: serde_json::json!({ "user_id": user_id }),
        }
    }

    pub fn pong() -> Self {
        Self {
            type_: "pong".into(),
            timestamp: Utc::now(),
            data: serde_json::json!({}),
        }
    }

    pub fn error(message: &str) -> Self {
        Self {
            type_: "error".into(),
            timestamp: Utc::now(),
            data: serde_json::json!({ "message": message }),
        }
    }

    pub fn presence_updated(user_id: Uuid, online: bool) -> Self {
        Self {
            type_: "presence.updated".into(),
            timestamp: Utc::now(),
            data: serde_json::json!({
                "user_id": user_id,
                "online": online,
            }),
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct ClientMessage {
    #[serde(rename = "type")]
    pub type_: String,
    #[serde(default)]
    pub data: Value,
}

#[cfg(test)]
mod tests {
    use super::{ClientMessage, ServerMessage};
    use uuid::Uuid;

    #[test]
    fn serializes_server_message_type_field() {
        let message = ServerMessage::connected(Uuid::new_v4());
        let json = serde_json::to_string(&message).unwrap();
        assert!(json.contains(r#""type":"connected""#));
    }

    #[test]
    fn parses_client_ping() {
        let message: ClientMessage = serde_json::from_str(r#"{"type":"ping"}"#).unwrap();
        assert_eq!(message.type_, "ping");
    }
}
