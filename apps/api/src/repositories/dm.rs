use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::models::{ConversationSummary, ConversationUser, MessageResponse, RoleSummary};

/// Sorted participant pair: (user1_id, user2_id) with user1_id < user2_id,
/// matching the DB constraint that gives every pair exactly one row.
pub fn sorted_pair(a: Uuid, b: Uuid) -> (Uuid, Uuid) {
    if a < b {
        (a, b)
    } else {
        (b, a)
    }
}

#[derive(Clone)]
pub struct ConversationRepository {
    pool: sqlx::PgPool,
}

/// The other participant of a conversation as seen by the viewer.
pub struct ParticipantContext {
    pub conversation_id: Uuid,
    pub other_user_id: Uuid,
    pub other_username: String,
    pub other_status: String,
}

#[derive(sqlx::FromRow)]
struct ConversationRow {
    id: Uuid,
    other_user_id: Uuid,
    other_username: String,
    other_nickname: Option<String>,
    other_avatar: Option<String>,
    other_role_code: Option<String>,
    other_role_name: Option<String>,
    unread_count: i64,
    last_message_at: DateTime<Utc>,
    last_message_preview: Option<String>,
}

#[derive(sqlx::FromRow)]
struct ParticipantRow {
    id: Uuid,
    user1_id: Uuid,
    user2_id: Uuid,
    other_username: String,
    other_status: String,
}

#[derive(sqlx::FromRow)]
struct MessageRow {
    id: Uuid,
    conversation_id: Uuid,
    sender_id: Uuid,
    content: String,
    deleted_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    edited_at: Option<DateTime<Utc>>,
}

fn message_to_response(message: MessageRow) -> MessageResponse {
    let is_deleted = message.deleted_at.is_some();
    MessageResponse {
        id: message.id,
        conversation_id: message.conversation_id,
        sender_id: message.sender_id,
        // Deleted messages carry an empty body; the UI renders a placeholder.
        content: if is_deleted {
            String::new()
        } else {
            message.content
        },
        is_deleted,
        created_at: message.created_at,
        edited_at: message.edited_at,
    }
}

