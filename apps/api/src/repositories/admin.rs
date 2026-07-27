use chrono::{DateTime, Utc};
use ipnetwork::IpNetwork;
use serde_json::Value;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::models::{
    AdminCommentItem, AdminDashboard, AdminFileItem, AdminLogItem, AdminTopicItem, AdminUserItem,
    DailyCount, HotTopicStat, ReportItem, ReportTargetType, RoleOption, RoleSummary, UserStatus,
};

#[derive(Clone)]
pub struct AdminRepository {
    pool: PgPool,
}

#[derive(sqlx::FromRow)]
struct UserRow {
    id: Uuid,
    username: String,
    email: String,
    avatar: Option<String>,
    nickname: Option<String>,
    role_code: String,
    role_name: String,
    status: String,
    email_verified: bool,
    followers_count: i64,
    following_count: i64,
    last_login_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
struct TopicRow {
    id: Uuid,
    title: String,
    slug: String,
    status: String,
    summary: Option<String>,
    category_id: Uuid,
    category_name: String,
    category_slug: String,
    author_id: Uuid,
    author_username: String,
    view_count: i64,
    reply_count: i64,
    like_count: i64,
    is_pinned: bool,
    is_featured: bool,
    deleted_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
struct CommentRow {
    id: Uuid,
    topic_id: Uuid,
    topic_title: String,
    topic_slug: String,
    parent_id: Option<Uuid>,
    content: String,
    status: String,
    author_id: Uuid,
    author_username: String,
    like_count: i64,
    reply_count: i64,
    deleted_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
struct FileRow {
    id: Uuid,
    user_id: Uuid,
    username: String,
    filename: String,
    original_filename: String,
    mime_type: String,
    file_size: i64,
    category: String,
    url: Option<String>,
    status: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
struct ReportRow {
    id: Uuid,
    reporter_id: Uuid,
    reporter_username: String,
    target_type: String,
    target_id: Uuid,
    reason: String,
    details: Option<String>,
    status: String,
    handler_id: Option<Uuid>,
    handler_username: Option<String>,
    resolution_note: Option<String>,
    handled_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
struct LogRow {
    id: Uuid,
    admin_id: Uuid,
    admin_username: String,
    action: String,
    target_type: String,
    target_id: Option<Uuid>,
    summary: String,
    metadata: Value,
    ip_address: Option<IpNetwork>,
    user_agent: Option<String>,
    created_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
pub struct ManagedUser {
    pub id: Uuid,
    pub username: String,
    pub status: String,
    pub role_code: String,
    pub role_priority: i16,
    pub auth_version: i32,
}

impl AdminRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    pub async fn dashboard(&self) -> Result<AdminDashboard, sqlx::Error> {
        let users_total = sqlx::query_scalar::<_, i64>("SELECT count(*) FROM users")
            .fetch_one(&self.pool)
            .await?;
        let topics_total =
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM topics WHERE status <> 'deleted'")
                .fetch_one(&self.pool)
                .await?;
        let comments_total = sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM comments WHERE status = 'published'",
        )
        .fetch_one(&self.pool)
        .await?;
        let uploads_total =
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM uploads WHERE status = 'ready'")
                .fetch_one(&self.pool)
                .await?;
        let reports_open = sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM reports WHERE status IN ('open', 'reviewing')",
        )
        .fetch_one(&self.pool)
        .await?;
        let users_today = sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM users WHERE created_at >= date_trunc('day', now())",
        )
        .fetch_one(&self.pool)
        .await?;
        let topics_today = sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM topics WHERE created_at >= date_trunc('day', now())",
        )
        .fetch_one(&self.pool)
        .await?;
        let active_users_7d = sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM users WHERE last_login_at >= now() - interval '7 days'",
        )
        .fetch_one(&self.pool)
        .await?;

        let registrations_7d = sqlx::query_as::<_, (chrono::NaiveDate, i64)>(
            r#"
            SELECT d::date, count(users.id)
            FROM generate_series(current_date - 6, current_date, interval '1 day') AS d
            LEFT JOIN users ON users.created_at::date = d::date
            GROUP BY d
            ORDER BY d
            "#,
        )
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|(date, count)| DailyCount {
            date: date.to_string(),
            count,
        })
        .collect();

