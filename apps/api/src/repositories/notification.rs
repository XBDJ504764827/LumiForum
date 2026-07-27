use chrono::{DateTime, Utc};
use serde_json::Value as JsonValue;
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{
    NotificationActor, NotificationResponse, NotificationTargetType, NotificationType, RoleSummary,
};

#[derive(Clone)]
pub struct NotificationRepository {
    pool: PgPool,
}

#[derive(sqlx::FromRow)]
struct RepositoryNotification {
    id: Uuid,
    #[sqlx(rename = "type")]
    notification_type: String,
    title: String,
    content: String,
    target_type: Option<String>,
    target_id: Option<Uuid>,
    metadata: JsonValue,
    is_read: bool,
    created_at: DateTime<Utc>,
    actor_id: Option<Uuid>,
    actor_username: Option<String>,
    actor_nickname: Option<String>,
    actor_avatar: Option<String>,
    actor_role_code: Option<String>,
    actor_role_name: Option<String>,
}

pub struct NewNotification<'a> {
    pub user_id: Uuid,
    pub actor_id: Option<Uuid>,
    pub notification_type: NotificationType,
    pub title: &'a str,
    pub content: &'a str,
    pub target_type: Option<NotificationTargetType>,
    pub target_id: Option<Uuid>,
    pub metadata: JsonValue,
}

pub struct NotificationListFilter<'a> {
    pub user_id: Uuid,
    pub is_read: Option<bool>,
    pub notification_type: Option<&'a str>,
    pub limit: i64,
    pub offset: i64,
}

impl NotificationRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, input: NewNotification<'_>) -> Result<Uuid, sqlx::Error> {
        sqlx::query_scalar::<_, Uuid>(
            r#"
            INSERT INTO notifications (
                user_id,
                actor_id,
                type,
                title,
                content,
                target_type,
                target_id,
                metadata
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING id
            "#,
        )
        .bind(input.user_id)
        .bind(input.actor_id)
        .bind(input.notification_type.as_str())
        .bind(input.title)
        .bind(input.content)
        .bind(input.target_type.map(NotificationTargetType::as_str))
        .bind(input.target_id)
        .bind(input.metadata)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn list(
        &self,
        filter: NotificationListFilter<'_>,
    ) -> Result<(Vec<NotificationResponse>, i64), sqlx::Error> {
        let total = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT count(*)
            FROM notifications n
            WHERE n.user_id = $1
              AND ($2::boolean IS NULL OR n.is_read = $2)
              AND ($3::text IS NULL OR n.type = $3)
            "#,
        )
        .bind(filter.user_id)
        .bind(filter.is_read)
        .bind(filter.notification_type)
        .fetch_one(&self.pool)
        .await?;

        let rows = sqlx::query_as::<_, RepositoryNotification>(
            r#"
            SELECT
                n.id,
                n.type,
                n.title,
                n.content,
                n.target_type,
                n.target_id,
                n.metadata,
                n.is_read,
                n.created_at,
                a.id AS actor_id,
                a.username AS actor_username,
                a.nickname AS actor_nickname,
                a.avatar_url AS actor_avatar,
                r.code AS actor_role_code,
                r.name AS actor_role_name
            FROM notifications n
            LEFT JOIN users a ON a.id = n.actor_id
            LEFT JOIN roles r ON r.id = a.role_id
            WHERE n.user_id = $1
              AND ($2::boolean IS NULL OR n.is_read = $2)
              AND ($3::text IS NULL OR n.type = $3)
            ORDER BY n.created_at DESC, n.id DESC
            LIMIT $4 OFFSET $5
            "#,
        )
        .bind(filter.user_id)
        .bind(filter.is_read)
        .bind(filter.notification_type)
        .bind(filter.limit)
        .bind(filter.offset)
        .fetch_all(&self.pool)
        .await?;

        let items = rows
            .into_iter()
            .filter_map(|row| to_response(row).ok())
            .collect();
        Ok((items, total))
    }

    pub async fn count_unread(&self, user_id: Uuid) -> Result<i64, sqlx::Error> {
        sqlx::query_scalar::<_, i64>(
            r#"
            SELECT count(*)
            FROM notifications
            WHERE user_id = $1 AND is_read = false
            "#,
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn mark_read(
        &self,
        user_id: Uuid,
        notification_id: Uuid,
    ) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            r#"
            UPDATE notifications
            SET is_read = true
            WHERE id = $1 AND user_id = $2 AND is_read = false
            "#,
        )
        .bind(notification_id)
        .bind(user_id)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() == 1)
    }

    pub async fn mark_all_read(&self, user_id: Uuid) -> Result<u64, sqlx::Error> {
        let result = sqlx::query(
            r#"
            UPDATE notifications
            SET is_read = true
            WHERE user_id = $1 AND is_read = false
            "#,
        )
        .bind(user_id)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected())
    }

    pub async fn belongs_to_user(
        &self,
        user_id: Uuid,
        notification_id: Uuid,
    ) -> Result<bool, sqlx::Error> {
        sqlx::query_scalar::<_, bool>(
            r#"
            SELECT EXISTS (
                SELECT 1 FROM notifications
                WHERE id = $1 AND user_id = $2
            )
            "#,
        )
        .bind(notification_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn topic_notify_context(
        &self,
        topic_id: Uuid,
    ) -> Result<Option<(Uuid, String, String)>, sqlx::Error> {
        sqlx::query_as::<_, (Uuid, String, String)>(
            r#"
            SELECT t.author_id, t.slug, t.title
            FROM topics t
            JOIN categories c ON c.id = t.category_id
            WHERE t.id = $1
              AND t.status = 'published'
              AND c.is_visible = true
            "#,
        )
        .bind(topic_id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn comment_notify_context(
        &self,
        comment_id: Uuid,
    ) -> Result<Option<(Uuid, Uuid, String)>, sqlx::Error> {
        sqlx::query_as::<_, (Uuid, Uuid, String)>(
            r#"
            SELECT c.author_id, c.topic_id, t.slug
            FROM comments c
            JOIN topics t ON t.id = c.topic_id
            JOIN categories cat ON cat.id = t.category_id
            WHERE c.id = $1
              AND c.status = 'published'
              AND t.status = 'published'
              AND cat.is_visible = true
            "#,
        )
        .bind(comment_id)
        .fetch_optional(&self.pool)
        .await
    }
}

fn to_response(row: RepositoryNotification) -> Result<NotificationResponse, &'static str> {
    let notification_type = row.notification_type.parse::<NotificationType>()?;
    let target_type = row
        .target_type
        .as_deref()
        .map(str::parse::<NotificationTargetType>)
        .transpose()?;
    let actor = match (
        row.actor_id,
        row.actor_username,
        row.actor_role_code,
        row.actor_role_name,
    ) {
        (Some(id), Some(username), Some(role_code), Some(role_name)) => Some(NotificationActor {
            id,
            username,
            nickname: row.actor_nickname,
            avatar: row.actor_avatar,
            role: RoleSummary {
                code: role_code,
                name: role_name,
            },
        }),
        _ => None,
    };

    Ok(NotificationResponse {
        id: row.id,
        notification_type,
        title: row.title,
        content: row.content,
        target_type,
        target_id: row.target_id,
        metadata: row.metadata,
        is_read: row.is_read,
        actor,
        created_at: row.created_at,
        stream_hint: "notifications",
    })
}
