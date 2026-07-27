use chrono::{DateTime, Utc};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::models::{CommentNode, CommentStats, RoleSummary, TopicAuthorSummary};

#[derive(Clone)]
pub struct CommentRepository {
    pool: PgPool,
}

#[derive(sqlx::FromRow)]
pub struct RepositoryComment {
    pub id: Uuid,
    pub topic_id: Uuid,
    pub author_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub content: String,
    pub status: String,
    pub like_count: i64,
    pub reply_count: i64,
    pub edited_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub author_username: String,
    pub author_nickname: Option<String>,
    pub author_avatar: Option<String>,
    pub author_role_code: String,
    pub author_role_name: String,
}

pub struct NewComment<'a> {
    pub topic_id: Uuid,
    pub author_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub content: &'a str,
}

impl CommentRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_tree_page(
        &self,
        topic_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<CommentNode>, i64), sqlx::Error> {
        let total = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT count(*)
            FROM comments
            WHERE topic_id = $1
              AND parent_id IS NULL
              AND status = 'published'
            "#,
        )
        .bind(topic_id)
        .fetch_one(&self.pool)
        .await?;

        let roots = sqlx::query_as::<_, RepositoryComment>(
            r#"
            SELECT
                c.id, c.topic_id, c.author_id, c.parent_id, c.content, c.status,
                c.like_count, c.reply_count, c.edited_at, c.created_at, c.updated_at, c.deleted_at,
                u.username AS author_username,
                u.nickname AS author_nickname,
                u.avatar AS author_avatar,
                r.code AS author_role_code,
                r.name AS author_role_name
            FROM comments c
            JOIN users u ON u.id = c.author_id
            JOIN roles r ON r.id = u.role_id
            WHERE c.topic_id = $1
              AND c.parent_id IS NULL
              AND c.status = 'published'
            ORDER BY c.created_at ASC, c.id ASC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(topic_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        if roots.is_empty() {
            return Ok((Vec::new(), total));
        }

        let root_ids: Vec<Uuid> = roots.iter().map(|c| c.id).collect();
        let children = sqlx::query_as::<_, RepositoryComment>(
            r#"
            SELECT
                c.id, c.topic_id, c.author_id, c.parent_id, c.content, c.status,
                c.like_count, c.reply_count, c.edited_at, c.created_at, c.updated_at, c.deleted_at,
                u.username AS author_username,
                u.nickname AS author_nickname,
                u.avatar AS author_avatar,
                r.code AS author_role_code,
                r.name AS author_role_name
            FROM comments c
            JOIN users u ON u.id = c.author_id
            JOIN roles r ON r.id = u.role_id
            WHERE c.parent_id = ANY($1)
              AND c.status = 'published'
            ORDER BY c.created_at ASC, c.id ASC
            "#,
        )
        .bind(&root_ids)
        .fetch_all(&self.pool)
        .await?;

        Ok((assemble_tree(roots, children), total))
    }

    pub async fn find_by_id(
        &self,
        comment_id: Uuid,
    ) -> Result<Option<RepositoryComment>, sqlx::Error> {
        sqlx::query_as::<_, RepositoryComment>(COMMENT_BY_ID)
            .bind(comment_id)
            .fetch_optional(&self.pool)
            .await
    }

    pub async fn create(&self, comment: NewComment<'_>) -> Result<RepositoryComment, sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        let id = sqlx::query_scalar::<_, Uuid>(
            r#"
            INSERT INTO comments (topic_id, author_id, parent_id, content)
            VALUES ($1, $2, $3, $4)
            RETURNING id
            "#,
        )
        .bind(comment.topic_id)
        .bind(comment.author_id)
        .bind(comment.parent_id)
        .bind(comment.content)
        .fetch_one(&mut *tx)
        .await?;

        if let Some(parent_id) = comment.parent_id {
            sqlx::query(
                r#"
                UPDATE comments
                SET reply_count = reply_count + 1
                WHERE id = $1 AND status = 'published'
                "#,
            )
            .bind(parent_id)
            .execute(&mut *tx)
            .await?;
        }

        recompute_topic_stats(&mut tx, comment.topic_id).await?;
        tx.commit().await?;

        self.find_by_id(id).await?.ok_or(sqlx::Error::RowNotFound)
    }

    pub async fn update_content(
        &self,
        comment_id: Uuid,
        content: &str,
    ) -> Result<Option<RepositoryComment>, sqlx::Error> {
        let id = sqlx::query_scalar::<_, Uuid>(
            r#"
            UPDATE comments
            SET content = $2,
                edited_at = now()
            WHERE id = $1 AND status = 'published'
            RETURNING id
            "#,
        )
        .bind(comment_id)
        .bind(content)
        .fetch_optional(&self.pool)
        .await?;

        match id {
            Some(id) => self.find_by_id(id).await,
            None => Ok(None),
        }
    }

    pub async fn soft_delete(&self, comment_id: Uuid) -> Result<bool, sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        let row = sqlx::query_as::<_, (Uuid, Uuid, Option<Uuid>)>(
            r#"
            SELECT id, topic_id, parent_id
            FROM comments
            WHERE id = $1 AND status = 'published'
            FOR UPDATE
            "#,
        )
        .bind(comment_id)
        .fetch_optional(&mut *tx)
        .await?;

        let Some((id, topic_id, parent_id)) = row else {
            return Ok(false);
        };

        // Cascade soft-delete children when deleting a root.
        if parent_id.is_none() {
            sqlx::query(
                r#"
                UPDATE comments
                SET status = 'deleted',
                    deleted_at = now()
                WHERE parent_id = $1 AND status = 'published'
                "#,
            )
            .bind(id)
            .execute(&mut *tx)
            .await?;
        } else if let Some(parent_id) = parent_id {
            sqlx::query(
                r#"
                UPDATE comments
                SET reply_count = GREATEST(reply_count - 1, 0)
                WHERE id = $1
                "#,
            )
            .bind(parent_id)
            .execute(&mut *tx)
            .await?;
        }

        sqlx::query(
            r#"
            UPDATE comments
            SET status = 'deleted',
                deleted_at = now(),
                reply_count = CASE WHEN parent_id IS NULL THEN 0 ELSE reply_count END
            WHERE id = $1 AND status = 'published'
            "#,
        )
        .bind(id)
        .execute(&mut *tx)
        .await?;

        recompute_topic_stats(&mut tx, topic_id).await?;
        tx.commit().await?;
        Ok(true)
    }

    pub async fn restore(
        &self,
        comment_id: Uuid,
    ) -> Result<Option<RepositoryComment>, sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        let row = sqlx::query_as::<_, RepositoryComment>(
            r#"
            SELECT
                c.id, c.topic_id, c.author_id, c.parent_id, c.content, c.status,
                c.like_count, c.reply_count, c.edited_at, c.created_at, c.updated_at, c.deleted_at,
                u.username AS author_username,
                u.nickname AS author_nickname,
                u.avatar AS author_avatar,
                r.code AS author_role_code,
                r.name AS author_role_name
            FROM comments c
            JOIN users u ON u.id = c.author_id
            JOIN roles r ON r.id = u.role_id
            WHERE c.id = $1
            FOR UPDATE OF c
            "#,
        )
        .bind(comment_id)
        .fetch_optional(&mut *tx)
        .await?;

        let Some(comment) = row else {
            return Ok(None);
        };
        if comment.status == "published" {
            return Ok(Some(comment));
        }

        if let Some(parent_id) = comment.parent_id {
            let parent_ok = sqlx::query_scalar::<_, bool>(
                r#"
                SELECT status = 'published' AND parent_id IS NULL AND topic_id = $2
                FROM comments
                WHERE id = $1
                "#,
            )
            .bind(parent_id)
            .bind(comment.topic_id)
            .fetch_optional(&mut *tx)
            .await?
            .unwrap_or(false);
            if !parent_ok {
                return Err(sqlx::Error::RowNotFound);
            }
            sqlx::query(
                r#"
                UPDATE comments
                SET reply_count = reply_count + 1
                WHERE id = $1 AND status = 'published'
                "#,
            )
            .bind(parent_id)
            .execute(&mut *tx)
            .await?;
        }

        sqlx::query(
            r#"
            UPDATE comments
            SET status = 'published',
                deleted_at = NULL
            WHERE id = $1
            "#,
        )
        .bind(comment_id)
        .execute(&mut *tx)
        .await?;

        recompute_topic_stats(&mut tx, comment.topic_id).await?;
        tx.commit().await?;
        self.find_by_id(comment_id).await
    }
}

