use std::str::FromStr;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::RoleSummary;

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UserStatus {
    #[default]
    Active,
    Pending,
    Suspended,
    Disabled,
}

impl UserStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Pending => "pending",
            Self::Suspended => "suspended",
            Self::Disabled => "disabled",
        }
    }
}

impl FromStr for UserStatus {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "active" => Ok(Self::Active),
            "pending" => Ok(Self::Pending),
            "suspended" => Ok(Self::Suspended),
            "disabled" => Ok(Self::Disabled),
            _ => Err("unknown user status"),
        }
    }
}

/// Persistence-only user row. This type must never be serialized or logged.
#[derive(sqlx::FromRow)]
pub struct UserRecord {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub avatar: Option<String>,
    pub nickname: Option<String>,
    pub role_id: Uuid,
    pub status: String,
    pub email_verified: bool,
    pub email_verified_at: Option<DateTime<Utc>>,
    pub auth_version: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize)]
pub struct UserResponse {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub avatar: Option<String>,
    pub nickname: Option<String>,
    pub role: RoleSummary,
    pub status: UserStatus,
    pub email_verified: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub enum PatchField<T> {
    Missing,
    Set(Option<T>),
}

impl<T> Default for PatchField<T> {
    fn default() -> Self {
        Self::Missing
    }
}

impl<'de, T> Deserialize<'de> for PatchField<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Option::<T>::deserialize(deserializer).map(Self::Set)
    }
}

#[derive(Default, Deserialize)]
pub struct ProfileUpdateRequest {
    #[serde(default)]
    pub avatar: PatchField<String>,
    #[serde(default)]
    pub nickname: PatchField<String>,
}
