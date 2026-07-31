use std::str::FromStr;

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{RoleSummary, UserResponse, UserStatus};

#[derive(sqlx::FromRow)]
pub struct RepositoryUser {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub password_hash: Option<String>,
    pub avatar: Option<String>,
    pub nickname: Option<String>,
    pub role_code: String,
    pub role_name: String,
    pub status: String,
    pub email_verified: bool,
    pub auth_version: i32,
    pub followers_count: i64,
    pub following_count: i64,
    pub steam_id: Option<String>,
    pub steam_persona_name: Option<String>,
    pub steam_avatar: Option<String>,
    pub steam_avatar_medium: Option<String>,
    pub steam_avatar_full: Option<String>,
    pub steam_profile_url: Option<String>,
    pub steam_country_code: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone)]
pub struct UserRepository {
    pool: PgPool,
}

impl UserRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn find_by_id(&self, user_id: Uuid) -> Result<Option<RepositoryUser>, sqlx::Error> {
        sqlx::query_as::<_, RepositoryUser>(USER_WITH_ROLE_QUERY)
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await
    }

    pub async fn update_profile(
        &self,
        user_id: Uuid,
        nickname_changed: bool,
        nickname: Option<&str>,
    ) -> Result<Option<RepositoryUser>, sqlx::Error> {
        sqlx::query_as::<_, RepositoryUser>(
            r#"
            WITH updated AS (
                UPDATE users
                SET nickname = CASE WHEN $2 THEN $3 ELSE nickname END
                WHERE id = $1
                RETURNING *
            )
            SELECT
                updated.id,
                updated.username,
                updated.email,
                updated.password_hash,
                updated.avatar_url AS avatar,
                updated.nickname,
                roles.code AS role_code,
                roles.name AS role_name,
                updated.status,
                updated.email_verified,
                updated.auth_version,
                updated.followers_count,
                updated.following_count,
                updated.steam_id,
                updated.steam_persona_name,
                updated.steam_avatar,
                updated.steam_avatar_medium,
                updated.steam_avatar_full,
                updated.steam_profile_url,
                updated.steam_country_code,
                updated.created_at,
                updated.updated_at
            FROM updated
            JOIN roles ON roles.id = updated.role_id
            "#,
        )
        .bind(user_id)
        .bind(nickname_changed)
        .bind(nickname)
        .fetch_optional(&self.pool)
        .await
    }
}

pub fn repository_user_to_response(user: RepositoryUser) -> Result<UserResponse, &'static str> {
    let status = UserStatus::from_str(&user.status)?;
    let has_password = user.password_hash.is_some();
    Ok(UserResponse {
        id: user.id,
        username: user.username,
        email: user.email,
        avatar: user.avatar,
        nickname: user.nickname,
        role: RoleSummary {
            code: user.role_code,
            name: user.role_name,
        },
        status,
        email_verified: user.email_verified,
        followers_count: user.followers_count,
        following_count: user.following_count,
        steam_id: user.steam_id,
        steam_persona_name: user.steam_persona_name,
        steam_avatar: user.steam_avatar,
        steam_avatar_medium: user.steam_avatar_medium,
        steam_avatar_full: user.steam_avatar_full,
        steam_profile_url: user.steam_profile_url,
        steam_country_code: user.steam_country_code,
        has_password,
        created_at: user.created_at,
        updated_at: user.updated_at,
    })
}

const USER_WITH_ROLE_QUERY: &str = r#"
    SELECT
        users.id,
        users.username,
        users.email,
        users.password_hash,
        users.avatar_url AS avatar,
        users.nickname,
        roles.code AS role_code,
        roles.name AS role_name,
        users.status,
        users.email_verified,
        users.auth_version,
        users.followers_count,
        users.following_count,
        users.steam_id,
        users.steam_persona_name,
        users.steam_avatar,
        users.steam_avatar_medium,
        users.steam_avatar_full,
        users.steam_profile_url,
        users.steam_country_code,
        users.created_at,
        users.updated_at
    FROM users
    JOIN roles ON roles.id = users.role_id
    WHERE users.id = $1
"#;
