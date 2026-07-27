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
    pub password_hash: String,
    pub avatar: Option<String>,
    pub nickname: Option<String>,
    pub role_code: String,
    pub role_name: String,
    pub status: String,
    pub email_verified: bool,
    pub auth_version: i32,
    pub followers_count: i64,
    pub following_count: i64,
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
        avatar_changed: bool,
        avatar: Option<&str>,
        nickname_changed: bool,
        nickname: Option<&str>,
    ) -> Result<Option<RepositoryUser>, sqlx::Error> {
        sqlx::query_as::<_, RepositoryUser>(
            r#"
            WITH updated AS (
                UPDATE users
                SET avatar = CASE WHEN $2 THEN $3 ELSE avatar END,
                    nickname = CASE WHEN $4 THEN $5 ELSE nickname END
                WHERE id = $1
                RETURNING *
            )
            SELECT
                updated.id,
                updated.username,
                updated.email,
                updated.password_hash,
                updated.avatar,
                updated.nickname,
                roles.code AS role_code,
                roles.name AS role_name,
                updated.status,
                updated.email_verified,
                updated.auth_version,
                updated.followers_count,
                updated.following_count,
                updated.created_at,
                updated.updated_at
            FROM updated
            JOIN roles ON roles.id = updated.role_id
            "#,
        )
        .bind(user_id)
        .bind(avatar_changed)
        .bind(avatar)
        .bind(nickname_changed)
        .bind(nickname)
        .fetch_optional(&self.pool)
        .await
    }
}

pub fn repository_user_to_response(user: RepositoryUser) -> Result<UserResponse, &'static str> {
    let status = UserStatus::from_str(&user.status)?;
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
        users.avatar,
        users.nickname,
        roles.code AS role_code,
        roles.name AS role_name,
        users.status,
        users.email_verified,
        users.auth_version,
        users.followers_count,
        users.following_count,
        users.created_at,
        users.updated_at
    FROM users
    JOIN roles ON roles.id = users.role_id
    WHERE users.id = $1
"#;