        let topics_7d = sqlx::query_as::<_, (chrono::NaiveDate, i64)>(
            r#"
            SELECT d::date, count(topics.id)
            FROM generate_series(current_date - 6, current_date, interval '1 day') AS d
            LEFT JOIN topics ON topics.created_at::date = d::date
            GROUP BY d
            ORDER BY d
            "#,
        )
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|(date, count)| DailyCount {
            date: date.to_string(),
            count,
        })
        .collect();

        let hot_topics = sqlx::query_as::<_, (Uuid, String, String, i64, i64, i64)>(
            r#"
            SELECT id, title, slug, view_count, reply_count, like_count
            FROM topics
            WHERE status = 'published'
            ORDER BY view_count DESC, like_count DESC, created_at DESC
            LIMIT 8
            "#,
        )
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(
            |(id, title, slug, view_count, reply_count, like_count)| HotTopicStat {
                id,
                title,
                slug,
                view_count,
                reply_count,
                like_count,
            },
        )
        .collect();

        Ok(AdminDashboard {
            users_total,
            topics_total,
            comments_total,
            uploads_total,
            reports_open,
            users_today,
            topics_today,
            active_users_7d,
            registrations_7d,
            topics_7d,
            hot_topics,
        })
    }

    pub async fn list_users(
        &self,
        q: Option<&str>,
        status: Option<&str>,
        role: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<AdminUserItem>, i64), sqlx::Error> {
        let rows = sqlx::query_as::<_, UserRow>(
            r#"
            SELECT u.id, u.username, u.email, u.avatar_url AS avatar, u.nickname,
                   r.code AS role_code, r.name AS role_name,
                   u.status, u.email_verified, u.followers_count, u.following_count,
                   u.last_login_at, u.created_at, u.updated_at
            FROM users u
            JOIN roles r ON r.id = u.role_id
            WHERE ($1::text IS NULL OR u.username ILIKE '%' || $1 || '%' OR u.email ILIKE '%' || $1 || '%' OR COALESCE(u.nickname, '') ILIKE '%' || $1 || '%')
              AND ($2::text IS NULL OR u.status = $2)
              AND ($3::text IS NULL OR r.code = $3)
            ORDER BY u.created_at DESC
            LIMIT $4 OFFSET $5
            "#,
        )
        .bind(q)
        .bind(status)
        .bind(role)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let total = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT count(*)
            FROM users u
            JOIN roles r ON r.id = u.role_id
            WHERE ($1::text IS NULL OR u.username ILIKE '%' || $1 || '%' OR u.email ILIKE '%' || $1 || '%' OR COALESCE(u.nickname, '') ILIKE '%' || $1 || '%')
              AND ($2::text IS NULL OR u.status = $2)
              AND ($3::text IS NULL OR r.code = $3)
            "#,
        )
        .bind(q)
        .bind(status)
        .bind(role)
        .fetch_one(&self.pool)
        .await?;

        Ok((rows.into_iter().map(map_user).collect(), total))
    }

    pub async fn get_user(&self, user_id: Uuid) -> Result<Option<AdminUserItem>, sqlx::Error> {
        let row = sqlx::query_as::<_, UserRow>(
            r#"
            SELECT u.id, u.username, u.email, u.avatar_url AS avatar, u.nickname,
                   r.code AS role_code, r.name AS role_name,
                   u.status, u.email_verified, u.followers_count, u.following_count,
                   u.last_login_at, u.created_at, u.updated_at
            FROM users u
            JOIN roles r ON r.id = u.role_id
            WHERE u.id = $1
            "#,
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(map_user))
    }

    pub async fn list_roles(&self) -> Result<Vec<RoleOption>, sqlx::Error> {
        Ok(sqlx::query_as::<_, (String, String, i16)>(
            "SELECT code, name, priority FROM roles ORDER BY priority ASC, code ASC",
        )
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|(code, name, priority)| RoleOption {
            code,
            name,
            priority,
        })
        .collect())
    }

    pub async fn lock_user(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        user_id: Uuid,
    ) -> Result<Option<ManagedUser>, sqlx::Error> {
        sqlx::query_as::<_, ManagedUser>(
            r#"
            SELECT u.id, u.username, u.status, r.code AS role_code, r.priority AS role_priority, u.auth_version
            FROM users u
            JOIN roles r ON r.id = u.role_id
            WHERE u.id = $1
            FOR UPDATE OF u
            "#,
        )
        .bind(user_id)
        .fetch_optional(&mut **tx)
        .await
    }

    pub async fn role_priority(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        role_code: &str,
    ) -> Result<Option<(Uuid, i16)>, sqlx::Error> {
        sqlx::query_as::<_, (Uuid, i16)>("SELECT id, priority FROM roles WHERE code = $1")
            .bind(role_code)
            .fetch_optional(&mut **tx)
            .await
    }

    pub async fn count_super_admins(
        &self,
        tx: &mut Transaction<'_, Postgres>,
    ) -> Result<i64, sqlx::Error> {
        sqlx::query_scalar::<_, i64>(
            r#"
            SELECT count(*)
            FROM users u
            JOIN roles r ON r.id = u.role_id
            WHERE r.code = 'super_administrator' AND u.status = 'active'
            "#,
        )
        .fetch_one(&mut **tx)
        .await
    }

    pub async fn update_user(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        user_id: Uuid,
        status: Option<&str>,
        role_id: Option<Uuid>,
        bump_auth: bool,
    ) -> Result<Option<AdminUserItem>, sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE users
            SET status = COALESCE($2, status),
                role_id = COALESCE($3, role_id),
                auth_version = CASE WHEN $4 THEN auth_version + 1 ELSE auth_version END
            WHERE id = $1
            "#,
        )
        .bind(user_id)
        .bind(status)
        .bind(role_id)
        .bind(bump_auth)
        .execute(&mut **tx)
        .await?;

        sqlx::query_as::<_, UserRow>(
            r#"
            SELECT u.id, u.username, u.email, u.avatar_url AS avatar, u.nickname,
                   r.code AS role_code, r.name AS role_name,
                   u.status, u.email_verified, u.followers_count, u.following_count,
                   u.last_login_at, u.created_at, u.updated_at
            FROM users u
            JOIN roles r ON r.id = u.role_id
            WHERE u.id = $1
            "#,
        )
        .bind(user_id)
        .fetch_optional(&mut **tx)
        .await
        .map(|row| row.map(map_user))
    }

    pub async fn revoke_refresh_tokens(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        user_id: Uuid,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE refresh_tokens
            SET revoked_at = now(),
                revocation_reason = 'admin_action'
            WHERE user_id = $1 AND revoked_at IS NULL
            "#,
        )
        .bind(user_id)
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub async fn touch_last_login(&self, user_id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE users SET last_login_at = now() WHERE id = $1")
            .bind(user_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn list_topics(
        &self,
        q: Option<&str>,
        status: Option<&str>,
        category_id: Option<Uuid>,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<AdminTopicItem>, i64), sqlx::Error> {
        let rows = sqlx::query_as::<_, TopicRow>(
            r#"
            SELECT t.id, t.title, t.slug, t.status, t.summary,
                   c.id AS category_id, c.name AS category_name, c.slug AS category_slug,
                   u.id AS author_id, u.username AS author_username,
                   t.view_count, t.reply_count, t.like_count, t.is_pinned, t.is_featured,
                   t.deleted_at, t.created_at, t.updated_at
            FROM topics t
            JOIN categories c ON c.id = t.category_id
            JOIN users u ON u.id = t.author_id
            WHERE ($1::text IS NULL OR t.title ILIKE '%' || $1 || '%' OR t.slug ILIKE '%' || $1 || '%')
              AND ($2::text IS NULL OR t.status = $2)
              AND ($3::uuid IS NULL OR t.category_id = $3)
            ORDER BY t.created_at DESC
            LIMIT $4 OFFSET $5
            "#,
        )
        .bind(q)
        .bind(status)
        .bind(category_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let total = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT count(*)
            FROM topics t
            WHERE ($1::text IS NULL OR t.title ILIKE '%' || $1 || '%' OR t.slug ILIKE '%' || $1 || '%')
              AND ($2::text IS NULL OR t.status = $2)
              AND ($3::uuid IS NULL OR t.category_id = $3)
            "#,
        )
        .bind(q)
        .bind(status)
        .bind(category_id)
        .fetch_one(&self.pool)
        .await?;

        Ok((rows.into_iter().map(map_topic).collect(), total))
    }

    pub async fn set_topic_status(
        &self,
        topic_id: Uuid,
        status: &str,
    ) -> Result<Option<AdminTopicItem>, sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE topics
            SET status = $2,
                deleted_at = CASE WHEN $2 = 'deleted' THEN now() ELSE NULL END,
                is_pinned = CASE WHEN $2 = 'deleted' THEN false ELSE is_pinned END,
                is_featured = CASE WHEN $2 = 'deleted' THEN false ELSE is_featured END
            WHERE id = $1
            "#,
        )
        .bind(topic_id)
        .bind(status)
        .execute(&self.pool)
        .await?;
        self.get_topic(topic_id).await
    }

    pub async fn set_topic_flags(
        &self,
        topic_id: Uuid,
        is_pinned: Option<bool>,
        is_featured: Option<bool>,
    ) -> Result<Option<AdminTopicItem>, sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE topics
            SET is_pinned = COALESCE($2, is_pinned),
                is_featured = COALESCE($3, is_featured)
            WHERE id = $1 AND status <> 'deleted'
            "#,
        )
        .bind(topic_id)
        .bind(is_pinned)
        .bind(is_featured)
        .execute(&self.pool)
        .await?;
        self.get_topic(topic_id).await
    }

    pub async fn get_topic(&self, topic_id: Uuid) -> Result<Option<AdminTopicItem>, sqlx::Error> {
        let row = sqlx::query_as::<_, TopicRow>(
            r#"
            SELECT t.id, t.title, t.slug, t.status, t.summary,
                   c.id AS category_id, c.name AS category_name, c.slug AS category_slug,
                   u.id AS author_id, u.username AS author_username,
                   t.view_count, t.reply_count, t.like_count, t.is_pinned, t.is_featured,
                   t.deleted_at, t.created_at, t.updated_at
            FROM topics t
            JOIN categories c ON c.id = t.category_id
            JOIN users u ON u.id = t.author_id
            WHERE t.id = $1
            "#,
        )
        .bind(topic_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(map_topic))
    }

    pub async fn get_comment(
        &self,
        comment_id: Uuid,
    ) -> Result<Option<AdminCommentItem>, sqlx::Error> {
        let row = sqlx::query_as::<_, CommentRow>(
            r#"
            SELECT c.id, c.topic_id, t.title AS topic_title, t.slug AS topic_slug, c.parent_id,
                   c.content, c.status, u.id AS author_id, u.username AS author_username,
                   c.like_count, c.reply_count, c.deleted_at, c.created_at, c.updated_at
            FROM comments c
            JOIN topics t ON t.id = c.topic_id
            JOIN users u ON u.id = c.author_id
            WHERE c.id = $1
            "#,
        )
        .bind(comment_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(map_comment))
    }

    pub async fn list_comments(
        &self,
        q: Option<&str>,
        status: Option<&str>,
        topic_id: Option<Uuid>,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<AdminCommentItem>, i64), sqlx::Error> {
        let rows = sqlx::query_as::<_, CommentRow>(
            r#"
            SELECT c.id, c.topic_id, t.title AS topic_title, t.slug AS topic_slug, c.parent_id,
                   c.content, c.status, u.id AS author_id, u.username AS author_username,
                   c.like_count, c.reply_count, c.deleted_at, c.created_at, c.updated_at
            FROM comments c
            JOIN topics t ON t.id = c.topic_id
            JOIN users u ON u.id = c.author_id
            WHERE ($1::text IS NULL OR c.content ILIKE '%' || $1 || '%')
              AND ($2::text IS NULL OR c.status = $2)
              AND ($3::uuid IS NULL OR c.topic_id = $3)
            ORDER BY c.created_at DESC
            LIMIT $4 OFFSET $5
            "#,
        )
        .bind(q)
        .bind(status)
        .bind(topic_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let total = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT count(*)
            FROM comments c
            WHERE ($1::text IS NULL OR c.content ILIKE '%' || $1 || '%')
              AND ($2::text IS NULL OR c.status = $2)
              AND ($3::uuid IS NULL OR c.topic_id = $3)
            "#,
        )
        .bind(q)
        .bind(status)
        .bind(topic_id)
        .fetch_one(&self.pool)
        .await?;

        Ok((rows.into_iter().map(map_comment).collect(), total))
    }

    pub async fn list_files(
        &self,
        q: Option<&str>,
        category: Option<&str>,
        status: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<AdminFileItem>, i64), sqlx::Error> {
        let rows = sqlx::query_as::<_, FileRow>(
            r#"
            SELECT up.id, up.user_id, u.username, up.filename, up.original_filename, up.mime_type,
                   up.file_size, up.category, up.url, up.status, up.created_at, up.updated_at
            FROM uploads up
            JOIN users u ON u.id = up.user_id
            WHERE ($1::text IS NULL OR up.original_filename ILIKE '%' || $1 || '%' OR up.filename ILIKE '%' || $1 || '%')
              AND ($2::text IS NULL OR up.category = $2)
              AND ($3::text IS NULL OR up.status = $3)
            ORDER BY up.created_at DESC
            LIMIT $4 OFFSET $5
            "#,
        )
        .bind(q)
        .bind(category)
        .bind(status)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let total = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT count(*)
            FROM uploads up
            WHERE ($1::text IS NULL OR up.original_filename ILIKE '%' || $1 || '%' OR up.filename ILIKE '%' || $1 || '%')
              AND ($2::text IS NULL OR up.category = $2)
              AND ($3::text IS NULL OR up.status = $3)
            "#,
        )
        .bind(q)
        .bind(category)
        .bind(status)
        .fetch_one(&self.pool)
        .await?;

        Ok((
            rows.into_iter()
                .filter_map(|row| map_file(row).ok())
                .collect(),
            total,
        ))
    }

    pub async fn list_orphan_file_ids(&self, limit: i64) -> Result<Vec<Uuid>, sqlx::Error> {
        sqlx::query_scalar::<_, Uuid>(
            r#"
            SELECT id FROM uploads
            WHERE status IN ('pending', 'failed')
              AND updated_at < now() - interval '24 hours'
            ORDER BY updated_at ASC
            LIMIT $1
            "#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn create_report(
        &self,
        reporter_id: Uuid,
        target_type: &str,
        target_id: Uuid,
        reason: &str,
        details: Option<&str>,
    ) -> Result<ReportItem, sqlx::Error> {
        let id = sqlx::query_scalar::<_, Uuid>(
            r#"
            INSERT INTO reports (reporter_id, target_type, target_id, reason, details)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id
            "#,
        )
        .bind(reporter_id)
        .bind(target_type)
        .bind(target_id)
        .bind(reason)
        .bind(details)
        .fetch_one(&self.pool)
        .await?;
        self.get_report(id).await?.ok_or(sqlx::Error::RowNotFound)
    }

    pub async fn list_reports(
        &self,
        status: Option<&str>,
        target_type: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<ReportItem>, i64), sqlx::Error> {
        let rows = sqlx::query_as::<_, ReportRow>(
            r#"
            SELECT r.id, r.reporter_id, ru.username AS reporter_username,
                   r.target_type, r.target_id, r.reason, r.details, r.status,
                   r.handler_id, hu.username AS handler_username, r.resolution_note,
                   r.handled_at, r.created_at, r.updated_at
            FROM reports r
            JOIN users ru ON ru.id = r.reporter_id
            LEFT JOIN users hu ON hu.id = r.handler_id
            WHERE ($1::text IS NULL OR r.status = $1)
              AND ($2::text IS NULL OR r.target_type = $2)
            ORDER BY r.created_at DESC
            LIMIT $3 OFFSET $4
            "#,
        )
        .bind(status)
        .bind(target_type)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let total = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT count(*) FROM reports r
            WHERE ($1::text IS NULL OR r.status = $1)
              AND ($2::text IS NULL OR r.target_type = $2)
            "#,
        )
        .bind(status)
        .bind(target_type)
        .fetch_one(&self.pool)
        .await?;

        Ok((
            rows.into_iter()
                .filter_map(|row| map_report(row).ok())
                .collect(),
            total,
        ))
    }

    pub async fn get_report(&self, report_id: Uuid) -> Result<Option<ReportItem>, sqlx::Error> {
        let row = sqlx::query_as::<_, ReportRow>(
            r#"
            SELECT r.id, r.reporter_id, ru.username AS reporter_username,
                   r.target_type, r.target_id, r.reason, r.details, r.status,
                   r.handler_id, hu.username AS handler_username, r.resolution_note,
                   r.handled_at, r.created_at, r.updated_at
            FROM reports r
            JOIN users ru ON ru.id = r.reporter_id
            LEFT JOIN users hu ON hu.id = r.handler_id
            WHERE r.id = $1
            "#,
        )
        .bind(report_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.and_then(|row| map_report(row).ok()))
    }

    pub async fn resolve_report(
        &self,
        report_id: Uuid,
        handler_id: Uuid,
        status: &str,
        note: Option<&str>,
    ) -> Result<Option<ReportItem>, sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE reports
            SET status = $2,
                handler_id = $3,
                resolution_note = $4,
                handled_at = CASE WHEN $2 IN ('resolved', 'rejected') THEN now() ELSE NULL END
            WHERE id = $1
            "#,
        )
        .bind(report_id)
        .bind(status)
        .bind(handler_id)
        .bind(note)
        .execute(&self.pool)
        .await?;
        self.get_report(report_id).await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn insert_log(
        &self,
        tx: Option<&mut Transaction<'_, Postgres>>,
        admin_id: Uuid,
        action: &str,
        target_type: &str,
        target_id: Option<Uuid>,
        summary: &str,
        metadata: Value,
        ip: Option<IpNetwork>,
        user_agent: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        let query = r#"
            INSERT INTO admin_logs (admin_id, action, target_type, target_id, summary, metadata, ip_address, user_agent)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        "#;
        match tx {
            Some(tx) => {
                sqlx::query(query)
                    .bind(admin_id)
                    .bind(action)
                    .bind(target_type)
                    .bind(target_id)
                    .bind(summary)
                    .bind(metadata)
                    .bind(ip)
                    .bind(user_agent)
                    .execute(&mut **tx)
                    .await?;
            }
            None => {
                sqlx::query(query)
                    .bind(admin_id)
                    .bind(action)
                    .bind(target_type)
                    .bind(target_id)
                    .bind(summary)
                    .bind(metadata)
                    .bind(ip)
                    .bind(user_agent)
                    .execute(&self.pool)
                    .await?;
            }
        }
        Ok(())
    }

    pub async fn list_logs(
        &self,
        q: Option<&str>,
        action: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<AdminLogItem>, i64), sqlx::Error> {
        let rows = sqlx::query_as::<_, LogRow>(
            r#"
            SELECT l.id, l.admin_id, u.username AS admin_username, l.action, l.target_type,
                   l.target_id, l.summary, l.metadata, l.ip_address, l.user_agent, l.created_at
            FROM admin_logs l
            JOIN users u ON u.id = l.admin_id
            WHERE ($1::text IS NULL OR l.summary ILIKE '%' || $1 || '%' OR u.username ILIKE '%' || $1 || '%')
              AND ($2::text IS NULL OR l.action = $2)
            ORDER BY l.created_at DESC
            LIMIT $3 OFFSET $4
            "#,
        )
        .bind(q)
        .bind(action)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let total = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT count(*)
            FROM admin_logs l
            JOIN users u ON u.id = l.admin_id
            WHERE ($1::text IS NULL OR l.summary ILIKE '%' || $1 || '%' OR u.username ILIKE '%' || $1 || '%')
              AND ($2::text IS NULL OR l.action = $2)
            "#,
        )
        .bind(q)
        .bind(action)
        .fetch_one(&self.pool)
        .await?;

        Ok((rows.into_iter().map(map_log).collect(), total))
    }
}

