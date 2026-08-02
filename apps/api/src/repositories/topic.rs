use chrono::{DateTime, Utc};
use sqlx::{PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

use crate::models::{
    CategorySummary, TopicAuthorSummary, TopicDetail, TopicListSort, TopicStats, TopicSummary,
};

#[derive(Clone)]
pub struct TopicRepository {
    pool: PgPool,
}

#[derive(sqlx::FromRow)]
pub struct RepositoryTopic {
    pub id: Uuid,
    pub category_id: Uuid,
    pub author_id: Uuid,
    pub title: String,
    pub slug: String,
    pub content: Option<String>,
    pub summary: Option<String>,
    pub status: String,
    pub view_count: i64,
    pub reply_count: i64,
    pub like_count: i64,
    pub is_pinned: bool,
    pub is_featured: bool,
    pub last_reply_at: Option<DateTime<Utc>>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub category_slug: String,
    pub category_name: String,
    pub category_icon: Option<String>,
    pub category_is_visible: bool,
    pub author_username: String,
    pub author_nickname: Option<String>,
    pub author_avatar: Option<String>,
    pub author_role_code: String,
    pub author_role_name: String,
    pub has_poll: bool,
}

pub struct TopicListOptions<'a> {
    pub category_slug: Option<&'a str>,
    pub author_id: Option<Uuid>,
    pub sort: TopicListSort,
    pub limit: i64,
    pub offset: i64,
}

pub struct NewTopic<'a> {
    pub category_id: Uuid,
    pub author_id: Uuid,
    pub title: &'a str,
    pub slug: &'a str,
    pub content: &'a str,
    pub summary: Option<&'a str>,
    /// Initial status: "published" or "hidden" (auto-moderation).
    pub status: &'a str,
}

pub struct TopicUpdate<'a> {
    pub category_id: Option<Uuid>,
    pub title: Option<&'a str>,
    pub content: Option<&'a str>,
    pub summary_changed: bool,
    pub summary: Option<&'a str>,
}

pub struct TopicModeration {
    pub is_pinned: Option<bool>,
    pub is_featured: Option<bool>,
}

impl TopicRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn list(
        &self,
        options: TopicListOptions<'_>,
    ) -> Result<(Vec<RepositoryTopic>, i64), sqlx::Error> {
        let mut items = QueryBuilder::<Postgres>::new(TOPIC_LIST_SELECT);
        push_public_filters(&mut items, options.category_slug, options.author_id, options.sort);
        push_order(&mut items, options.sort);
        items
            .push(" LIMIT ")
            .push_bind(options.limit)
            .push(" OFFSET ")
            .push_bind(options.offset);

        let rows = items
            .build_query_as::<RepositoryTopic>()
            .fetch_all(&self.pool)
            .await?;

        let mut count = QueryBuilder::<Postgres>::new(
            "SELECT count(*) FROM topics t JOIN categories c ON c.id = t.category_id",
        );
        push_public_filters(&mut count, options.category_slug, options.author_id, options.sort);
        let total = count
            .build_query_scalar::<i64>()
            .fetch_one(&self.pool)
            .await?;

        Ok((rows, total))
    }

    pub async fn find_published_by_slug_and_increment(
        &self,
        slug: &str,
    ) -> Result<Option<RepositoryTopic>, sqlx::Error> {
        sqlx::query_as::<_, RepositoryTopic>(TOPIC_BY_SLUG_AND_INCREMENT)
            .bind(slug)
            .fetch_optional(&self.pool)
            .await
    }

    pub async fn find_by_id(&self, topic_id: Uuid) -> Result<Option<RepositoryTopic>, sqlx::Error> {
        sqlx::query_as::<_, RepositoryTopic>(TOPIC_BY_ID)
            .bind(topic_id)
            .fetch_optional(&self.pool)
            .await
    }

    pub async fn create(&self, topic: NewTopic<'_>) -> Result<RepositoryTopic, sqlx::Error> {
        let id = sqlx::query_scalar::<_, Uuid>(
            r#"
            INSERT INTO topics (category_id, author_id, title, slug, content, summary, status)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING id
            "#,
        )
        .bind(topic.category_id)
        .bind(topic.author_id)
        .bind(topic.title)
        .bind(topic.slug)
        .bind(topic.content)
        .bind(topic.summary)
        .bind(topic.status)
        .fetch_one(&self.pool)
        .await?;

        self.find_by_id(id).await?.ok_or(sqlx::Error::RowNotFound)
    }

    pub async fn update(
        &self,
        topic_id: Uuid,
        update: TopicUpdate<'_>,
    ) -> Result<Option<RepositoryTopic>, sqlx::Error> {
        let id = sqlx::query_scalar::<_, Uuid>(
            r#"
            UPDATE topics
            SET category_id = COALESCE($2, category_id),
                title = COALESCE($3, title),
                content = COALESCE($4, content),
                summary = CASE WHEN $5 THEN $6 ELSE summary END
            WHERE id = $1 AND status = 'published'
            RETURNING id
            "#,
        )
        .bind(topic_id)
        .bind(update.category_id)
        .bind(update.title)
        .bind(update.content)
        .bind(update.summary_changed)
        .bind(update.summary)
        .fetch_optional(&self.pool)
        .await?;

        match id {
            Some(id) => self.find_by_id(id).await,
            None => Ok(None),
        }
    }

    pub async fn moderate(
        &self,
        topic_id: Uuid,
        moderation: TopicModeration,
    ) -> Result<Option<RepositoryTopic>, sqlx::Error> {
        let id = sqlx::query_scalar::<_, Uuid>(
            r#"
            UPDATE topics
            SET is_pinned = COALESCE($2, is_pinned),
                is_featured = COALESCE($3, is_featured)
            WHERE id = $1 AND status = 'published'
            RETURNING id
            "#,
        )
        .bind(topic_id)
        .bind(moderation.is_pinned)
        .bind(moderation.is_featured)
        .fetch_optional(&self.pool)
        .await?;

        match id {
            Some(id) => self.find_by_id(id).await,
            None => Ok(None),
        }
    }

    pub async fn soft_delete(&self, topic_id: Uuid) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            r#"
            UPDATE topics
            SET status = 'deleted',
                deleted_at = now(),
                is_pinned = false,
                is_featured = false
            WHERE id = $1 AND status = 'published'
            "#,
        )
        .bind(topic_id)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() == 1)
    }
}

