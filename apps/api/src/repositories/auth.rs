use chrono::{DateTime, Utc};
use ipnetwork::IpNetwork;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use super::RepositoryUser;

#[derive(Clone)]
pub struct AuthRepository {
    pool: PgPool,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum RefreshRotation {
    Rotated,
    NotFound,
    Expired,
    Replayed,
    AccountUnavailable,
}

#[derive(sqlx::FromRow)]
struct LockedRefreshToken {
    id: Uuid,
    user_id: Uuid,
    family_id: Uuid,
    expires_at: DateTime<Utc>,
    revoked_at: Option<DateTime<Utc>>,
}

impl AuthRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create_user(
        &self,
        username: &str,
        email: &str,
        password_hash: &str,
        nickname: Option<&str>,
    ) -> Result<RepositoryUser, sqlx::Error> {
        sqlx::query_as::<_, RepositoryUser>(
            r#"
            WITH inserted AS (
                INSERT INTO users (username, email, password_hash, nickname, role_id)
                SELECT $1, $2, $3, $4, id
                FROM roles
                WHERE code = 'user'
                RETURNING *
            )
            SELECT
                inserted.id,
                inserted.username,
                inserted.email,
                inserted.password_hash,
                inserted.avatar_url AS avatar,
                inserted.nickname,
                roles.code AS role_code,
                roles.name AS role_name,
                inserted.status,
                inserted.email_verified,
                inserted.auth_version,
                inserted.followers_count,
                inserted.following_count,
                inserted.created_at,
                inserted.updated_at
            FROM inserted
            JOIN roles ON roles.id = inserted.role_id
            "#,
        )
        .bind(username)
        .bind(email)
        .bind(password_hash)
        .bind(nickname)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn find_user_by_identifier(
        &self,
        identifier: &str,
    ) -> Result<Option<RepositoryUser>, sqlx::Error> {
        sqlx::query_as::<_, RepositoryUser>(
            r#"
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
                users.created_at,
                users.updated_at
            FROM users
            JOIN roles ON roles.id = users.role_id
            WHERE lower(users.username) = lower($1)
               OR users.email = lower($1)
            LIMIT 1
            "#,
        )
        .bind(identifier)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn create_refresh_token(
        &self,
        user_id: Uuid,
        family_id: Uuid,
        token_hash: &[u8],
        expires_at: DateTime<Utc>,
        created_by_ip: Option<IpNetwork>,
        user_agent: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO refresh_tokens (
                user_id,
                family_id,
                token_hash,
                expires_at,
                created_by_ip,
                user_agent
            )
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
        )
        .bind(user_id)
        .bind(family_id)
        .bind(token_hash)
        .bind(expires_at)
        .bind(created_by_ip)
        .bind(user_agent)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn revoke_token(&self, token_hash: &[u8], reason: &str) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE refresh_tokens
            SET revoked_at = COALESCE(revoked_at, now()),
                revocation_reason = COALESCE(revocation_reason, $2),
                last_used_at = now()
            WHERE token_hash = $1
            "#,
        )
        .bind(token_hash)
        .bind(reason)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn rotate_refresh_token(
        &self,
        current_hash: &[u8],
        successor_hash: &[u8],
        created_by_ip: Option<IpNetwork>,
        user_agent: Option<&str>,
    ) -> Result<(RefreshRotation, Option<RepositoryUser>), sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        let Some(token) = lock_refresh_token(&mut tx, current_hash).await? else {
            tx.commit().await?;
            return Ok((RefreshRotation::NotFound, None));
        };

        if token.revoked_at.is_some() {
            revoke_family(&mut tx, token.family_id, "replay_detected").await?;
            tx.commit().await?;
            return Ok((RefreshRotation::Replayed, None));
        }

        if token.expires_at <= Utc::now() {
            revoke_token_by_id(&mut tx, token.id, "expired").await?;
            tx.commit().await?;
            return Ok((RefreshRotation::Expired, None));
        }

        let user = find_user_by_id_for_update(&mut tx, token.user_id).await?;
        if user.status != "active" {
            revoke_user_tokens(&mut tx, token.user_id, "account_unavailable").await?;
            tx.commit().await?;
            return Ok((RefreshRotation::AccountUnavailable, None));
        }

        let successor_id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO refresh_tokens (
                id,
                user_id,
                family_id,
                token_hash,
                expires_at,
                created_by_ip,
                user_agent
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(successor_id)
        .bind(token.user_id)
        .bind(token.family_id)
        .bind(successor_hash)
        .bind(token.expires_at)
        .bind(created_by_ip)
        .bind(user_agent)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r#"
            UPDATE refresh_tokens
            SET last_used_at = now(),
                revoked_at = now(),
                revocation_reason = 'rotated',
                replaced_by_id = $2
            WHERE id = $1
            "#,
        )
        .bind(token.id)
        .bind(successor_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok((RefreshRotation::Rotated, Some(user)))
    }
}

async fn lock_refresh_token(
    tx: &mut Transaction<'_, Postgres>,
    token_hash: &[u8],
) -> Result<Option<LockedRefreshToken>, sqlx::Error> {
    sqlx::query_as::<_, LockedRefreshToken>(
        r#"
        SELECT id, user_id, family_id, expires_at, revoked_at
        FROM refresh_tokens
        WHERE token_hash = $1
        FOR UPDATE
        "#,
    )
    .bind(token_hash)
    .fetch_optional(&mut **tx)
    .await
}

async fn find_user_by_id_for_update(
    tx: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
) -> Result<RepositoryUser, sqlx::Error> {
    sqlx::query_as::<_, RepositoryUser>(
        r#"
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
            users.created_at,
            users.updated_at
        FROM users
        JOIN roles ON roles.id = users.role_id
        WHERE users.id = $1
        FOR UPDATE OF users
        "#,
    )
    .bind(user_id)
    .fetch_one(&mut **tx)
    .await
}

async fn revoke_family(
    tx: &mut Transaction<'_, Postgres>,
    family_id: Uuid,
    reason: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE refresh_tokens
        SET revoked_at = COALESCE(revoked_at, now()),
            revocation_reason = CASE
                WHEN revoked_at IS NULL THEN $2
                ELSE revocation_reason
            END
        WHERE family_id = $1
        "#,
    )
    .bind(family_id)
    .bind(reason)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn revoke_user_tokens(
    tx: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
    reason: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE refresh_tokens
        SET revoked_at = now(), revocation_reason = $2
        WHERE user_id = $1 AND revoked_at IS NULL
        "#,
    )
    .bind(user_id)
    .bind(reason)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn revoke_token_by_id(
    tx: &mut Transaction<'_, Postgres>,
    token_id: Uuid,
    reason: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE refresh_tokens
        SET revoked_at = now(), revocation_reason = $2, last_used_at = now()
        WHERE id = $1 AND revoked_at IS NULL
        "#,
    )
    .bind(token_id)
    .bind(reason)
    .execute(&mut **tx)
    .await?;
    Ok(())
}
