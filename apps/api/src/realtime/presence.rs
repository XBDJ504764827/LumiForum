use chrono::{DateTime, Utc};
use redis::aio::ConnectionManager;
use redis::AsyncCommands;
use serde::Serialize;
use uuid::Uuid;

#[derive(Clone)]
pub struct PresenceService {
    redis: ConnectionManager,
    ttl_secs: u64,
}

#[derive(Clone, Debug, Serialize)]
pub struct PresenceStatus {
    pub user_id: Uuid,
    pub online: bool,
    pub last_seen_at: Option<DateTime<Utc>>,
}

impl PresenceService {
    pub fn new(redis: ConnectionManager, ttl_secs: u64) -> Self {
        Self {
            redis,
            ttl_secs: ttl_secs.max(15),
        }
    }

    fn key(user_id: Uuid) -> String {
        format!("presence:online:{user_id}")
    }

    pub async fn touch(&self, user_id: Uuid) {
        let mut redis = self.redis.clone();
        let now = Utc::now().timestamp();
        if let Err(error) = redis
            .set_ex::<_, _, ()>(Self::key(user_id), now, self.ttl_secs)
            .await
        {
            tracing::warn!(%error, %user_id, "failed to refresh presence");
        }
    }

    pub async fn get(&self, user_id: Uuid) -> PresenceStatus {
        let mut redis = self.redis.clone();
        match redis.get::<_, Option<i64>>(Self::key(user_id)).await {
            Ok(Some(ts)) => PresenceStatus {
                user_id,
                online: true,
                last_seen_at: DateTime::from_timestamp(ts, 0),
            },
            Ok(None) => PresenceStatus {
                user_id,
                online: false,
                last_seen_at: None,
            },
            Err(error) => {
                tracing::warn!(%error, %user_id, "presence lookup failed");
                PresenceStatus {
                    user_id,
                    online: false,
                    last_seen_at: None,
                }
            }
        }
    }

    pub async fn get_many(&self, user_ids: &[Uuid]) -> Vec<PresenceStatus> {
        let mut out = Vec::with_capacity(user_ids.len());
        for user_id in user_ids {
            out.push(self.get(*user_id).await);
        }
        out
    }

    /// Number of distinct online users (SCAN over presence keys).
    pub async fn count_online(&self) -> usize {
        let mut redis = self.redis.clone();
        let iter = match redis.scan_match::<_, String>("presence:online:*").await {
            Ok(iter) => iter,
            Err(error) => {
                tracing::warn!(%error, "presence count unavailable");
                return 0;
            }
        };
        let mut count = 0_usize;
        let mut iter = iter;
        while iter.next_item().await.is_some() {
            count += 1;
        }
        count
    }

    pub fn ttl_secs(&self) -> u64 {
        self.ttl_secs
    }
}