pub fn repository_topic_to_summary(topic: RepositoryTopic) -> TopicSummary {
    let category = category_summary(&topic);
    let author = author_summary(&topic);
    let stats = topic_stats(&topic);
    TopicSummary {
        id: topic.id,
        title: topic.title,
        slug: topic.slug,
        summary: topic.summary,
        category,
        author,
        stats,
        is_pinned: topic.is_pinned,
        is_featured: topic.is_featured,
        last_reply_at: topic.last_reply_at,
        created_at: topic.created_at,
        updated_at: topic.updated_at,
        has_poll: topic.has_poll,
    }
}

pub fn repository_topic_to_detail(topic: RepositoryTopic) -> Result<TopicDetail, &'static str> {
    let content = topic
        .content
        .clone()
        .ok_or("topic content was not selected")?;
    Ok(TopicDetail {
        id: topic.id,
        title: topic.title.clone(),
        slug: topic.slug.clone(),
        content,
        summary: topic.summary.clone(),
        category: category_summary(&topic),
        author: author_summary(&topic),
        stats: topic_stats(&topic),
        is_pinned: topic.is_pinned,
        is_featured: topic.is_featured,
        last_reply_at: topic.last_reply_at,
        created_at: topic.created_at,
        updated_at: topic.updated_at,
        has_poll: topic.has_poll,
        liked_by_me: false,
        favorited_by_me: false,
        following_author: false,
    })
}

fn category_summary(topic: &RepositoryTopic) -> CategorySummary {
    CategorySummary {
        id: topic.category_id,
        slug: topic.category_slug.clone(),
        name: topic.category_name.clone(),
        icon: topic.category_icon.clone(),
    }
}

fn author_summary(topic: &RepositoryTopic) -> TopicAuthorSummary {
    TopicAuthorSummary {
        id: topic.author_id,
        username: topic.author_username.clone(),
        nickname: topic.author_nickname.clone(),
        avatar: topic.author_avatar.clone(),
        role: crate::models::RoleSummary {
            code: topic.author_role_code.clone(),
            name: topic.author_role_name.clone(),
        },
    }
}

fn topic_stats(topic: &RepositoryTopic) -> TopicStats {
    TopicStats {
        views: topic.view_count,
        replies: topic.reply_count,
        likes: topic.like_count,
    }
}