fn map_user(row: UserRow) -> AdminUserItem {
    AdminUserItem {
        id: row.id,
        username: row.username,
        email: row.email,
        avatar: row.avatar,
        nickname: row.nickname,
        role: RoleSummary {
            code: row.role_code,
            name: row.role_name,
        },
        status: row.status.parse().unwrap_or(UserStatus::Disabled),
        email_verified: row.email_verified,
        followers_count: row.followers_count,
        following_count: row.following_count,
        last_login_at: row.last_login_at,
        created_at: row.created_at,
        updated_at: row.updated_at,
    }
}

fn map_topic(row: TopicRow) -> AdminTopicItem {
    AdminTopicItem {
        id: row.id,
        title: row.title,
        slug: row.slug,
        status: row.status,
        summary: row.summary,
        category_id: row.category_id,
        category_name: row.category_name,
        category_slug: row.category_slug,
        author_id: row.author_id,
        author_username: row.author_username,
        view_count: row.view_count,
        reply_count: row.reply_count,
        like_count: row.like_count,
        is_pinned: row.is_pinned,
        is_featured: row.is_featured,
        deleted_at: row.deleted_at,
        created_at: row.created_at,
        updated_at: row.updated_at,
    }
}

fn map_comment(row: CommentRow) -> AdminCommentItem {
    AdminCommentItem {
        id: row.id,
        topic_id: row.topic_id,
        topic_title: row.topic_title,
        topic_slug: row.topic_slug,
        parent_id: row.parent_id,
        content: row.content,
        status: row.status,
        author_id: row.author_id,
        author_username: row.author_username,
        like_count: row.like_count,
        reply_count: row.reply_count,
        deleted_at: row.deleted_at,
        created_at: row.created_at,
        updated_at: row.updated_at,
    }
}

