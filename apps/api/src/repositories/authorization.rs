use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Clone, Deserialize, Serialize, sqlx::FromRow)]
pub struct AuthorizationSnapshot {
    pub user_id: Uuid,
    pub status: String,
    pub auth_version: i32,
    pub role: String,
    pub permissions: Vec<String>,
}

#[derive(Clone)]
pub struct AuthorizationRepository {
    pool: PgPool,
}

impl AuthorizationRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn find_snapshot(
        &self,
        user_id: Uuid,
    ) -> Result<Option<AuthorizationSnapshot>, sqlx::Error> {
        sqlx::query_as::<_, AuthorizationSnapshot>(
            r#"
            SELECT
                users.id AS user_id,
                users.status,
                users.auth_version,
                roles.code AS role,
                COALESCE(
                    array_agg(permissions.code) FILTER (WHERE permissions.code IS NOT NULL),
                    ARRAY[]::varchar[]
                ) AS permissions
            FROM users
            JOIN roles ON roles.id = users.role_id
            LEFT JOIN role_permissions ON role_permissions.role_id = roles.id
            LEFT JOIN permissions ON permissions.id = role_permissions.permission_id
            WHERE users.id = $1
            GROUP BY users.id, users.status, users.auth_version, roles.code
            "#,
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
    }
}