fn push_public_filters(
    query: &mut QueryBuilder<'_, Postgres>,
    category_slug: Option<&str>,
    author_id: Option<Uuid>,
    sort: TopicListSort,
) {
    query.push(" WHERE t.status = 'published' AND c.is_visible = true");
    if let Some(category_slug) = category_slug {
        query
            .push(" AND c.slug = ")
            .push_bind(category_slug.to_owned());
    }
    if let Some(author_id) = author_id {
        query.push(" AND t.author_id = ").push_bind(author_id);
    }
    match sort {
        TopicListSort::Featured => {
            query.push(" AND t.is_featured = true");
        }
        TopicListSort::Pinned => {
            query.push(" AND t.is_pinned = true");
        }
        TopicListSort::Latest | TopicListSort::Hot => {}
    }
}

fn push_order(query: &mut QueryBuilder<'_, Postgres>, sort: TopicListSort) {
    match sort {
        TopicListSort::Latest | TopicListSort::Featured | TopicListSort::Pinned => {
            query.push(" ORDER BY t.created_at DESC, t.id DESC");
        }
        TopicListSort::Hot => {
            query.push(" ORDER BY t.view_count DESC, t.created_at DESC, t.id DESC");
        }
    }
}

const TOPIC_LIST_SELECT: &str = r#"
    SELECT
        t.id,
        t.category_id,
        t.author_id,
        t.title,
        t.slug,
        NULL::text AS content,
        t.summary,
        t.status,
        t.view_count,
        t.reply_count,
        t.like_count,
        t.is_pinned,
        t.is_featured,
        t.last_reply_at,
        t.deleted_at,
        t.created_at,
        t.updated_at,
        c.slug AS category_slug,
        c.name AS category_name,
        c.icon AS category_icon,
        c.is_visible AS category_is_visible,
        u.username AS author_username,
        u.nickname AS author_nickname,
        u.avatar_url AS author_avatar,
        r.code AS author_role_code,
        r.name AS author_role_name,
        EXISTS (SELECT 1 FROM polls poll WHERE poll.topic_id = t.id) AS has_poll
    FROM topics t
    JOIN categories c ON c.id = t.category_id
    JOIN users u ON u.id = t.author_id
    JOIN roles r ON r.id = u.role_id
"#;

const TOPIC_BY_ID: &str = r#"
    SELECT
        t.id,
        t.category_id,
        t.author_id,
        t.title,
        t.slug,
        t.content,
        t.summary,
        t.status,
        t.view_count,
        t.reply_count,
        t.like_count,
        t.is_pinned,
        t.is_featured,
        t.last_reply_at,
        t.deleted_at,
        t.created_at,
        t.updated_at,
        c.slug AS category_slug,
        c.name AS category_name,
        c.icon AS category_icon,
        c.is_visible AS category_is_visible,
        u.username AS author_username,
        u.nickname AS author_nickname,
        u.avatar_url AS author_avatar,
        r.code AS author_role_code,
        r.name AS author_role_name,
        EXISTS (SELECT 1 FROM polls poll WHERE poll.topic_id = t.id) AS has_poll
    FROM topics t
    JOIN categories c ON c.id = t.category_id
    JOIN users u ON u.id = t.author_id
    JOIN roles r ON r.id = u.role_id
    WHERE t.id = $1
"#;

const TOPIC_BY_SLUG_AND_INCREMENT: &str = r#"
    WITH viewed AS (
        UPDATE topics AS topic
        SET view_count = topic.view_count + 1
        WHERE topic.slug = $1
          AND topic.status = 'published'
          AND EXISTS (
              SELECT 1
              FROM categories visible_category
              WHERE visible_category.id = topic.category_id
                AND visible_category.is_visible = true
          )
        RETURNING topic.*
    )
    SELECT
        viewed.id,
        viewed.category_id,
        viewed.author_id,
        viewed.title,
        viewed.slug,
        viewed.content,
        viewed.summary,
        viewed.status,
        viewed.view_count,
        viewed.reply_count,
        viewed.like_count,
        viewed.is_pinned,
        viewed.is_featured,
        viewed.last_reply_at,
        viewed.deleted_at,
        viewed.created_at,
        viewed.updated_at,
        c.slug AS category_slug,
        c.name AS category_name,
        c.icon AS category_icon,
        c.is_visible AS category_is_visible,
        u.username AS author_username,
        u.nickname AS author_nickname,
        u.avatar_url AS author_avatar,
        r.code AS author_role_code,
        r.name AS author_role_name,
        EXISTS (SELECT 1 FROM polls poll WHERE poll.topic_id = viewed.id) AS has_poll
    FROM viewed
    JOIN categories c ON c.id = viewed.category_id
    JOIN users u ON u.id = viewed.author_id
    JOIN roles r ON r.id = u.role_id
"#;
