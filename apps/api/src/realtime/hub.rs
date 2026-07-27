use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use tokio::sync::{broadcast, RwLock};
use uuid::Uuid;

use super::protocol::ServerMessage;
use super::DEFAULT_MAX_CONNECTIONS_PER_USER;

#[derive(Clone)]
pub struct RealtimeHub {
    inner: Arc<HubInner>,
    max_connections_per_user: usize,
}

struct HubInner {
    users: RwLock<HashMap<Uuid, UserSockets>>,
    connection_count: AtomicUsize,
}

struct UserSockets {
    sender: broadcast::Sender<ServerMessage>,
    sockets: usize,
}

impl RealtimeHub {
    pub fn new(max_connections_per_user: usize) -> Self {
        Self {
            inner: Arc::new(HubInner {
                users: RwLock::new(HashMap::new()),
                connection_count: AtomicUsize::new(0),
            }),
            max_connections_per_user: max_connections_per_user.max(1),
        }
    }

    pub fn connection_count(&self) -> usize {
        self.inner.connection_count.load(Ordering::Relaxed)
    }

    pub async fn subscribe(
        &self,
        user_id: Uuid,
    ) -> Result<(Uuid, broadcast::Receiver<ServerMessage>), HubError> {
        let mut users = self.inner.users.write().await;
        let entry = users.entry(user_id).or_insert_with(|| {
            let (sender, _) = broadcast::channel(64);
            UserSockets { sender, sockets: 0 }
        });
        if entry.sockets >= self.max_connections_per_user {
            return Err(HubError::TooManyConnections);
        }
        entry.sockets += 1;
        let receiver = entry.sender.subscribe();
        self.inner.connection_count.fetch_add(1, Ordering::Relaxed);
        Ok((user_id, receiver))
    }

    pub async fn unsubscribe(&self, user_id: Uuid) {
        let mut users = self.inner.users.write().await;
        let remove = if let Some(entry) = users.get_mut(&user_id) {
            entry.sockets = entry.sockets.saturating_sub(1);
            entry.sockets == 0
        } else {
            false
        };
        if remove {
            users.remove(&user_id);
        }
        self.inner
            .connection_count
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |count| {
                Some(count.saturating_sub(1))
            })
            .ok();
    }

    pub async fn user_connection_count(&self, user_id: Uuid) -> usize {
        self.inner
            .users
            .read()
            .await
            .get(&user_id)
            .map(|entry| entry.sockets)
            .unwrap_or(0)
    }

    pub async fn send_to_user(&self, user_id: Uuid, message: ServerMessage) {
        let users = self.inner.users.read().await;
        if let Some(entry) = users.get(&user_id) {
            let _ = entry.sender.send(message);
        }
    }

    pub fn max_connections_per_user(&self) -> usize {
        self.max_connections_per_user
    }
}

impl Default for RealtimeHub {
    fn default() -> Self {
        Self::new(DEFAULT_MAX_CONNECTIONS_PER_USER)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum HubError {
    #[error("too many connections for user")]
    TooManyConnections,
}

#[cfg(test)]
mod tests {
    use super::{HubError, RealtimeHub};
    use crate::realtime::protocol::ServerMessage;
    use uuid::Uuid;

    #[tokio::test]
    async fn enforces_per_user_connection_limit() {
        let hub = RealtimeHub::new(2);
        let user = Uuid::new_v4();
        hub.subscribe(user).await.unwrap();
        hub.subscribe(user).await.unwrap();
        let err = hub.subscribe(user).await.unwrap_err();
        assert!(matches!(err, HubError::TooManyConnections));
        hub.unsubscribe(user).await;
        hub.subscribe(user).await.unwrap();
    }

    #[tokio::test]
    async fn delivers_local_messages() {
        let hub = RealtimeHub::new(5);
        let user = Uuid::new_v4();
        let (_id, mut receiver) = hub.subscribe(user).await.unwrap();
        hub.send_to_user(user, ServerMessage::pong()).await;
        let message = receiver.recv().await.unwrap();
        assert_eq!(message.type_, "pong");
    }
}