async fn recompute_topic_stats(
    tx: &mut Transaction<'_, Postgres>,
    topic_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        WITH stats AS (
            SELECT
                count(*) FILTER (WHERE status = 'published') AS reply_count,
                max(created_at) FILTER (WHERE status = 'published') AS last_reply_at
            FROM comments
            WHERE topic_id = $1
        ),
        last_author AS (
            SELECT author_id
            FROM comments
            WHERE topic_id = $1 AND status = 'published'
            ORDER BY created_at DESC, id DESC
            LIMIT 1
        )
        UPDATE topics
        SET reply_count = COALESCE(stats.reply_count, 0),
            last_reply_at = stats.last_reply_at,
            last_reply_user_id = last_author.author_id
        FROM stats
        LEFT JOIN last_author ON true
        WHERE topics.id = $1
        "#,
    )
    .bind(topic_id)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

fn assemble_tree(
    roots: Vec<RepositoryComment>,
    children: Vec<RepositoryComment>,
) -> Vec<CommentNode> {
    let mut map: std::collections::HashMap<Uuid, Vec<CommentNode>> =
        std::collections::HashMap::new();
    for child in children {
        let parent_id = child.parent_id.expect("child has parent");
        map.entry(parent_id)
            .or_default()
            .push(repository_comment_to_node(child, Vec::new()));
    }

    roots
        .into_iter()
        .map(|root| {
            let replies = map.remove(&root.id).unwrap_or_default();
            repository_comment_to_node(root, replies)
        })
        .collect()
}

pub fn repository_comment_to_node(
    comment: RepositoryComment,
    replies: Vec<CommentNode>,
) -> CommentNode {
    CommentNode {
        id: comment.id,
        topic_id: comment.topic_id,
        parent_id: comment.parent_id,
        content: comment.content,
        author: TopicAuthorSummary {
            id: comment.author_id,
            username: comment.author_username,
            nickname: comment.author_nickname,
            avatar: comment.author_avatar,
            role: RoleSummary {
                code: comment.author_role_code,
                name: comment.author_role_name,
            },
        },
        stats: CommentStats {
            likes: comment.like_count,
            replies: comment.reply_count,
        },
        edited_at: comment.edited_at,
        created_at: comment.created_at,
        updated_at: comment.updated_at,
        replies,
    }
}

const COMMENT_BY_ID: &str = r#"
    SELECT
        c.id, c.topic_id, c.author_id, c.parent_id, c.content, c.status,
        c.like_count, c.reply_count, c.edited_at, c.created_at, c.updated_at, c.deleted_at,
        u.username AS author_username,
        u.nickname AS author_nickname,
        u.avatar AS author_avatar,
        r.code AS author_role_code,
        r.name AS author_role_name
    FROM comments c
    JOIN users u ON u.id = c.author_id
    JOIN roles r ON r.id = u.role_id
    WHERE c.id = $1
"#;