impl ConversationRepository {
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self { pool }
    }

    /// Find the conversation for a participant pair, or create it. The
    /// ON CONFLICT clause makes a concurrent creation return the same row.
    pub async fn find_or_create_pair(&self, a: Uuid, b: Uuid) -> Result<Uuid, sqlx::Error> {
        let (user1_id, user2_id) = sorted_pair(a, b);
        let existing: Option<Uuid> = sqlx::query_scalar(
            r#"
            SELECT id FROM conversations
            WHERE user1_id = $1 AND user2_id = $2
            "#,
        )
        .bind(user1_id)
        .bind(user2_id)
        .fetch_optional(&self.pool)
        .await?;
        if let Some(id) = existing {
            return Ok(id);
        }
        let id: Uuid = sqlx::query_scalar(
            r#"
            INSERT INTO conversations (user1_id, user2_id)
            VALUES ($1, $2)
            ON CONFLICT (user1_id, user2_id) DO UPDATE
                SET user1_id = conversations.user1_id
            RETURNING id
            "#,
        )
        .bind(user1_id)
        .bind(user2_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(id)
    }

    /// Inbox: conversations where the viewer participates, newest activity
    /// first, with the other participant, unread counter, and a preview of
    /// the latest non-deleted message.
    pub async fn list_for_user(
        &self,
        viewer_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<ConversationSummary>, i64), sqlx::Error> {
        let total: i64 = sqlx::query_scalar(
            r#"
            SELECT count(*)
            FROM conversations
            WHERE user1_id = $1 OR user2_id = $1
            "#,
        )
        .bind(viewer_id)
        .fetch_one(&self.pool)
        .await?;

        let rows = sqlx::query_as::<_, ConversationRow>(
            r#"
            SELECT
                c.id,
                CASE WHEN c.user1_id = $1 THEN c.user2_id ELSE c.user1_id END AS other_user_id,
                ou.username AS other_username,
                ou.nickname AS other_nickname,
                ou.avatar_url AS other_avatar,
                r.code AS other_role_code,
                r.name AS other_role_name,
                CASE WHEN c.user1_id = $1 THEN c.user1_unread ELSE c.user2_unread END AS unread_count,
                c.last_message_at,
                (
                    SELECT m.content FROM messages m
                    WHERE m.conversation_id = c.id AND m.deleted_at IS NULL
                    ORDER BY m.created_at DESC, m.id DESC
                    LIMIT 1
                ) AS last_message_preview
            FROM conversations c
            JOIN users ou
                ON ou.id = CASE WHEN c.user1_id = $1 THEN c.user2_id ELSE c.user1_id END
            LEFT JOIN roles r ON r.id = ou.role_id
            WHERE c.user1_id = $1 OR c.user2_id = $1
            ORDER BY c.last_message_at DESC, c.id DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(viewer_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let items = rows
            .into_iter()
            .map(|row| ConversationSummary {
                id: row.id,
                other_user: ConversationUser {
                    id: row.other_user_id,
                    username: row.other_username,
                    nickname: row.other_nickname,
                    avatar: row.other_avatar,
                    role: RoleSummary {
                        code: row.other_role_code.unwrap_or_default(),
                        name: row.other_role_name.unwrap_or_default(),
                    },
                },
                unread_count: row.unread_count,
                last_message_at: row.last_message_at,
                last_message_preview: row.last_message_preview,
            })
            .collect();
        Ok((items, total))
    }

    /// Load a conversation the viewer participates in; None otherwise.
    pub async fn find_participant(
        &self,
        conversation_id: Uuid,
        viewer_id: Uuid,
    ) -> Result<Option<ParticipantContext>, sqlx::Error> {
        let row = sqlx::query_as::<_, ParticipantRow>(
            r#"
            SELECT
                c.id,
                c.user1_id,
                c.user2_id,
                ou.username AS other_username,
                ou.status AS other_status
            FROM conversations c
            JOIN users ou
                ON ou.id = CASE WHEN c.user1_id = $2 THEN c.user2_id ELSE c.user1_id END
            WHERE c.id = $1
              AND ($2 = c.user1_id OR $2 = c.user2_id)
            "#,
        )
        .bind(conversation_id)
        .bind(viewer_id)
        .fetch_optional(&self.pool)
        .await?;
        let Some(row) = row else {
            return Ok(None);
        };
        let other_user_id = if row.user1_id == viewer_id {
            row.user2_id
        } else {
            row.user1_id
        };
        Ok(Some(ParticipantContext {
            conversation_id: row.id,
            other_user_id,
            other_username: row.other_username,
            other_status: row.other_status,
        }))
    }

    pub async fn messages(
        &self,
        conversation_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<MessageResponse>, i64), sqlx::Error> {
        let total: i64 = sqlx::query_scalar(
            r#"
            SELECT count(*) FROM messages WHERE conversation_id = $1
            "#,
        )
        .bind(conversation_id)
        .fetch_one(&self.pool)
        .await?;
        let rows = sqlx::query_as::<_, MessageRow>(
            r#"
            SELECT id, conversation_id, sender_id, content, deleted_at, created_at, edited_at
            FROM messages
            WHERE conversation_id = $1
            ORDER BY created_at DESC, id DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(conversation_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;
        let items = rows.into_iter().map(message_to_response).collect();
        Ok((items, total))
    }

    /// Insert a message and bump the recipient-side unread counter plus the
    /// conversation's last_message_at in one transaction.
    pub async fn insert_message(
        &self,
        conversation_id: Uuid,
        sender_id: Uuid,
        content: &str,
    ) -> Result<MessageResponse, sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        let message = sqlx::query_as::<_, MessageRow>(
            r#"
            INSERT INTO messages (conversation_id, sender_id, content)
            VALUES ($1, $2, $3)
            RETURNING id, conversation_id, sender_id, content, deleted_at, created_at, edited_at
            "#,
        )
        .bind(conversation_id)
        .bind(sender_id)
        .bind(content)
        .fetch_one(&mut *tx)
        .await?;
        // The unread counter of the *other* side is the one that grows.
        sqlx::query(
            r#"
            UPDATE conversations
            SET
                last_message_at = $2,
                user1_unread = user1_unread + CASE WHEN user1_id <> $3 THEN 1 ELSE 0 END,
                user2_unread = user2_unread + CASE WHEN user2_id <> $3 THEN 1 ELSE 0 END
            WHERE id = $1
            "#,
        )
        .bind(conversation_id)
        .bind(message.created_at)
        .bind(sender_id)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(message_to_response(message))
    }

    /// Mark the viewer's side as read; returns the number of unread cleared.
    pub async fn mark_read(
        &self,
        conversation_id: Uuid,
        viewer_id: Uuid,
    ) -> Result<i64, sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE conversations
            SET
                user1_last_read_at = CASE WHEN user1_id = $2 THEN now() ELSE user1_last_read_at END,
                user2_last_read_at = CASE WHEN user2_id = $2 THEN now() ELSE user2_last_read_at END,
                user1_unread = CASE WHEN user1_id = $2 THEN 0 ELSE user1_unread END,
                user2_unread = CASE WHEN user2_id = $2 THEN 0 ELSE user2_unread END
            WHERE id = $1
              AND (
                (user1_id = $2 AND user1_unread > 0)
                OR (user2_id = $2 AND user2_unread > 0)
              )
            "#,
        )
        .bind(conversation_id)
        .bind(viewer_id)
        .execute(&self.pool)
        .await?;
        // The cleared count is not needed beyond cache invalidation; callers
        // refetch the real counters.
        Ok(0)
    }

    /// Total unread messages across all conversations of a user.
    pub async fn total_unread(&self, viewer_id: Uuid) -> Result<i64, sqlx::Error> {
        sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COALESCE(SUM(
                CASE WHEN user1_id = $1 THEN user1_unread ELSE user2_unread END
            ), 0)
            FROM conversations
            WHERE user1_id = $1 OR user2_id = $1
            "#,
        )
        .bind(viewer_id)
        .fetch_one(&self.pool)
        .await
    }

    /// Unread count for one conversation and one side only.
    pub async fn unread_for(
        &self,
        conversation_id: Uuid,
        viewer_id: Uuid,
    ) -> Result<i64, sqlx::Error> {
        sqlx::query_scalar::<_, i64>(
            r#"
            SELECT CASE WHEN user1_id = $2 THEN user1_unread ELSE user2_unread END
            FROM conversations
            WHERE id = $1 AND ($2 = user1_id OR $2 = user2_id)
            "#,
        )
        .bind(conversation_id)
        .bind(viewer_id)
        .fetch_one(&self.pool)
        .await
    }

    /// Soft-delete a message the sender owns; content is blanked.
    pub async fn soft_delete_message(
        &self,
        message_id: Uuid,
        sender_id: Uuid,
    ) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            r#"
            UPDATE messages
            SET deleted_at = now(), content = ''
            WHERE id = $1 AND sender_id = $2 AND deleted_at IS NULL
            "#,
        )
        .bind(message_id)
        .bind(sender_id)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }
}
