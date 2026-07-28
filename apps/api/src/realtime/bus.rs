use redis::aio::ConnectionManager;
use redis::AsyncCommands;
use tokio::sync::broadcast;
use tracing::{error, info, warn};
use uuid::Uuid;

use super::hub::RealtimeHub;
use super::protocol::RealtimeEnvelope;
use super::USER_EVENTS_CHANNEL;

#[derive(Clone)]
pub struct RealtimeBus {
    redis: ConnectionManager,
    redis_url: String,
    hub: RealtimeHub,
    local_tx: broadcast::Sender<RealtimeEnvelope>,
}

impl RealtimeBus {
    pub fn new(redis: ConnectionManager, redis_url: String, hub: RealtimeHub) -> Self {
        let (local_tx, _) = broadcast::channel(256);
        Self {
            redis,
            redis_url,
            hub,
            local_tx,
        }
    }

    pub fn hub(&self) -> &RealtimeHub {
        &self.hub
    }

    pub fn subscribe_local(&self) -> broadcast::Receiver<RealtimeEnvelope> {
        self.local_tx.subscribe()
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
