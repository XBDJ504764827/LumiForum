use sqlx::PgPool;
use uuid::Uuid;

use super::RepositoryUser;
use crate::services::SteamProfile;

#[derive(Clone)]
pub struct SteamAuthRepository {
    pool: PgPool,
}

impl SteamAuthRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn find_by_steam_id(
        &self,
        steam_id: &str,
    ) -> Result<Option<RepositoryUser>, sqlx::Error> {
        sqlx::query_as::<_, RepositoryUser>(&user_query("users.steam_id = $1"))
            .bind(steam_id)
            .fetch_optional(&self.pool)
            .await
    }

    pub async fn find_by_id(&self, user_id: Uuid) -> Result<Option<RepositoryUser>, sqlx::Error> {
        sqlx::query_as::<_, RepositoryUser>(&user_query("users.id = $1"))
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await
    }

    pub async fn create_steam_user(
        &self,
        email: &str,
        profile: &SteamProfile,
    ) -> Result<RepositoryUser, sqlx::Error> {
        sqlx::query_as::<_, RepositoryUser>(
            r#"
            WITH inserted AS (
                INSERT INTO users (
                    username, email, password_hash, avatar_url, nickname, role_id,
                    steam_id, steam_persona_name, steam_avatar, steam_avatar_medium,
                    steam_avatar_full, steam_profile_url, steam_country_code,
                    last_login_at
                )
                SELECT $1, $2, NULL, COALESCE($4, $3), $5, id, $1, $5, $3, $4, $6, $7, $8, now()
                FROM roles
                WHERE code = 'user'
                RETURNING *
            )
            SELECT
                inserted.id, inserted.username, inserted.email, inserted.password_hash,
                inserted.avatar_url AS avatar, inserted.nickname,
                roles.code AS role_code, roles.name AS role_name,
                inserted.status, inserted.email_verified, inserted.auth_version,
                inserted.followers_count, inserted.following_count,
                inserted.steam_id, inserted.steam_persona_name, inserted.steam_avatar,
                inserted.steam_avatar_medium, inserted.steam_avatar_full,
                inserted.steam_profile_url, inserted.steam_country_code,
                inserted.created_at, inserted.updated_at
            FROM inserted
            JOIN roles ON roles.id = inserted.role_id
            "#,
        )
        .bind(&profile.steam_id)
        .bind(email)
        .bind(&profile.avatar)
        .bind(&profile.avatar_medium)
        .bind(&profile.persona_name)
        .bind(&profile.avatar_full)
        .bind(&profile.profile_url)
        .bind(&profile.country_code)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn sync_profile(
        &self,
        user_id: Uuid,
        profile: &SteamProfile,
    ) -> Result<Option<RepositoryUser>, sqlx::Error> {
        sqlx::query_as::<_, RepositoryUser>(
            r#"
            WITH updated AS (
                UPDATE users
                SET username = CASE WHEN password_hash IS NULL THEN $8 ELSE username END,
                    nickname = CASE WHEN password_hash IS NULL THEN $2 ELSE nickname END,
                    avatar_url = CASE
                        WHEN password_hash IS NULL THEN COALESCE($4, $3)
                        ELSE avatar_url
                    END,
                    steam_persona_name = $2,
                    steam_avatar = $3,
                    steam_avatar_medium = $4,
                    steam_avatar_full = $5,
                    steam_profile_url = $6,
                    steam_country_code = $7,
                    last_login_at = now()
                WHERE id = $1 AND steam_id = $8
                RETURNING *
            )
            SELECT
                updated.id, updated.username, updated.email, updated.password_hash,
                updated.avatar_url AS avatar, updated.nickname,
                roles.code AS role_code, roles.name AS role_name,
                updated.status, updated.email_verified, updated.auth_version,
                updated.followers_count, updated.following_count,
                updated.steam_id, updated.steam_persona_name, updated.steam_avatar,
                updated.steam_avatar_medium, updated.steam_avatar_full,
                updated.steam_profile_url, updated.steam_country_code,
                updated.created_at, updated.updated_at
            FROM updated
            JOIN roles ON roles.id = updated.role_id
            "#,
        )
        .bind(user_id)
        .bind(&profile.persona_name)
        .bind(&profile.avatar)
        .bind(&profile.avatar_medium)
        .bind(&profile.avatar_full)
        .bind(&profile.profile_url)
        .bind(&profile.country_code)
        .bind(&profile.steam_id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn bind(
        &self,
        user_id: Uuid,
        profile: &SteamProfile,
    ) -> Result<Option<RepositoryUser>, sqlx::Error> {
        sqlx::query_as::<_, RepositoryUser>(
            r#"
            WITH updated AS (
                UPDATE users
                SET steam_id = $2,
                    steam_persona_name = $3,
                    steam_avatar = $4,
                    steam_avatar_medium = $5,
                    steam_avatar_full = $6,
                    steam_profile_url = $7,
                    steam_country_code = $8,
                    last_login_at = now()
                WHERE id = $1 AND steam_id IS NULL
                RETURNING *
            )
            SELECT
                updated.id, updated.username, updated.email, updated.password_hash,
                updated.avatar_url AS avatar, updated.nickname,
                roles.code AS role_code, roles.name AS role_name,
                updated.status, updated.email_verified, updated.auth_version,
                updated.followers_count, updated.following_count,
                updated.steam_id, updated.steam_persona_name, updated.steam_avatar,
                updated.steam_avatar_medium, updated.steam_avatar_full,
                updated.steam_profile_url, updated.steam_country_code,
                updated.created_at, updated.updated_at
            FROM updated
            JOIN roles ON roles.id = updated.role_id
            "#,
        )
        .bind(user_id)
        .bind(&profile.steam_id)
        .bind(&profile.persona_name)
        .bind(&profile.avatar)
        .bind(&profile.avatar_medium)
        .bind(&profile.avatar_full)
        .bind(&profile.profile_url)
        .bind(&profile.country_code)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn unbind(&self, user_id: Uuid) -> Result<Option<RepositoryUser>, sqlx::Error> {
        sqlx::query_as::<_, RepositoryUser>(
            r#"
            WITH updated AS (
                UPDATE users
                SET steam_id = NULL,
                    steam_persona_name = NULL,
                    steam_avatar = NULL,
                    steam_avatar_medium = NULL,
                    steam_avatar_full = NULL,
                    steam_profile_url = NULL,
                    steam_country_code = NULL
                WHERE id = $1 AND steam_id IS NOT NULL AND password_hash IS NOT NULL
                RETURNING *
            )
            SELECT
                updated.id, updated.username, updated.email, updated.password_hash,
                updated.avatar_url AS avatar, updated.nickname,
                roles.code AS role_code, roles.name AS role_name,
                updated.status, updated.email_verified, updated.auth_version,
                updated.followers_count, updated.following_count,
                updated.steam_id, updated.steam_persona_name, updated.steam_avatar,
                updated.steam_avatar_medium, updated.steam_avatar_full,
                updated.steam_profile_url, updated.steam_country_code,
                updated.created_at, updated.updated_at
            FROM updated
            JOIN roles ON roles.id = updated.role_id
            "#,
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
    }
}

pub fn is_unique_violation(error: &sqlx::Error) -> bool {
    matches!(error, sqlx::Error::Database(db) if db.is_unique_violation())
}

fn user_query(predicate: &str) -> String {
    format!(
        r#"
        SELECT
            users.id, users.username, users.email, users.password_hash,
            users.avatar_url AS avatar, users.nickname,
            roles.code AS role_code, roles.name AS role_name,
            users.status, users.email_verified, users.auth_version,
            users.followers_count, users.following_count,
            users.steam_id, users.steam_persona_name, users.steam_avatar,
            users.steam_avatar_medium, users.steam_avatar_full,
            users.steam_profile_url, users.steam_country_code,
            users.created_at, users.updated_at
        FROM users
        JOIN roles ON roles.id = users.role_id
        WHERE {predicate}
        "#
    )
}
