use chrono::{DateTime, Utc};
use ipnetwork::IpNetwork;
use uuid::Uuid;

/// Persistence-only refresh-token row. `token_hash` is a SHA-256 digest.
#[derive(sqlx::FromRow)]
pub struct RefreshTokenRecord {
    pub id: Uuid,
    pub user_id: Uuid,
    pub family_id: Uuid,
    pub token_hash: Vec<u8>,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub revocation_reason: Option<String>,
    pub replaced_by_id: Option<Uuid>,
    pub created_by_ip: Option<IpNetwork>,
    pub user_agent: Option<String>,
}