fn map_file(row: FileRow) -> Result<AdminFileItem, &'static str> {
    Ok(AdminFileItem {
        id: row.id,
        user_id: row.user_id,
        username: row.username,
        filename: row.filename,
        original_filename: row.original_filename,
        mime_type: row.mime_type,
        file_size: row.file_size,
        category: row
            .category
            .parse()
            .map_err(|_| "invalid upload category")?,
        url: row.url,
        status: row.status,
        created_at: row.created_at,
        updated_at: row.updated_at,
    })
}

fn map_report(row: ReportRow) -> Result<ReportItem, &'static str> {
    let target_type = match row.target_type.as_str() {
        "topic" => ReportTargetType::Topic,
        "comment" => ReportTargetType::Comment,
        "user" => ReportTargetType::User,
        _ => return Err("invalid report target"),
    };
    Ok(ReportItem {
        id: row.id,
        reporter_id: row.reporter_id,
        reporter_username: row.reporter_username,
        target_type,
        target_id: row.target_id,
        reason: row.reason,
        details: row.details,
        status: row.status.parse()?,
        handler_id: row.handler_id,
        handler_username: row.handler_username,
        resolution_note: row.resolution_note,
        handled_at: row.handled_at,
        created_at: row.created_at,
        updated_at: row.updated_at,
    })
}

fn map_log(row: LogRow) -> AdminLogItem {
    AdminLogItem {
        id: row.id,
        admin_id: row.admin_id,
        admin_username: row.admin_username,
        action: row.action,
        target_type: row.target_type,
        target_id: row.target_id,
        summary: row.summary,
        metadata: row.metadata,
        ip_address: row.ip_address.map(|ip| ip.to_string()),
        user_agent: row.user_agent,
        created_at: row.created_at,
    }
}
