use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{
    CategorySummary, CommentSearchHit, RoleSummary, SearchAuthor, SearchSort, SearchTopicStats,
    TopicSearchHit, UserSearchHit,
};

#[derive(Clone)]
pub struct SearchRepository {
    pool: PgPool,
}

pub struct TopicSearchFilter<'a> {
    pub keyword: &'a str,
    pub category_id: Option<Uuid>,
    pub author_id: Option<Uuid>,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    pub has_poll: Option<bool>,
    pub sort: SearchSort,
    pub limit: i64,
    pub offset: i64,
}

pub struct CommentSearchFilter<'a> {
    pub keyword: &'a str,
    pub author_id: Option<Uuid>,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    pub sort: SearchSort,
    pub limit: i64,
    pub offset: i64,
}

pub struct UserSearchFilter<'a> {
    pub keyword: &'a str,
    pub sort: SearchSort,
    pub limit: i64,
    pub offset: i64,
}

#[derive(sqlx::FromRow)]
struct TopicHitRow {
    id: Uuid,
    title: String,
    slug: String,
    summary: Option<String>,
    highlight: String,
    view_count: i64,
    reply_count: i64,
    like_count: i64,
    created_at: DateTime<Utc>,
    rank: f32,
    category_id: Uuid,
    category_slug: String,
    category_name: String,
    category_icon: Option<String>,
    author_id: Uuid,
    author_username: String,
    author_nickname: Option<String>,
    author_avatar: Option<String>,
    author_role_code: String,
    author_role_name: String,
    has_poll: bool,
}

#[derive(sqlx::FromRow)]
struct CommentHitRow {
    id: Uuid,
    topic_id: Uuid,
    topic_slug: String,
    topic_title: String,
    content_preview: String,
    highlight: String,
    like_count: i64,
    created_at: DateTime<Utc>,
    rank: f32,
    author_id: Uuid,
    author_username: String,
    author_nickname: Option<String>,
    author_avatar: Option<String>,
    author_role_code: String,
    author_role_name: String,
}

#[derive(sqlx::FromRow)]
struct UserHitRow {
    id: Uuid,
    username: String,
    nickname: Option<String>,
    avatar: Option<String>,
    role_code: String,
    role_name: String,
    followers_count: i64,
    following_count: i64,
    highlight: String,
    created_at: DateTime<Utc>,
    rank: f32,
}

