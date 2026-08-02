use chrono::{DateTime, Utc};
use ipnetwork::IpNetwork;
use serde_json::Value;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::models::{
    AdminAnalytics, AdminCommentItem, AdminDashboard, AdminDashboardRange, AdminFileItem,
    AdminLogItem, AdminTopicItem, AdminUserDetail, AdminUserItem, DailyCount, HotCategoryStat,
    HotTopicStat, LoginRecordItem, PermissionOption, PublicSettings, QueueCaseItem,
    QueueReportItem, QueueSummary, ReportItem, ReportStatus, ReportTargetType, RoleOption,
    RoleSummary, SystemSettingItem, UserStatus,
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
    is_locked: bool,
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
struct QueueReportRow {
    id: Uuid,
    reporter_username: String,
    target_type: String,
    target_id: Uuid,
    reason: String,
    status: String,
    created_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
struct QueueCaseRow {
    id: Uuid,
    target_type: String,
    target_id: Uuid,
    priority: String,
    source: String,
    opened_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
struct SystemSettingRow {
    key: String,
    value: serde_json::Value,
    description: Option<String>,
    updated_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
struct LoginRecordRow {
    id: Uuid,
    created_at: DateTime<Utc>,
    created_by_ip: Option<IpNetwork>,
    user_agent: Option<String>,
    last_used_at: Option<DateTime<Utc>>,
    revoked_at: Option<DateTime<Utc>>,
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

    pub async fn dashboard(
        &self,
        range: AdminDashboardRange,
    ) -> Result<AdminDashboard, sqlx::Error> {
        let days = range.days();
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
        let polls_total = sqlx::query_scalar::<_, i64>("SELECT count(*) FROM polls")
            .fetch_one(&self.pool)
            .await?;
        let uploads_total =
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM uploads WHERE status = 'ready'")
                .fetch_one(&self.pool)
                .await?;
        let storage_bytes = sqlx::query_scalar::<_, i64>(
            "SELECT COALESCE(sum(file_size), 0)::bigint FROM uploads WHERE status = 'ready'",
        )
        .fetch_one(&self.pool)
        .await?;
        let reports_open = sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM reports WHERE status IN ('open', 'reviewing')",
        )
        .fetch_one(&self.pool)
        .await?;
        let reports_total = sqlx::query_scalar::<_, i64>("SELECT count(*) FROM reports")
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
        let comments_today = sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM comments WHERE created_at >= date_trunc('day', now())",
        )
        .fetch_one(&self.pool)
        .await?;
        let active_users_today = sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM users WHERE last_login_at >= date_trunc('day', now())",
        )
        .fetch_one(&self.pool)
        .await?;
        let active_users_7d = sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM users WHERE last_login_at >= now() - interval '7 days'",
        )
        .fetch_one(&self.pool)
        .await?;

        let registrations = self.daily_series("users", days).await?;
        let topics = self.daily_series("topics", days).await?;
        let comments = self.daily_series("comments", days).await?;

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

        let hot_categories = sqlx::query_as::<_, (Uuid, String, String, i64, i64)>(
            r#"
            SELECT c.id, c.name, c.slug,
                   count(DISTINCT t.id) AS topic_count,
                   count(DISTINCT cm.id) AS comment_count
            FROM categories c
            LEFT JOIN topics t ON t.category_id = c.id AND t.status = 'published'
            LEFT JOIN comments cm ON cm.topic_id = t.id AND cm.status = 'published'
            WHERE c.is_visible = true
            GROUP BY c.id, c.name, c.slug
            ORDER BY topic_count DESC, comment_count DESC
            LIMIT 8
            "#,
        )
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(
            |(id, name, slug, topic_count, comment_count)| HotCategoryStat {
                id,
                name,
                slug,
                topic_count,
                comment_count,
            },
        )
        .collect();

        Ok(AdminDashboard {
            users_total,
            users_today,
            active_users_today,
            active_users_7d,
            online_users: 0, // filled by the route handler (presence)
            topics_total,
            topics_today,
            comments_total,
            comments_today,
            polls_total,
            uploads_total,
            storage_bytes,
            reports_open,
            reports_total,
            api_requests_total: 0, // filled by the route handler (metrics)
            ws_connections: 0,     // filled by the route handler (hub)
            range: range.as_str(),
            registrations,
            topics,
            comments,
            hot_topics,
            hot_categories,
        })
    }

    async fn daily_series(&self, table: &str, days: i64) -> Result<Vec<DailyCount>, sqlx::Error> {
        let sql = format!(
            r#"
            SELECT d::date, count(t.id)
            FROM generate_series(current_date - {offset}, current_date, interval '1 day') AS d
            LEFT JOIN {table} t ON t.created_at::date = d::date
            GROUP BY d
            ORDER BY d
            "#,
            offset = days - 1
        );
        sqlx::query_as::<_, (chrono::NaiveDate, i64)>(&sql)
            .fetch_all(&self.pool)
            .await
            .map(|rows| {
                rows.into_iter()
                    .map(|(date, count)| DailyCount {
                        date: date.to_string(),
                        count,
                    })
                    .collect()
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
            WHERE ($1::text IS NULL OR u.username ILIKE '%' || $1 || '%' OR u.email ILIKE '%' || $1 || '%' OR COALESCE(u.nickname, '') ILIKE '%' || $1 || '%' OR COALESCE(u.steam_id::text, '') ILIKE '%' || $1 || '%')
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
            WHERE ($1::text IS NULL OR u.username ILIKE '%' || $1 || '%' OR u.email ILIKE '%' || $1 || '%' OR COALESCE(u.nickname, '') ILIKE '%' || $1 || '%' OR COALESCE(u.steam_id::text, '') ILIKE '%' || $1 || '%')
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

    pub async fn list_permissions(&self) -> Result<Vec<PermissionOption>, sqlx::Error> {
        Ok(sqlx::query_as::<_, (String, String, Option<String>)>(
            "SELECT code, name, description FROM permissions ORDER BY code",
        )
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|(code, name, description)| PermissionOption {
            group: code.split('.').next().unwrap_or("system").to_owned(),
            code,
            name,
            description,
        })
        .collect())
    }

    pub async fn role_permissions(
        &self,
        role_code: &str,
    ) -> Result<Option<(String, Vec<String>)>, sqlx::Error> {
        let row =
            sqlx::query_as::<_, (String, String)>("SELECT code, name FROM roles WHERE code = $1")
                .bind(role_code)
                .fetch_optional(&self.pool)
                .await?;
        let Some((code, _name)) = row else {
            return Ok(None);
        };
        let permissions = sqlx::query_scalar::<_, String>(
            r#"
            SELECT p.code
            FROM role_permissions rp
            JOIN permissions p ON p.id = rp.permission_id
            WHERE rp.role_id = (SELECT id FROM roles WHERE code = $1)
            ORDER BY p.code
            "#,
        )
        .bind(role_code)
        .fetch_all(&self.pool)
        .await?;
        Ok(Some((code, permissions)))
    }

    /// Replace the permission set of a role atomically.
    pub async fn update_role_permissions(
        &self,
        role_code: &str,
        permission_codes: &[String],
    ) -> Result<bool, sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        let role_id = sqlx::query_scalar::<_, Uuid>("SELECT id FROM roles WHERE code = $1")
            .bind(role_code)
            .fetch_optional(&mut *tx)
            .await?;
        let Some(role_id) = role_id else {
            return Ok(false);
        };
        sqlx::query("DELETE FROM role_permissions WHERE role_id = $1")
            .bind(role_id)
            .execute(&mut *tx)
            .await?;
        for code in permission_codes {
            sqlx::query(
                r#"
                INSERT INTO role_permissions (role_id, permission_id)
                SELECT $1, id FROM permissions WHERE code = $2
                ON CONFLICT DO NOTHING
                "#,
            )
            .bind(role_id)
            .bind(code)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(true)
    }

    /// User ids holding a role — used to invalidate authorization caches after
    /// permission changes.
    pub async fn user_ids_by_role(&self, role_code: &str) -> Result<Vec<Uuid>, sqlx::Error> {
        sqlx::query_scalar::<_, Uuid>(
            r#"
            SELECT u.id
            FROM users u
            JOIN roles r ON r.id = u.role_id
            WHERE r.code = $1
            "#,
        )
        .bind(role_code)
        .fetch_all(&self.pool)
        .await
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

    pub async fn user_detail(&self, user_id: Uuid) -> Result<Option<AdminUserDetail>, sqlx::Error> {
        let user = match self.get_user(user_id).await? {
            Some(user) => user,
            None => return Ok(None),
        };
        let (steam_id, steam_persona_name) = sqlx::query_as::<_, (Option<String>, Option<String>)>(
            "SELECT steam_id, steam_persona_name FROM users WHERE id = $1",
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;
        let login_count =
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM refresh_tokens WHERE user_id = $1")
                .bind(user_id)
                .fetch_one(&self.pool)
                .await?;
        let topics_count = sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM topics WHERE author_id = $1 AND status <> 'deleted'",
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;
        let comments_count = sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM comments WHERE author_id = $1 AND status = 'published'",
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;
        let reports_made =
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM reports WHERE reporter_id = $1")
                .bind(user_id)
                .fetch_one(&self.pool)
                .await?;
        let sanctions_active = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT count(*) FROM user_sanctions
            WHERE user_id = $1
              AND status = 'active'
              AND (ends_at IS NULL OR ends_at > now())
            "#,
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;
        let recent_logins = self.login_records(user_id, 10, 0).await?.0;
        Ok(Some(AdminUserDetail {
            user,
            steam_id,
            steam_persona_name,
            login_count,
            topics_count,
            comments_count,
            reports_made,
            sanctions_active,
            recent_logins,
        }))
    }

    pub async fn login_records(
        &self,
        user_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<LoginRecordItem>, i64), sqlx::Error> {
        let rows = sqlx::query_as::<_, LoginRecordRow>(
            r#"
            SELECT id, created_at, created_by_ip, user_agent, last_used_at, revoked_at
            FROM refresh_tokens
            WHERE user_id = $1
            ORDER BY created_at DESC, id DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(user_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;
        let total =
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM refresh_tokens WHERE user_id = $1")
                .bind(user_id)
                .fetch_one(&self.pool)
                .await?;
        Ok((
            rows.into_iter()
                .map(|row| LoginRecordItem {
                    id: row.id,
                    created_at: row.created_at,
                    ip: row.created_by_ip.map(|ip| ip.to_string()),
                    user_agent: row.user_agent,
                    last_used_at: row.last_used_at,
                    revoked_at: row.revoked_at,
                })
                .collect(),
            total,
        ))
    }

    /// Bump auth_version and revoke all live refresh tokens (force logout).
    pub async fn force_logout(&self, user_id: Uuid) -> Result<(), sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("UPDATE users SET auth_version = auth_version + 1 WHERE id = $1")
            .bind(user_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query(
            r#"
            UPDATE refresh_tokens
            SET revoked_at = now(), revocation_reason = 'admin_force_logout'
            WHERE user_id = $1 AND revoked_at IS NULL
            "#,
        )
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn queue_summary(&self) -> Result<QueueSummary, sqlx::Error> {
        let pending_reports =
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM reports WHERE status = 'open'")
                .fetch_one(&self.pool)
                .await?;
        let reviewing_reports =
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM reports WHERE status = 'reviewing'")
                .fetch_one(&self.pool)
                .await?;
        let open_cases = sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM moderation_cases WHERE status = 'open'",
        )
        .fetch_one(&self.pool)
        .await?;
        let hidden_topics =
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM topics WHERE status = 'hidden'")
                .fetch_one(&self.pool)
                .await?;
        let hidden_comments =
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM comments WHERE status = 'hidden'")
                .fetch_one(&self.pool)
                .await?;
        let pending_uploads =
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM uploads WHERE status = 'pending'")
                .fetch_one(&self.pool)
                .await?;

        let latest_reports = sqlx::query_as::<_, QueueReportRow>(
            r#"
            SELECT r.id, u.username AS reporter_username, r.target_type, r.target_id,
                   r.reason, r.status, r.created_at
            FROM reports r
            JOIN users u ON u.id = r.reporter_id
            WHERE r.status IN ('open', 'reviewing')
            ORDER BY r.created_at DESC
            LIMIT 5
            "#,
        )
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|row| QueueReportItem {
            id: row.id,
            reporter_username: row.reporter_username,
            target_type: match row.target_type.as_str() {
                "comment" => ReportTargetType::Comment,
                "user" => ReportTargetType::User,
                _ => ReportTargetType::Topic,
            },
            target_id: row.target_id,
            reason: row.reason,
            status: match row.status.as_str() {
                "reviewing" => ReportStatus::Reviewing,
                "resolved" => ReportStatus::Resolved,
                "rejected" => ReportStatus::Rejected,
                _ => ReportStatus::Open,
            },
            created_at: row.created_at,
        })
        .collect();

        let latest_cases = sqlx::query_as::<_, QueueCaseRow>(
            r#"
            SELECT id, target_type, target_id, priority, source, opened_at
            FROM moderation_cases
            WHERE status = 'open'
            ORDER BY opened_at DESC
            LIMIT 5
            "#,
        )
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|row| QueueCaseItem {
            id: row.id,
            target_type: row.target_type,
            target_id: row.target_id,
            priority: row.priority,
            source: row.source,
            opened_at: row.opened_at,
        })
        .collect();

        Ok(QueueSummary {
            pending_reports,
            reviewing_reports,
            open_cases,
            hidden_topics,
            hidden_comments,
            pending_uploads,
            latest_reports,
            latest_cases,
        })
    }

    pub async fn analytics(&self, days: i64) -> Result<AdminAnalytics, sqlx::Error> {
        let days = days.clamp(1, 90);
        let registrations = self.daily_series("users", days).await?;
        let topics = self.daily_series("topics", days).await?;
        let comments = self.daily_series("comments", days).await?;
        let polls = self.daily_series("polls", days).await?;

        let base = self.daily_series("users", days).await?;
        let users_before = sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM users WHERE created_at < current_date - $1::int",
        )
        .bind(days - 1)
        .fetch_one(&self.pool)
        .await?;
        let mut cumulative = Vec::with_capacity(base.len());
        let mut running = users_before;
        for item in &base {
            running += item.count;
            cumulative.push(DailyCount {
                date: item.date.clone(),
                count: running,
            });
        }

        let hot_categories = sqlx::query_as::<_, (Uuid, String, String, i64, i64)>(
            r#"
            SELECT c.id, c.name, c.slug,
                   count(DISTINCT t.id) AS topic_count,
                   count(DISTINCT cm.id) AS comment_count
            FROM categories c
            LEFT JOIN topics t ON t.category_id = c.id AND t.status = 'published'
            LEFT JOIN comments cm ON cm.topic_id = t.id AND cm.status = 'published'
            WHERE c.is_visible = true
            GROUP BY c.id, c.name, c.slug
            ORDER BY topic_count DESC, comment_count DESC
            LIMIT 10
            "#,
        )
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(
            |(id, name, slug, topic_count, comment_count)| HotCategoryStat {
                id,
                name,
                slug,
                topic_count,
                comment_count,
            },
        )
        .collect();

        let hot_topics = sqlx::query_as::<_, (Uuid, String, String, i64, i64, i64)>(
            r#"
            SELECT id, title, slug, view_count, reply_count, like_count
            FROM topics
            WHERE status = 'published'
            ORDER BY (view_count + like_count * 10 + reply_count * 5) DESC
            LIMIT 10
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

        Ok(AdminAnalytics {
            days,
            registrations,
            topics,
            comments,
            polls,
            cumulative_users: cumulative,
            hot_categories,
            hot_topics,
        })
    }

    pub async fn list_settings(&self) -> Result<Vec<SystemSettingItem>, sqlx::Error> {
        sqlx::query_as::<_, SystemSettingRow>(
            r#"
            SELECT key, value, description, updated_at
            FROM system_settings
            ORDER BY key
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map(|rows| {
            rows.into_iter()
                .map(|row| SystemSettingItem {
                    key: row.key,
                    value: row.value,
                    description: row.description,
                    updated_at: row.updated_at,
                })
                .collect()
        })
    }

    pub async fn upsert_settings(
        &self,
        actor_id: Uuid,
        settings: &[(String, serde_json::Value)],
    ) -> Result<(), sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        for (key, value) in settings {
            sqlx::query(
                r#"
                INSERT INTO system_settings (key, value, updated_by, updated_at)
                VALUES ($1, $2, $3, now())
                ON CONFLICT (key)
                DO UPDATE SET value = EXCLUDED.value, updated_by = EXCLUDED.updated_by, updated_at = now()
                "#,
            )
            .bind(key)
            .bind(value)
            .bind(actor_id)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    /// Public settings snapshot (Redis-cached by the caller).
    pub async fn public_settings(&self) -> Result<PublicSettings, sqlx::Error> {
        let rows: Vec<(String, serde_json::Value)> =
            sqlx::query_as::<_, (String, serde_json::Value)>(
                r#"
            SELECT key, value FROM system_settings
            WHERE key IN ('site_name', 'site_description', 'registration_enabled',
                          'topic_create_enabled', 'comment_enabled', 'upload_enabled',
                          'upload_max_bytes')
            "#,
            )
            .fetch_all(&self.pool)
            .await?;
        let get = |key: &str| -> serde_json::Value {
            rows.iter()
                .find(|(k, _)| k == key)
                .map(|(_, v)| v.clone())
                .unwrap_or_else(|| serde_json::Value::Null)
        };
        Ok(PublicSettings {
            site_name: get("site_name").as_str().unwrap_or("LumiForum").to_owned(),
            site_description: get("site_description")
                .as_str()
                .map(|value| value.to_owned()),
            registration_enabled: get("registration_enabled").as_bool().unwrap_or(true),
            topic_create_enabled: get("topic_create_enabled").as_bool().unwrap_or(true),
            comment_enabled: get("comment_enabled").as_bool().unwrap_or(true),
            upload_enabled: get("upload_enabled").as_bool().unwrap_or(true),
            upload_max_bytes: get("upload_max_bytes").as_i64().unwrap_or(10 * 1024 * 1024),
        })
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
        sort: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<AdminTopicItem>, i64), sqlx::Error> {
        let order_by = match sort {
            Some("hot") => "t.view_count DESC, t.reply_count DESC, t.created_at DESC",
            Some("most_reported") => "report_counts.reports DESC, t.created_at DESC",
            Some("violating") => "t.updated_at DESC",
            _ => "t.created_at DESC",
        };
        let report_join = match sort {
            Some("most_reported") => {
                r#"LEFT JOIN (
                       SELECT target_id, count(*) AS reports
                       FROM reports
                       WHERE target_type = 'topic' AND status IN ('open', 'resolved')
                       GROUP BY target_id
                   ) report_counts ON report_counts.target_id = t.id"#
            }
            Some("violating") => {
                r#"JOIN reports violation_report ON violation_report.target_id = t.id
                       AND violation_report.target_type = 'topic'
                       AND violation_report.status = 'resolved'"#
            }
            _ => "",
        };

        let rows = sqlx::query_as::<_, TopicRow>(
            &format!(
            r#"
            SELECT t.id, t.title, t.slug, t.status, t.summary,
                   c.id AS category_id, c.name AS category_name, c.slug AS category_slug,
                   u.id AS author_id, u.username AS author_username,
                   t.view_count, t.reply_count, t.like_count, t.is_pinned, t.is_featured,
                   t.is_locked, t.deleted_at, t.created_at, t.updated_at
            FROM topics t
            JOIN categories c ON c.id = t.category_id
            JOIN users u ON u.id = t.author_id
            {report_join}
            WHERE ($1::text IS NULL OR t.title ILIKE '%' || $1 || '%' OR t.slug ILIKE '%' || $1 || '%')
              AND ($2::text IS NULL OR t.status = $2)
              AND ($3::uuid IS NULL OR t.category_id = $3)
            ORDER BY {order_by}
            LIMIT $4 OFFSET $5
            "#,
            ),
        )
        .bind(q)
        .bind(status)
        .bind(category_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let total = if matches!(sort, Some("most_reported") | Some("violating")) {
            sqlx::query_scalar::<_, i64>(
                r#"
                SELECT count(DISTINCT t.id)
                FROM topics t
                JOIN reports violation_filter ON violation_filter.target_id = t.id
                    AND violation_filter.target_type = 'topic'
                WHERE ($1::text IS NULL OR t.title ILIKE '%' || $1 || '%' OR t.slug ILIKE '%' || $1 || '%')
                  AND ($2::text IS NULL OR t.status = $2)
                  AND ($3::uuid IS NULL OR t.category_id = $3)
                "#,
            )
            .bind(q)
            .bind(status)
            .bind(category_id)
            .fetch_one(&self.pool)
            .await?
        } else {
            sqlx::query_scalar::<_, i64>(
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
            .await?
        };

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
        is_locked: Option<bool>,
    ) -> Result<Option<AdminTopicItem>, sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE topics
            SET is_pinned = COALESCE($2, is_pinned),
                is_featured = COALESCE($3, is_featured),
                is_locked = COALESCE($4, is_locked)
            WHERE id = $1 AND status <> 'deleted'
            "#,
        )
        .bind(topic_id)
        .bind(is_pinned)
        .bind(is_featured)
        .bind(is_locked)
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
                   t.is_locked, t.deleted_at, t.created_at, t.updated_at
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
        filter: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<AdminCommentItem>, i64), sqlx::Error> {
        let filter_clause = match filter {
            Some("reported") => {
                "AND EXISTS (SELECT 1 FROM reports r WHERE r.target_type = 'comment' AND r.target_id = c.id AND r.status IN ('open', 'reviewing'))"
            }
            Some("high_frequency") => "AND c.reply_count >= 5",
            _ => "",
        };

        let rows = sqlx::query_as::<_, CommentRow>(&format!(
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
              {filter_clause}
            ORDER BY c.created_at DESC
            LIMIT $4 OFFSET $5
            "#,
        ))
        .bind(q)
        .bind(status)
        .bind(topic_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let total = sqlx::query_scalar::<_, i64>(&format!(
            r#"
            SELECT count(*)
            FROM comments c
            WHERE ($1::text IS NULL OR c.content ILIKE '%' || $1 || '%')
              AND ($2::text IS NULL OR c.status = $2)
              AND ($3::uuid IS NULL OR c.topic_id = $3)
              {filter_clause}
            "#,
        ))
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
        target_type: Option<&str>,
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
              AND ($3::text IS NULL OR l.target_type = $3)
            ORDER BY l.created_at DESC
            LIMIT $4 OFFSET $5
            "#,
        )
        .bind(q)
        .bind(action)
        .bind(target_type)
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
              AND ($3::text IS NULL OR l.target_type = $3)
            "#,
        )
        .bind(q)
        .bind(action)
        .bind(target_type)
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
        is_locked: row.is_locked,
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
