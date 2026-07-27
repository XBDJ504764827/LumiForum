use std::collections::HashSet;

use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

#[derive(Clone, Debug, sqlx::FromRow)]
pub struct RoleRecord {
    pub id: Uuid,
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub priority: i16,
    pub is_system: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, sqlx::FromRow)]
pub struct PermissionRecord {
    pub id: Uuid,
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, sqlx::FromRow)]
pub struct RolePermissionRecord {
    pub role_id: Uuid,
    pub permission_id: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize)]
pub struct RoleSummary {
    pub code: String,
    pub name: String,
}

pub const ROLE_GUEST: &str = "guest";
pub const ROLE_USER: &str = "user";
pub const ROLE_MODERATOR: &str = "moderator";
pub const ROLE_ADMINISTRATOR: &str = "administrator";
pub const ROLE_SUPER_ADMINISTRATOR: &str = "super_administrator";

pub const PERMISSION_PROFILE_READ_SELF: &str = "user.profile.read:self";
pub const PERMISSION_PROFILE_UPDATE_SELF: &str = "user.profile.update:self";
pub const PERMISSION_CATEGORY_MANAGE: &str = "category.manage";
pub const PERMISSION_TOPIC_CREATE: &str = "topic.create";
pub const PERMISSION_TOPIC_UPDATE_SELF: &str = "topic.update:self";
pub const PERMISSION_TOPIC_UPDATE_ANY: &str = "topic.update:any";
pub const PERMISSION_TOPIC_DELETE_SELF: &str = "topic.delete:self";
pub const PERMISSION_TOPIC_DELETE_ANY: &str = "topic.delete:any";
pub const PERMISSION_TOPIC_PIN: &str = "topic.pin";
pub const PERMISSION_TOPIC_FEATURE: &str = "topic.feature";
pub const PERMISSION_COMMENT_CREATE: &str = "comment.create";
pub const PERMISSION_COMMENT_REPLY: &str = "comment.reply";
pub const PERMISSION_COMMENT_UPDATE_SELF: &str = "comment.update:self";
pub const PERMISSION_COMMENT_UPDATE_ANY: &str = "comment.update:any";
pub const PERMISSION_COMMENT_DELETE_SELF: &str = "comment.delete:self";
pub const PERMISSION_COMMENT_DELETE_ANY: &str = "comment.delete:any";
pub const PERMISSION_COMMENT_RESTORE: &str = "comment.restore";

#[derive(Clone)]
pub struct AuthenticatedPrincipal {
    pub user_id: Uuid,
    pub role: String,
    pub auth_version: i32,
    pub token_id: Uuid,
    permissions: HashSet<String>,
}

impl AuthenticatedPrincipal {
    pub fn new(
        user_id: Uuid,
        role: String,
        auth_version: i32,
        token_id: Uuid,
        permissions: impl IntoIterator<Item = String>,
    ) -> Self {
        Self {
            user_id,
            role,
            auth_version,
            token_id,
            permissions: permissions.into_iter().collect(),
        }
    }

    pub fn has_permission(&self, permission: &str) -> bool {
        self.permissions.contains(permission)
    }
}

#[derive(Clone)]
pub enum Principal {
    Guest,
    Authenticated(AuthenticatedPrincipal),
}

impl Principal {
    pub fn role(&self) -> &str {
        match self {
            Self::Guest => ROLE_GUEST,
            Self::Authenticated(principal) => &principal.role,
        }
    }

    pub fn has_permission(&self, permission: &str) -> bool {
        match self {
            Self::Guest => false,
            Self::Authenticated(principal) => principal.has_permission(permission),
        }
    }
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use super::{
        AuthenticatedPrincipal, Principal, PERMISSION_PROFILE_READ_SELF, ROLE_GUEST, ROLE_USER,
    };

    #[test]
    fn evaluates_guest_and_authenticated_permissions() {
        let guest = Principal::Guest;
        assert_eq!(guest.role(), ROLE_GUEST);
        assert!(!guest.has_permission(PERMISSION_PROFILE_READ_SELF));

        let user = Principal::Authenticated(AuthenticatedPrincipal::new(
            Uuid::new_v4(),
            ROLE_USER.into(),
            0,
            Uuid::new_v4(),
            [PERMISSION_PROFILE_READ_SELF.into()],
        ));
        assert_eq!(user.role(), ROLE_USER);
        assert!(user.has_permission(PERMISSION_PROFILE_READ_SELF));
    }
}