impl SearchRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn search_topics(
        &self,
        filter: TopicSearchFilter<'_>,
    ) -> Result<(Vec<TopicSearchHit>, i64), sqlx::Error> {
        let total = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT count(*)
            FROM topics t
            JOIN categories c ON c.id = t.category_id
            JOIN users u ON u.id = t.author_id
            WHERE t.status = 'published'
              AND c.is_visible = true
              AND ($2::uuid IS NULL OR t.category_id = $2)
              AND ($3::uuid IS NULL OR t.author_id = $3)
              AND ($4::timestamptz IS NULL OR t.created_at >= $4)
              AND ($5::timestamptz IS NULL OR t.created_at <= $5)
              AND ($6::boolean IS NULL
                   OR ($6 = true AND EXISTS (SELECT 1 FROM polls poll WHERE poll.topic_id = t.id))
                   OR ($6 = false AND NOT EXISTS (SELECT 1 FROM polls poll WHERE poll.topic_id = t.id)))
              AND (
                    t.search_vector @@ plainto_tsquery('simple', $1)
                    OR t.title ILIKE '%' || $1 || '%' ESCAPE '\'
                    OR coalesce(t.summary, '') ILIKE '%' || $1 || '%' ESCAPE '\'
                    OR u.username ILIKE '%' || $1 || '%' ESCAPE '\'
                    OR coalesce(u.nickname, '') ILIKE '%' || $1 || '%' ESCAPE '\'
                  )
            "#,
        )
        .bind(filter.keyword)
        .bind(filter.category_id)
        .bind(filter.author_id)
        .bind(filter.from)
        .bind(filter.to)
        .bind(filter.has_poll)
        .fetch_one(&self.pool)
        .await?;

        let order_sql = match filter.sort {
            SearchSort::Latest => "t.created_at DESC, t.id DESC",
            SearchSort::Hot => "t.view_count DESC, t.like_count DESC, t.created_at DESC, t.id DESC",
            SearchSort::Relevance => "rank DESC, t.like_count DESC, t.created_at DESC, t.id DESC",
        };

        let sql = format!(
            r#"
            SELECT
                t.id,
                t.title,
                t.slug,
                t.summary,
                ts_headline(
                    'simple',
                    coalesce(t.summary, left(t.content, 280), t.title),
                    plainto_tsquery('simple', $1),
                    'MaxFragments=1, MaxWords=18, MinWords=5, StartSel=<<, StopSel=>>'
                ) AS highlight,
                t.view_count,
                t.reply_count,
                t.like_count,
                t.created_at,
                (
                    ts_rank_cd(t.search_vector, plainto_tsquery('simple', $1))
                    + GREATEST(similarity(t.title, $1), 0)
                    + CASE
                        WHEN t.title ILIKE '%' || $1 || '%' ESCAPE '\' THEN 0.35
                        WHEN coalesce(t.summary, '') ILIKE '%' || $1 || '%' ESCAPE '\' THEN 0.2
                        WHEN u.username ILIKE '%' || $1 || '%' ESCAPE '\'
                          OR coalesce(u.nickname, '') ILIKE '%' || $1 || '%' ESCAPE '\' THEN 0.15
                        ELSE 0
                      END
                )::real AS rank,
                c.id AS category_id,
                c.slug AS category_slug,
                c.name AS category_name,
                c.icon AS category_icon,
                u.id AS author_id,
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
            WHERE t.status = 'published'
              AND c.is_visible = true
              AND ($2::uuid IS NULL OR t.category_id = $2)
              AND ($3::uuid IS NULL OR t.author_id = $3)
              AND ($4::timestamptz IS NULL OR t.created_at >= $4)
              AND ($5::timestamptz IS NULL OR t.created_at <= $5)
              AND ($6::boolean IS NULL
                   OR ($6 = true AND EXISTS (SELECT 1 FROM polls poll WHERE poll.topic_id = t.id))
                   OR ($6 = false AND NOT EXISTS (SELECT 1 FROM polls poll WHERE poll.topic_id = t.id)))
              AND (
                    t.search_vector @@ plainto_tsquery('simple', $1)
                    OR t.title ILIKE '%' || $1 || '%' ESCAPE '\'
                    OR coalesce(t.summary, '') ILIKE '%' || $1 || '%' ESCAPE '\'
                    OR u.username ILIKE '%' || $1 || '%' ESCAPE '\'
                    OR coalesce(u.nickname, '') ILIKE '%' || $1 || '%' ESCAPE '\'
                  )
            ORDER BY {order_sql}
            LIMIT $7 OFFSET $8
            "#
        );

        let rows = sqlx::query_as::<_, TopicHitRow>(&sql)
            .bind(filter.keyword)
            .bind(filter.category_id)
            .bind(filter.author_id)
            .bind(filter.from)
            .bind(filter.to)
            .bind(filter.has_poll)
            .bind(filter.limit)
            .bind(filter.offset)
            .fetch_all(&self.pool)
            .await?;

        Ok((rows.into_iter().map(map_topic_hit).collect(), total))
    }

    pub async fn search_comments(
        &self,
        filter: CommentSearchFilter<'_>,
    ) -> Result<(Vec<CommentSearchHit>, i64), sqlx::Error> {
        let total = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT count(*)
            FROM comments cm
            JOIN topics t ON t.id = cm.topic_id
            JOIN categories c ON c.id = t.category_id
            WHERE cm.status = 'published'
              AND t.status = 'published'
              AND c.is_visible = true
              AND ($2::uuid IS NULL OR cm.author_id = $2)
              AND ($3::timestamptz IS NULL OR cm.created_at >= $3)
              AND ($4::timestamptz IS NULL OR cm.created_at <= $4)
              AND (
                    cm.search_vector @@ plainto_tsquery('simple', $1)
                    OR cm.content ILIKE '%' || $1 || '%' ESCAPE '\'
                  )
            "#,
        )
        .bind(filter.keyword)
        .bind(filter.author_id)
        .bind(filter.from)
        .bind(filter.to)
        .fetch_one(&self.pool)
        .await?;

        let order_sql = match filter.sort {
            SearchSort::Latest => "cm.created_at DESC, cm.id DESC",
            SearchSort::Hot => "cm.like_count DESC, cm.created_at DESC, cm.id DESC",
            SearchSort::Relevance => {
                "rank DESC, cm.like_count DESC, cm.created_at DESC, cm.id DESC"
            }
        };

        let sql = format!(
            r#"
            SELECT
                cm.id,
                cm.topic_id,
                t.slug AS topic_slug,
                t.title AS topic_title,
                left(cm.content, 200) AS content_preview,
                ts_headline(
                    'simple',
                    cm.content,
                    plainto_tsquery('simple', $1),
                    'MaxFragments=1, MaxWords=20, MinWords=6, StartSel=<<, StopSel=>>'
                ) AS highlight,
                cm.like_count,
                cm.created_at,
                (
                    ts_rank_cd(cm.search_vector, plainto_tsquery('simple', $1))
                    + CASE
                        WHEN cm.content ILIKE '%' || $1 || '%' ESCAPE '\' THEN 0.3
                        ELSE 0
                      END
                )::real AS rank,
                u.id AS author_id,
                u.username AS author_username,
                u.nickname AS author_nickname,
                u.avatar_url AS author_avatar,
                r.code AS author_role_code,
                r.name AS author_role_name
            FROM comments cm
            JOIN topics t ON t.id = cm.topic_id
            JOIN categories c ON c.id = t.category_id
            JOIN users u ON u.id = cm.author_id
            JOIN roles r ON r.id = u.role_id
            WHERE cm.status = 'published'
              AND t.status = 'published'
              AND c.is_visible = true
              AND ($2::uuid IS NULL OR cm.author_id = $2)
              AND ($3::timestamptz IS NULL OR cm.created_at >= $3)
              AND ($4::timestamptz IS NULL OR cm.created_at <= $4)
              AND (
                    cm.search_vector @@ plainto_tsquery('simple', $1)
                    OR cm.content ILIKE '%' || $1 || '%' ESCAPE '\'
                  )
            ORDER BY {order_sql}
            LIMIT $5 OFFSET $6
            "#
        );

        let rows = sqlx::query_as::<_, CommentHitRow>(&sql)
            .bind(filter.keyword)
            .bind(filter.author_id)
            .bind(filter.from)
            .bind(filter.to)
            .bind(filter.limit)
            .bind(filter.offset)
            .fetch_all(&self.pool)
            .await?;

        Ok((rows.into_iter().map(map_comment_hit).collect(), total))
    }

    pub async fn search_users(
        &self,
        filter: UserSearchFilter<'_>,
    ) -> Result<(Vec<UserSearchHit>, i64), sqlx::Error> {
        let total = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT count(*)
            FROM users u
            WHERE u.status = 'active'
              AND (
                    u.search_vector @@ plainto_tsquery('simple', $1)
                    OR u.username ILIKE '%' || $1 || '%' ESCAPE '\'
                    OR coalesce(u.nickname, '') ILIKE '%' || $1 || '%' ESCAPE '\'
                  )
            "#,
        )
        .bind(filter.keyword)
        .fetch_one(&self.pool)
        .await?;

        let order_sql = match filter.sort {
            SearchSort::Latest => "u.created_at DESC, u.id DESC",
            SearchSort::Hot => "u.followers_count DESC, u.created_at DESC, u.id DESC",
            SearchSort::Relevance => {
                "rank DESC, u.followers_count DESC, u.created_at DESC, u.id DESC"
            }
        };

        let sql = format!(
            r#"
            SELECT
                u.id,
                u.username,
                u.nickname,
                u.avatar_url AS avatar,
                r.code AS role_code,
                r.name AS role_name,
                u.followers_count,
                u.following_count,
                CASE
                    WHEN u.username ILIKE '%' || $1 || '%' ESCAPE '\' THEN u.username
                    WHEN coalesce(u.nickname, '') ILIKE '%' || $1 || '%' ESCAPE '\' THEN coalesce(u.nickname, u.username)
                    ELSE u.username
                END AS highlight,
                u.created_at,
                (
                    ts_rank_cd(u.search_vector, plainto_tsquery('simple', $1))
                    + GREATEST(similarity(u.username, $1), similarity(coalesce(u.nickname, ''), $1), 0)
                    + CASE
                        WHEN u.username ILIKE '%' || $1 || '%' ESCAPE '\' THEN 0.4
                        WHEN coalesce(u.nickname, '') ILIKE '%' || $1 || '%' ESCAPE '\' THEN 0.25
                        ELSE 0
                      END
                )::real AS rank
            FROM users u
            JOIN roles r ON r.id = u.role_id
            WHERE u.status = 'active'
              AND (
                    u.search_vector @@ plainto_tsquery('simple', $1)
                    OR u.username ILIKE '%' || $1 || '%' ESCAPE '\'
                    OR coalesce(u.nickname, '') ILIKE '%' || $1 || '%' ESCAPE '\'
                  )
            ORDER BY {order_sql}
            LIMIT $2 OFFSET $3
            "#
        );

        let rows = sqlx::query_as::<_, UserHitRow>(&sql)
            .bind(filter.keyword)
            .bind(filter.limit)
            .bind(filter.offset)
            .fetch_all(&self.pool)
            .await?;

        Ok((rows.into_iter().map(map_user_hit).collect(), total))
    }

    pub async fn suggest_topics(
        &self,
        keyword: &str,
        limit: i64,
    ) -> Result<Vec<String>, sqlx::Error> {
        sqlx::query_scalar::<_, String>(
            r#"
            SELECT t.title
            FROM topics t
            JOIN categories c ON c.id = t.category_id
            WHERE t.status = 'published'
              AND c.is_visible = true
              AND (
                    t.search_vector @@ plainto_tsquery('simple', $1)
                    OR t.title ILIKE '%' || $1 || '%' ESCAPE '\'
                  )
            ORDER BY
                ts_rank_cd(t.search_vector, plainto_tsquery('simple', $1)) DESC,
                t.view_count DESC
            LIMIT $2
            "#,
        )
        .bind(keyword)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
    }
}

fn map_topic_hit(row: TopicHitRow) -> TopicSearchHit {
    TopicSearchHit {
        id: row.id,
        title: row.title,
        slug: row.slug,
        summary: row.summary,
        highlight: normalize_highlight(&row.highlight),
        category: CategorySummary {
            id: row.category_id,
            slug: row.category_slug,
            name: row.category_name,
            icon: row.category_icon,
            restricted_posting: false,
            allow_anonymous: false,
        },
        author: SearchAuthor {
            id: row.author_id,
            username: row.author_username,
            nickname: row.author_nickname,
            avatar: row.author_avatar,
            role: RoleSummary {
                code: row.author_role_code,
                name: row.author_role_name,
            },
        },
        stats: SearchTopicStats {
            views: row.view_count,
            replies: row.reply_count,
            likes: row.like_count,
        },
        created_at: row.created_at,
        rank: row.rank,
        has_poll: row.has_poll,
    }
}

fn map_comment_hit(row: CommentHitRow) -> CommentSearchHit {
    CommentSearchHit {
        id: row.id,
        topic_id: row.topic_id,
        topic_slug: row.topic_slug,
        topic_title: row.topic_title,
        content_preview: row.content_preview,
        highlight: normalize_highlight(&row.highlight),
        author: SearchAuthor {
            id: row.author_id,
            username: row.author_username,
            nickname: row.author_nickname,
            avatar: row.author_avatar,
            role: RoleSummary {
                code: row.author_role_code,
                name: row.author_role_name,
            },
        },
        like_count: row.like_count,
        created_at: row.created_at,
        rank: row.rank,
    }
}

fn map_user_hit(row: UserHitRow) -> UserSearchHit {
    UserSearchHit {
        id: row.id,
        username: row.username,
        nickname: row.nickname,
        avatar: row.avatar,
        role: RoleSummary {
            code: row.role_code,
            name: row.role_name,
        },
        followers_count: row.followers_count,
        following_count: row.following_count,
        highlight: row.highlight,
        created_at: row.created_at,
        rank: row.rank,
    }
}

fn normalize_highlight(value: &str) -> String {
    value.replace("<<", "").replace(">>", "")
}
