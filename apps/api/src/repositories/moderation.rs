//! Phase 13: moderation repository (SQL access for reports, cases, actions,
//! snapshots, sanctions, appeals, rules, metrics).

use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::models::{
    AppealItem, AppealStatus, AppealType, CaseItem, CaseSource, CaseStatus, CountItem,
    DailyMetric, GovernanceMetrics, ModerationActionItem, ModerationNoteItem, ReportItemV2,
    ReportPriority, ReportStatus, ReportTargetType, RuleAction, RuleHitItem,
    RuleItem, RuleType, SanctionItem, SanctionStatus, SanctionType,
};

#[derive(Clone)]
pub struct ModerationRepository {
    pool: PgPool,
}

#[derive(sqlx::FromRow)]
pub struct ModTopicRow {
    pub id: Uuid,
    pub category_id: Uuid,
    pub author_id: Uuid,
    pub author_username: String,
    pub title: String,
    pub slug: String,
    pub content: String,
    pub summary: Option<String>,
    pub status: String,
    pub is_pinned: bool,
    pub is_featured: bool,
    pub is_locked: bool,
    pub is_sensitive: bool,
    pub restrict_interactions: bool,
    pub deleted_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
pub struct ModCommentRow {
    pub id: Uuid,
    pub topic_id: Uuid,
    pub topic_slug: String,
    pub topic_title: String,
    pub parent_id: Option<Uuid>,
    pub author_id: Uuid,
    pub author_username: String,
    pub content: String,
    pub status: String,
    pub is_collapsed: bool,
    pub is_sensitive: bool,
    pub replies_locked: bool,
    pub deleted_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
pub struct ModUserRow {
    pub id: Uuid,
    pub username: String,
    pub role_code: String,
    pub role_priority: i16,
    pub status: String,
    pub auth_version: i32,
    pub created_at: DateTime<Utc>,
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
    priority: String,
    risk_score: i32,
    case_id: Option<Uuid>,
    duplicate_of: Option<Uuid>,
    handler_id: Option<Uuid>,
    handler_username: Option<String>,
    resolution_note: Option<String>,
    handled_at: Option<DateTime<Utc>>,
    cancelled_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
struct CaseRow {
    id: Uuid,
    target_type: String,
    target_id: Uuid,
    status: String,
    priority: String,
    risk_score: i32,
    source: String,
    assignee_id: Option<Uuid>,
    assignee_username: Option<String>,
    opened_by: Option<Uuid>,
    opened_at: DateTime<Utc>,
    closed_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
pub struct CaseQueueRow {
    pub id: Uuid,
    pub target_type: String,
    pub target_id: Uuid,
    pub status: String,
    pub priority: String,
    pub risk_score: i32,
    pub source: String,
    pub assignee_id: Option<Uuid>,
    pub assignee_username: Option<String>,
    pub report_count: i64,
    pub content_summary: Option<String>,
    pub author_id: Option<Uuid>,
    pub author_username: Option<String>,
    pub opened_by: Option<Uuid>,
    pub opened_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,
}

#[derive(sqlx::FromRow)]
struct ActionRow {
    id: Uuid,
    case_id: Option<Uuid>,
    action: String,
    target_type: String,
    target_id: Uuid,
    before_status: Option<String>,
    after_status: Option<String>,
    reason: Option<String>,
    operator_id: Option<Uuid>,
    operator_username: Option<String>,
    report_id: Option<Uuid>,
    created_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
struct SnapshotRow {
    id: Uuid,
    target_type: String,
    target_id: Uuid,
    title: Option<String>,
    content: Option<String>,
    summary: Option<String>,
    status: Option<String>,
    reason: Option<String>,
    created_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
struct SanctionRow {
    id: Uuid,
    user_id: Uuid,
    username: String,
    sanction_type: String,
    reason: String,
    user_visible_reason: Option<String>,
    internal_note: Option<String>,
    restrictions: Vec<String>,
    starts_at: DateTime<Utc>,
    ends_at: Option<DateTime<Utc>>,
    is_permanent: bool,
    status: String,
    issued_by: Option<Uuid>,
    issuer_username: Option<String>,
    case_id: Option<Uuid>,
    report_id: Option<Uuid>,
    related_content_type: Option<String>,
    related_content_id: Option<Uuid>,
    revoked_by: Option<Uuid>,
    revoked_at: Option<DateTime<Utc>>,
    revoke_reason: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
struct AppealRow {
    id: Uuid,
    user_id: Uuid,
    username: String,
    appeal_type: String,
    sanction_id: Option<Uuid>,
    content_type: Option<String>,
    content_id: Option<Uuid>,
    reason: String,
    details: Option<String>,
    evidence: Value,
    status: String,
    reviewer_id: Option<Uuid>,
    reviewer_username: Option<String>,
    review_note: Option<String>,
    reviewed_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
struct RuleRow {
    id: Uuid,
    name: String,
    rule_type: String,
    target_type: String,
    priority: i32,
    enabled: bool,
    risk_score: i32,
    action: String,
    config: Value,
    hit_count: i64,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
struct RuleHitRow {
    id: Uuid,
    rule_id: Uuid,
    rule_name: String,
    rule_type: String,
    target_type: String,
    risk_score: i32,
    action: String,
    content_snippet: Option<String>,
    created_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
struct NoteRow {
    id: Uuid,
    case_id: Uuid,
    author_id: Uuid,
    author_username: String,
    note: String,
    created_at: DateTime<Utc>,
}

impl ModerationRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    // ------------------------------------------------------------------
    // Reports
    // ------------------------------------------------------------------

    pub async fn create_report(
        &self,
        reporter_id: Uuid,
        target_type: &str,
        target_id: Uuid,
        reason_code: &str,
        details: Option<&str>,
        priority: &str,
        risk_score: i32,
    ) -> Result<ReportItemV2, sqlx::Error> {
        let id = sqlx::query_scalar::<_, Uuid>(
            r#"
            INSERT INTO reports (reporter_id, target_type, target_id, reason, details, priority, risk_score)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING id
            "#,
        )
        .bind(reporter_id)
        .bind(target_type)
        .bind(target_id)
        .bind(reason_code)
        .bind(details)
        .bind(priority)
        .bind(risk_score)
        .fetch_one(&self.pool)
        .await?;
        self.get_report(id).await?.ok_or(sqlx::Error::RowNotFound)
    }

    pub async fn find_recent_report(
        &self,
        reporter_id: Uuid,
        target_type: &str,
        target_id: Uuid,
        window_hours: i64,
    ) -> Result<Option<ReportItemV2>, sqlx::Error> {
        let row = sqlx::query_as::<_, ReportRow>(
            r#"
            SELECT r.id, r.reporter_id, ru.username AS reporter_username,
                   r.target_type, r.target_id, r.reason, r.details, r.status,
                   r.priority, r.risk_score, r.case_id, r.duplicate_of,
                   r.handler_id, hu.username AS handler_username, r.resolution_note,
                   r.handled_at, r.cancelled_at, r.created_at, r.updated_at
            FROM reports r
            JOIN users ru ON ru.id = r.reporter_id
            LEFT JOIN users hu ON hu.id = r.handler_id
            WHERE r.reporter_id = $1 AND r.target_type = $2 AND r.target_id = $3
              AND r.status IN ('open', 'reviewing')
              AND r.created_at >= now() - make_interval(hours => $4)
            ORDER BY r.created_at DESC
            LIMIT 1
            "#,
        )
        .bind(reporter_id)
        .bind(target_type)
        .bind(target_id)
        .bind(window_hours)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(map_report))
    }

    pub async fn get_report(&self, report_id: Uuid) -> Result<Option<ReportItemV2>, sqlx::Error> {
        let row = sqlx::query_as::<_, ReportRow>(
            r#"
            SELECT r.id, r.reporter_id, ru.username AS reporter_username,
                   r.target_type, r.target_id, r.reason, r.details, r.status,
                   r.priority, r.risk_score, r.case_id, r.duplicate_of,
                   r.handler_id, hu.username AS handler_username, r.resolution_note,
                   r.handled_at, r.cancelled_at, r.created_at, r.updated_at
            FROM reports r
            JOIN users ru ON ru.id = r.reporter_id
            LEFT JOIN users hu ON hu.id = r.handler_id
            WHERE r.id = $1
            "#,
        )
        .bind(report_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(map_report))
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn list_reports(
        &self,
        q: Option<&str>,
        status: Option<&str>,
        target_type: Option<&str>,
        reason: Option<&str>,
        priority: Option<&str>,
        assignee_id: Option<Uuid>,
        unassigned: bool,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<ReportItemV2>, i64), sqlx::Error> {
        let total = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT count(*)
            FROM reports r
            JOIN users ru ON ru.id = r.reporter_id
            WHERE ($1::text IS NULL
                    OR r.reason ILIKE '%' || $1 || '%'
                    OR COALESCE(r.details, '') ILIKE '%' || $1 || '%'
                    OR ru.username ILIKE '%' || $1 || '%')
              AND ($2::text IS NULL OR r.status = $2)
              AND ($3::text IS NULL OR r.target_type = $3)
              AND ($4::text IS NULL OR r.reason = $4)
              AND ($5::text IS NULL OR r.priority = $5)
              AND ($6::boolean OR r.handler_id = $7)
              AND ($8::timestamptz IS NULL OR r.created_at >= $8)
              AND ($9::timestamptz IS NULL OR r.created_at <= $9)
            "#,
        )
        .bind(q)
        .bind(status)
        .bind(target_type)
        .bind(reason)
        .bind(priority)
        .bind(unassigned)
        .bind(assignee_id)
        .bind(from)
        .bind(to)
        .fetch_one(&self.pool)
        .await?;

        let rows = sqlx::query_as::<_, ReportRow>(
            r#"
            SELECT r.id, r.reporter_id, ru.username AS reporter_username,
                   r.target_type, r.target_id, r.reason, r.details, r.status,
                   r.priority, r.risk_score, r.case_id, r.duplicate_of,
                   r.handler_id, hu.username AS handler_username, r.resolution_note,
                   r.handled_at, r.cancelled_at, r.created_at, r.updated_at
            FROM reports r
            JOIN users ru ON ru.id = r.reporter_id
            LEFT JOIN users hu ON hu.id = r.handler_id
            WHERE ($1::text IS NULL
                    OR r.reason ILIKE '%' || $1 || '%'
                    OR COALESCE(r.details, '') ILIKE '%' || $1 || '%'
                    OR ru.username ILIKE '%' || $1 || '%')
              AND ($2::text IS NULL OR r.status = $2)
              AND ($3::text IS NULL OR r.target_type = $3)
              AND ($4::text IS NULL OR r.reason = $4)
              AND ($5::text IS NULL OR r.priority = $5)
              AND ($6::boolean OR r.handler_id = $7)
              AND ($8::timestamptz IS NULL OR r.created_at >= $8)
              AND ($9::timestamptz IS NULL OR r.created_at <= $9)
            ORDER BY r.created_at DESC
            LIMIT $10 OFFSET $11
            "#,
        )
        .bind(q)
        .bind(status)
        .bind(target_type)
        .bind(reason)
        .bind(priority)
        .bind(unassigned)
        .bind(assignee_id)
        .bind(from)
        .bind(to)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok((rows.into_iter().map(map_report).collect(), total))
    }

    pub async fn list_reports_by_user(
        &self,
        reporter_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<ReportItemV2>, i64), sqlx::Error> {
        let total = sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM reports WHERE reporter_id = $1",
        )
        .bind(reporter_id)
        .fetch_one(&self.pool)
        .await?;
        let rows = sqlx::query_as::<_, ReportRow>(
            r#"
            SELECT r.id, r.reporter_id, ru.username AS reporter_username,
                   r.target_type, r.target_id, r.reason, r.details, r.status,
                   r.priority, r.risk_score, r.case_id, r.duplicate_of,
                   r.handler_id, hu.username AS handler_username, r.resolution_note,
                   r.handled_at, r.cancelled_at, r.created_at, r.updated_at
            FROM reports r
            JOIN users ru ON ru.id = r.reporter_id
            LEFT JOIN users hu ON hu.id = r.handler_id
            WHERE r.reporter_id = $1
            ORDER BY r.created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(reporter_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;
        Ok((rows.into_iter().map(map_report).collect(), total))
    }

    /// Guarded status transition: only open/reviewing reports may change,
    /// terminal states are idempotent.
    #[allow(clippy::too_many_arguments)]
    pub async fn update_report_status(
        &self,
        tx: Option<&mut Transaction<'_, Postgres>>,
        report_id: Uuid,
        handler_id: Option<Uuid>,
        status: &str,
        note: Option<&str>,
        duplicate_of: Option<Uuid>,
    ) -> Result<bool, sqlx::Error> {
        let query = r#"
            UPDATE reports
            SET status = $2,
                handler_id = COALESCE($3, handler_id),
                resolution_note = $4,
                duplicate_of = $5,
                handled_at = CASE WHEN $2 IN ('resolved', 'rejected', 'duplicate') THEN now() ELSE handled_at END,
                cancelled_at = CASE WHEN $2 = 'cancelled' THEN now() ELSE cancelled_at END
            WHERE id = $1 AND status IN ('open', 'reviewing')
        "#;
        let result = match tx {
            Some(tx) => {
                sqlx::query(query)
                    .bind(report_id)
                    .bind(status)
                    .bind(handler_id)
                    .bind(note)
                    .bind(duplicate_of)
                    .execute(&mut **tx)
                    .await?
            }
            None => {
                sqlx::query(query)
                    .bind(report_id)
                    .bind(status)
                    .bind(handler_id)
                    .bind(note)
                    .bind(duplicate_of)
                    .execute(&self.pool)
                    .await?
            }
        };
        Ok(result.rows_affected() == 1)
    }

    pub async fn insert_report_event(
        &self,
        report_id: Uuid,
        actor_type: &str,
        actor_id: Option<Uuid>,
        action: &str,
        note: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO report_events (report_id, actor_type, actor_id, action, note)
            VALUES ($1, $2, $3, $4, $5)
            "#,
        )
        .bind(report_id)
        .bind(actor_type)
        .bind(actor_id)
        .bind(action)
        .bind(note)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Record a case-level lifecycle event against every open report in the case.
    pub async fn insert_report_event_from_case(
        &self,
        case_id: Uuid,
        actor_id: Uuid,
        action: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO report_events (report_id, actor_type, actor_id, action)
            SELECT r.id, 'moderator', $2, $3
            FROM reports r
            WHERE r.case_id = $1 AND r.status IN ('open', 'reviewing')
            "#,
        )
        .bind(case_id)
        .bind(actor_id)
        .bind(action)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn list_reports_by_case(
        &self,
        case_id: Uuid,
    ) -> Result<Vec<ReportItemV2>, sqlx::Error> {
        let rows = sqlx::query_as::<_, ReportRow>(
            r#"
            SELECT r.id, r.reporter_id, ru.username AS reporter_username,
                   r.target_type, r.target_id, r.reason, r.details, r.status,
                   r.priority, r.risk_score, r.case_id, r.duplicate_of,
                   r.handler_id, hu.username AS handler_username, r.resolution_note,
                   r.handled_at, r.cancelled_at, r.created_at, r.updated_at
            FROM reports r
            JOIN users ru ON ru.id = r.reporter_id
            LEFT JOIN users hu ON hu.id = r.handler_id
            WHERE r.case_id = $1
            ORDER BY r.created_at ASC
            "#,
        )
        .bind(case_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(map_report).collect())
    }

    // ------------------------------------------------------------------
    // Cases
    // ------------------------------------------------------------------

    pub async fn find_open_case(
        &self,
        target_type: &str,
        target_id: Uuid,
    ) -> Result<Option<Uuid>, sqlx::Error> {
        sqlx::query_scalar::<_, Uuid>(
            r#"
            SELECT id FROM moderation_cases
            WHERE target_type = $1 AND target_id = $2 AND status IN ('open', 'reviewing')
            LIMIT 1
            "#,
        )
        .bind(target_type)
        .bind(target_id)
        .fetch_optional(&self.pool)
        .await
    }

    /// Create a case, racing safely against the partial unique index.
    pub async fn create_case(
        &self,
        target_type: &str,
        target_id: Uuid,
        priority: &str,
        risk_score: i32,
        source: &str,
        opened_by: Option<Uuid>,
    ) -> Result<Uuid, sqlx::Error> {
        let inserted = sqlx::query_scalar::<_, Uuid>(
            r#"
            INSERT INTO moderation_cases (target_type, target_id, priority, risk_score, source, opened_by)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (target_type, target_id) WHERE status IN ('open', 'reviewing') DO NOTHING
            RETURNING id
            "#,
        )
        .bind(target_type)
        .bind(target_id)
        .bind(priority)
        .bind(risk_score)
        .bind(source)
        .bind(opened_by)
        .fetch_optional(&self.pool)
        .await?;
        match inserted {
            Some(id) => Ok(id),
            None => {
                self.find_open_case(target_type, target_id)
                    .await?
                    .ok_or(sqlx::Error::RowNotFound)
            }
        }
    }

    pub async fn link_report_to_case(
        &self,
        report_id: Uuid,
        case_id: Uuid,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE reports SET case_id = $2 WHERE id = $1 AND case_id IS NULL",
        )
        .bind(report_id)
        .bind(case_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn bump_case_priority(
        &self,
        case_id: Uuid,
        priority: &str,
        risk_score: i32,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE moderation_cases
            SET priority = CASE
                    WHEN priority IN ('low', 'normal', 'high', 'urgent')
                         AND (priority = 'urgent' OR $2 = 'urgent' OR priority = 'high' OR $2 = 'high') THEN
                        CASE
                            WHEN priority = 'urgent' OR $2 = 'urgent' THEN 'urgent'
                            WHEN priority = 'high' OR $2 = 'high' THEN 'high'
                            ELSE priority
                        END
                    ELSE priority
                END,
                risk_score = GREATEST(risk_score, $3)
            WHERE id = $1
            "#,
        )
        .bind(case_id)
        .bind(priority)
        .bind(risk_score)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_case(&self, case_id: Uuid) -> Result<Option<CaseItem>, sqlx::Error> {
        let row = sqlx::query_as::<_, CaseRow>(
            r#"
            SELECT c.id, c.target_type, c.target_id, c.status, c.priority, c.risk_score,
                   c.source, c.assignee_id, au.username AS assignee_username,
                   c.opened_by, c.opened_at, c.closed_at, c.created_at, c.updated_at
            FROM moderation_cases c
            LEFT JOIN users au ON au.id = c.assignee_id
            WHERE c.id = $1
            "#,
        )
        .bind(case_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(map_case))
    }

    pub async fn get_case_by_target(
        &self,
        target_type: &str,
        target_id: Uuid,
    ) -> Result<Option<CaseItem>, sqlx::Error> {
        let row = sqlx::query_as::<_, CaseRow>(
            r#"
            SELECT c.id, c.target_type, c.target_id, c.status, c.priority, c.risk_score,
                   c.source, c.assignee_id, au.username AS assignee_username,
                   c.opened_by, c.opened_at, c.closed_at, c.created_at, c.updated_at
            FROM moderation_cases c
            LEFT JOIN users au ON au.id = c.assignee_id
            WHERE c.target_type = $1 AND c.target_id = $2
              AND c.status IN ('open', 'reviewing')
            LIMIT 1
            "#,
        )
        .bind(target_type)
        .bind(target_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(map_case))
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn list_cases(
        &self,
        q: Option<&str>,
        status: Option<&str>,
        priority: Option<&str>,
        source: Option<&str>,
        target_type: Option<&str>,
        assignee_id: Option<Uuid>,
        unassigned: bool,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<CaseQueueRow>, i64), sqlx::Error> {
        let total = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT count(*)
            FROM moderation_cases c
            LEFT JOIN users au ON au.id = c.assignee_id
            WHERE ($1::text IS NULL
                    OR au.username ILIKE '%' || $1 || '%'
                    OR c.target_id::text ILIKE '%' || $1 || '%')
              AND ($2::text IS NULL OR c.status = $2)
              AND ($3::text IS NULL OR c.priority = $3)
              AND ($4::text IS NULL OR c.source = $4)
              AND ($5::text IS NULL OR c.target_type = $5)
              AND ($6::boolean OR c.assignee_id = $7)
            "#,
        )
        .bind(q)
        .bind(status)
        .bind(priority)
        .bind(source)
        .bind(target_type)
        .bind(unassigned)
        .bind(assignee_id)
        .fetch_one(&self.pool)
        .await?;

        let rows = sqlx::query_as::<_, CaseQueueRow>(
            r#"
            SELECT c.id, c.target_type, c.target_id, c.status, c.priority, c.risk_score,
                   c.source, c.assignee_id, au.username AS assignee_username,
                   (SELECT count(*) FROM reports r WHERE r.case_id = c.id) AS report_count,
                   CASE c.target_type
                       WHEN 'topic' THEN (SELECT left(t.title, 120) FROM topics t WHERE t.id = c.target_id)
                       WHEN 'comment' THEN (SELECT left(cm.content, 120) FROM comments cm WHERE cm.id = c.target_id)
                       WHEN 'user' THEN (SELECT u.username FROM users u WHERE u.id = c.target_id)
                       ELSE NULL
                   END AS content_summary,
                   CASE c.target_type
                       WHEN 'topic' THEN (SELECT t.author_id FROM topics t WHERE t.id = c.target_id)
                       WHEN 'comment' THEN (SELECT cm.author_id FROM comments cm WHERE cm.id = c.target_id)
                       WHEN 'user' THEN c.target_id
                       ELSE NULL
                   END AS author_id,
                   CASE c.target_type
                       WHEN 'topic' THEN (SELECT tu.username FROM topics t JOIN users tu ON tu.id = t.author_id WHERE t.id = c.target_id)
                       WHEN 'comment' THEN (SELECT cu.username FROM comments cm JOIN users cu ON cu.id = cm.author_id WHERE cm.id = c.target_id)
                       WHEN 'user' THEN (SELECT u.username FROM users u WHERE u.id = c.target_id)
                       ELSE NULL
                   END AS author_username,
                   c.opened_by, c.opened_at, c.created_at, c.updated_at, c.closed_at
            FROM moderation_cases c
            LEFT JOIN users au ON au.id = c.assignee_id
            WHERE ($1::text IS NULL
                    OR au.username ILIKE '%' || $1 || '%'
                    OR c.target_id::text ILIKE '%' || $1 || '%')
              AND ($2::text IS NULL OR c.status = $2)
              AND ($3::text IS NULL OR c.priority = $3)
              AND ($4::text IS NULL OR c.source = $4)
              AND ($5::text IS NULL OR c.target_type = $5)
              AND ($6::boolean OR c.assignee_id = $7)
            ORDER BY
                CASE c.priority WHEN 'urgent' THEN 0 WHEN 'high' THEN 1 WHEN 'normal' THEN 2 ELSE 3 END,
                c.created_at ASC
            LIMIT $8 OFFSET $9
            "#,
        )
        .bind(q)
        .bind(status)
        .bind(priority)
        .bind(source)
        .bind(target_type)
        .bind(unassigned)
        .bind(assignee_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok((rows, total))
    }

    /// Guarded case assignment: only open cases may be claimed/released.
    pub async fn update_case_assignment(
        &self,
        case_id: Uuid,
        assignee_id: Option<Uuid>,
        status: &str,
    ) -> Result<bool, sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        let result = sqlx::query(
            r#"
            UPDATE moderation_cases
            SET assignee_id = $2, status = $3
            WHERE id = $1 AND status IN ('open', 'reviewing')
            "#,
        )
        .bind(case_id)
        .bind(assignee_id)
        .bind(status)
        .execute(&mut *tx)
        .await?;
        if result.rows_affected() == 0 {
            tx.rollback().await?;
            return Ok(false);
        }
        sqlx::query(
            r#"
            UPDATE reports
            SET handler_id = $2,
                status = CASE WHEN $2::uuid IS NULL THEN 'open' ELSE 'reviewing' END
            WHERE case_id = $1 AND status IN ('open', 'reviewing')
            "#,
        )
        .bind(case_id)
        .bind(assignee_id)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(true)
    }

    pub async fn close_case(
        &self,
        case_id: Uuid,
    ) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            r#"
            UPDATE moderation_cases
            SET status = 'closed', closed_at = now()
            WHERE id = $1 AND status IN ('open', 'reviewing')
            "#,
        )
        .bind(case_id)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() == 1)
    }

    pub async fn reopen_case(&self, case_id: Uuid) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            r#"
            UPDATE moderation_cases
            SET status = 'open', closed_at = NULL
            WHERE id = $1 AND status = 'closed'
            "#,
        )
        .bind(case_id)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() == 1)
    }

    // ------------------------------------------------------------------
    // Actions & snapshots & notes
    // ------------------------------------------------------------------

    #[allow(clippy::too_many_arguments)]
    pub async fn insert_action(
        &self,
        tx: Option<&mut Transaction<'_, Postgres>>,
        case_id: Option<Uuid>,
        action: &str,
        target_type: &str,
        target_id: Uuid,
        before_status: Option<&str>,
        after_status: Option<&str>,
        reason: Option<&str>,
        operator_id: Option<Uuid>,
        report_id: Option<Uuid>,
        metadata: Value,
    ) -> Result<(), sqlx::Error> {
        let query = r#"
            INSERT INTO moderation_actions (
                case_id, action, target_type, target_id, before_status, after_status,
                reason, operator_id, report_id, metadata
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        "#;
        match tx {
            Some(tx) => {
                sqlx::query(query)
                    .bind(case_id)
                    .bind(action)
                    .bind(target_type)
                    .bind(target_id)
                    .bind(before_status)
                    .bind(after_status)
                    .bind(reason)
                    .bind(operator_id)
                    .bind(report_id)
                    .bind(metadata)
                    .execute(&mut **tx)
                    .await?;
            }
            None => {
                sqlx::query(query)
                    .bind(case_id)
                    .bind(action)
                    .bind(target_type)
                    .bind(target_id)
                    .bind(before_status)
                    .bind(after_status)
                    .bind(reason)
                    .bind(operator_id)
                    .bind(report_id)
                    .bind(metadata)
                    .execute(&self.pool)
                    .await?;
            }
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn insert_snapshot(
        &self,
        tx: Option<&mut Transaction<'_, Postgres>>,
        case_id: Option<Uuid>,
        target_type: &str,
        target_id: Uuid,
        title: Option<&str>,
        content: Option<&str>,
        summary: Option<&str>,
        status: Option<&str>,
        reason: Option<&str>,
        created_by: Option<Uuid>,
    ) -> Result<(), sqlx::Error> {
        let query = r#"
            INSERT INTO content_snapshots (
                case_id, target_type, target_id, title, content, summary, status, reason, created_by
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        "#;
        match tx {
            Some(tx) => {
                sqlx::query(query)
                    .bind(case_id)
                    .bind(target_type)
                    .bind(target_id)
                    .bind(title)
                    .bind(content)
                    .bind(summary)
                    .bind(status)
                    .bind(reason)
                    .bind(created_by)
                    .execute(&mut **tx)
                    .await?;
            }
            None => {
                sqlx::query(query)
                    .bind(case_id)
                    .bind(target_type)
                    .bind(target_id)
                    .bind(title)
                    .bind(content)
                    .bind(summary)
                    .bind(status)
                    .bind(reason)
                    .bind(created_by)
                    .execute(&self.pool)
                    .await?;
            }
        }
        Ok(())
    }

    pub async fn list_actions_by_case(
        &self,
        case_id: Uuid,
    ) -> Result<Vec<ModerationActionItem>, sqlx::Error> {
        let rows = sqlx::query_as::<_, ActionRow>(
            r#"
            SELECT a.id, a.case_id, a.action, a.target_type, a.target_id,
                   a.before_status, a.after_status, a.reason,
                   a.operator_id, ou.username AS operator_username, a.report_id, a.created_at
            FROM moderation_actions a
            LEFT JOIN users ou ON ou.id = a.operator_id
            WHERE a.case_id = $1
            ORDER BY a.created_at ASC
            "#,
        )
        .bind(case_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|row| ModerationActionItem {
                id: row.id,
                case_id: row.case_id,
                action: row.action,
                target_type: row.target_type,
                target_id: row.target_id,
                before_status: row.before_status,
                after_status: row.after_status,
                reason: row.reason,
                operator_id: row.operator_id,
                operator_username: row.operator_username,
                report_id: row.report_id,
                created_at: row.created_at,
            })
            .collect())
    }

    pub async fn list_snapshots_by_case(
        &self,
        case_id: Uuid,
    ) -> Result<Vec<crate::models::ContentSnapshotItem>, sqlx::Error> {
        let rows = sqlx::query_as::<_, SnapshotRow>(
            r#"
            SELECT id, target_type, target_id, title, content, summary, status, reason, created_at
            FROM content_snapshots
            WHERE case_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(case_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|row| crate::models::ContentSnapshotItem {
                id: row.id,
                target_type: row.target_type,
                target_id: row.target_id,
                title: row.title,
                content: row.content,
                summary: row.summary,
                status: row.status,
                reason: row.reason,
                created_at: row.created_at,
            })
            .collect())
    }

    pub async fn list_notes_by_case(
        &self,
        case_id: Uuid,
    ) -> Result<Vec<ModerationNoteItem>, sqlx::Error> {
        let rows = sqlx::query_as::<_, NoteRow>(
            r#"
            SELECT n.id, n.case_id, n.author_id, u.username AS author_username, n.note, n.created_at
            FROM moderation_notes n
            JOIN users u ON u.id = n.author_id
            WHERE n.case_id = $1
            ORDER BY n.created_at ASC
            "#,
        )
        .bind(case_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|row| ModerationNoteItem {
                id: row.id,
                case_id: row.case_id,
                author_id: row.author_id,
                author_username: row.author_username,
                note: row.note,
                created_at: row.created_at,
            })
            .collect())
    }

    pub async fn insert_note(
        &self,
        case_id: Uuid,
        author_id: Uuid,
        note: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO moderation_notes (case_id, author_id, note) VALUES ($1, $2, $3)",
        )
        .bind(case_id)
        .bind(author_id)
        .bind(note)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    // ------------------------------------------------------------------
    // Content rows (for moderation views & actions)
    // ------------------------------------------------------------------

    pub async fn get_topic(&self, topic_id: Uuid) -> Result<Option<ModTopicRow>, sqlx::Error> {
        sqlx::query_as::<_, ModTopicRow>(
            r#"
            SELECT t.id, t.category_id, t.author_id, u.username AS author_username,
                   t.title, t.slug, t.content, t.summary, t.status,
                   t.is_pinned, t.is_featured, t.is_locked, t.is_sensitive, t.restrict_interactions,
                   t.deleted_at, t.created_at, t.updated_at
            FROM topics t
            JOIN users u ON u.id = t.author_id
            WHERE t.id = $1
            "#,
        )
        .bind(topic_id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn get_comment(&self, comment_id: Uuid) -> Result<Option<ModCommentRow>, sqlx::Error> {
        sqlx::query_as::<_, ModCommentRow>(
            r#"
            SELECT c.id, c.topic_id, t.slug AS topic_slug, t.title AS topic_title,
                   c.parent_id, c.author_id, u.username AS author_username,
                   c.content, c.status, c.is_collapsed, c.is_sensitive, c.replies_locked,
                   c.deleted_at, c.created_at, c.updated_at
            FROM comments c
            JOIN topics t ON t.id = c.topic_id
            JOIN users u ON u.id = c.author_id
            WHERE c.id = $1
            "#,
        )
        .bind(comment_id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn get_user(&self, user_id: Uuid) -> Result<Option<ModUserRow>, sqlx::Error> {
        sqlx::query_as::<_, ModUserRow>(
            r#"
            SELECT u.id, u.username, r.code AS role_code, r.priority AS role_priority,
                   u.status, u.auth_version, u.created_at
            FROM users u
            JOIN roles r ON r.id = u.role_id
            WHERE u.id = $1
            "#,
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn lock_target_user(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        user_id: Uuid,
    ) -> Result<Option<ModUserRow>, sqlx::Error> {
        sqlx::query_as::<_, ModUserRow>(
            r#"
            SELECT u.id, u.username, r.code AS role_code, r.priority AS role_priority,
                   u.status, u.auth_version, u.created_at
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

    #[allow(clippy::too_many_arguments)]
    pub async fn set_topic_governance(
        &self,
        tx: Option<&mut Transaction<'_, Postgres>>,
        topic_id: Uuid,
        status: Option<&str>,
        is_locked: Option<bool>,
        is_sensitive: Option<bool>,
        restrict_interactions: Option<bool>,
        is_pinned: Option<bool>,
        category_id: Option<Uuid>,
    ) -> Result<Option<ModTopicRow>, sqlx::Error> {
        let update = r#"
            UPDATE topics
            SET status = COALESCE($2, status),
                is_locked = COALESCE($3, is_locked),
                is_sensitive = COALESCE($4, is_sensitive),
                restrict_interactions = COALESCE($5, restrict_interactions),
                is_pinned = CASE WHEN $2 = 'deleted' THEN false ELSE COALESCE($6, is_pinned) END,
                category_id = COALESCE($7, category_id),
                deleted_at = CASE WHEN $2 = 'deleted' THEN now() WHEN $2 IN ('published', 'hidden') THEN NULL ELSE deleted_at END,
                is_featured = CASE WHEN $2 = 'deleted' THEN false ELSE is_featured END
            WHERE id = $1
        "#;
        let select = r#"
            SELECT t.id, t.category_id, t.author_id, u.username AS author_username,
                   t.title, t.slug, t.content, t.summary, t.status,
                   t.is_pinned, t.is_featured, t.is_locked, t.is_sensitive, t.restrict_interactions,
                   t.deleted_at, t.created_at, t.updated_at
            FROM topics t JOIN users u ON u.id = t.author_id WHERE t.id = $1
        "#;
        match tx {
            Some(tx) => {
                let result = sqlx::query(update)
                    .bind(topic_id)
                    .bind(status)
                    .bind(is_locked)
                    .bind(is_sensitive)
                    .bind(restrict_interactions)
                    .bind(is_pinned)
                    .bind(category_id)
                    .execute(&mut **tx)
                    .await?;
                if result.rows_affected() == 0 {
                    return Ok(None);
                }
                sqlx::query_as::<_, ModTopicRow>(select)
                    .bind(topic_id)
                    .fetch_optional(&mut **tx)
                    .await
            }
            None => {
                let result = sqlx::query(update)
                    .bind(topic_id)
                    .bind(status)
                    .bind(is_locked)
                    .bind(is_sensitive)
                    .bind(restrict_interactions)
                    .bind(is_pinned)
                    .bind(category_id)
                    .execute(&self.pool)
                    .await?;
                if result.rows_affected() == 0 {
                    return Ok(None);
                }
                self.get_topic(topic_id).await
            }
        }
    }

    pub async fn set_comment_governance(
        &self,
        tx: Option<&mut Transaction<'_, Postgres>>,
        comment_id: Uuid,
        status: Option<&str>,
        is_collapsed: Option<bool>,
        is_sensitive: Option<bool>,
        replies_locked: Option<bool>,
    ) -> Result<Option<ModCommentRow>, sqlx::Error> {
        let update = r#"
            UPDATE comments
            SET status = COALESCE($2, status),
                is_collapsed = COALESCE($3, is_collapsed),
                is_sensitive = COALESCE($4, is_sensitive),
                replies_locked = COALESCE($5, replies_locked),
                deleted_at = CASE WHEN $2 = 'deleted' THEN now() WHEN $2 IN ('published', 'hidden') THEN NULL ELSE deleted_at END
            WHERE id = $1
        "#;
        let select = r#"
            SELECT c.id, c.topic_id, t.slug AS topic_slug, t.title AS topic_title,
                   c.parent_id, c.author_id, u.username AS author_username,
                   c.content, c.status, c.is_collapsed, c.is_sensitive, c.replies_locked,
                   c.deleted_at, c.created_at, c.updated_at
            FROM comments c
            JOIN topics t ON t.id = c.topic_id
            JOIN users u ON u.id = c.author_id
            WHERE c.id = $1
        "#;
        match tx {
            Some(tx) => {
                let result = sqlx::query(update)
                    .bind(comment_id)
                    .bind(status)
                    .bind(is_collapsed)
                    .bind(is_sensitive)
                    .bind(replies_locked)
                    .execute(&mut **tx)
                    .await?;
                if result.rows_affected() == 0 {
                    return Ok(None);
                }
                sqlx::query_as::<_, ModCommentRow>(select)
                    .bind(comment_id)
                    .fetch_optional(&mut **tx)
                    .await
            }
            None => {
                let result = sqlx::query(update)
                    .bind(comment_id)
                    .bind(status)
                    .bind(is_collapsed)
                    .bind(is_sensitive)
                    .bind(replies_locked)
                    .execute(&self.pool)
                    .await?;
                if result.rows_affected() == 0 {
                    return Ok(None);
                }
                self.get_comment(comment_id).await
            }
        }
    }

    // ------------------------------------------------------------------
    // Sanctions
    // ------------------------------------------------------------------

    pub async fn create_sanction(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        user_id: Uuid,
        sanction_type: &str,
        reason: &str,
        user_visible_reason: Option<&str>,
        internal_note: Option<&str>,
        restrictions: &[String],
        starts_at: DateTime<Utc>,
        ends_at: Option<DateTime<Utc>>,
        is_permanent: bool,
        status: &str,
        issued_by: Uuid,
        case_id: Option<Uuid>,
        report_id: Option<Uuid>,
        related_content_type: Option<&str>,
        related_content_id: Option<Uuid>,
    ) -> Result<Uuid, sqlx::Error> {
        let id = sqlx::query_scalar::<_, Uuid>(
            r#"
            INSERT INTO user_sanctions (
                user_id, sanction_type, reason, user_visible_reason, internal_note,
                restrictions, starts_at, ends_at, is_permanent, status,
                issued_by, case_id, report_id, related_content_type, related_content_id
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
            RETURNING id
            "#,
        )
        .bind(user_id)
        .bind(sanction_type)
        .bind(reason)
        .bind(user_visible_reason)
        .bind(internal_note)
        .bind(restrictions)
        .bind(starts_at)
        .bind(ends_at)
        .bind(is_permanent)
        .bind(status)
        .bind(issued_by)
        .bind(case_id)
        .bind(report_id)
        .bind(related_content_type)
        .bind(related_content_id)
        .fetch_one(&mut **tx)
        .await?;
        Ok(id)
    }

    pub async fn get_sanction(&self, sanction_id: Uuid) -> Result<Option<SanctionItem>, sqlx::Error> {
        let row = sqlx::query_as::<_, SanctionRow>(
            r#"
            SELECT s.id, s.user_id, u.username, s.sanction_type, s.reason,
                   s.user_visible_reason, s.internal_note, s.restrictions,
                   s.starts_at, s.ends_at, s.is_permanent, s.status,
                   s.issued_by, iu.username AS issuer_username,
                   s.case_id, s.report_id, s.related_content_type, s.related_content_id,
                   s.revoked_by, s.revoked_at, s.revoke_reason, s.created_at, s.updated_at
            FROM user_sanctions s
            JOIN users u ON u.id = s.user_id
            LEFT JOIN users iu ON iu.id = s.issued_by
            WHERE s.id = $1
            "#,
        )
        .bind(sanction_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(map_sanction))
    }

    pub async fn list_sanctions(
        &self,
        user_id: Option<Uuid>,
        status: Option<&str>,
        sanction_type: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<SanctionItem>, i64), sqlx::Error> {
        let total = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT count(*)
            FROM user_sanctions s
            JOIN users u ON u.id = s.user_id
            WHERE ($1::uuid IS NULL OR s.user_id = $1)
              AND ($2::text IS NULL OR s.status = $2)
              AND ($3::text IS NULL OR s.sanction_type = $3)
            "#,
        )
        .bind(user_id)
        .bind(status)
        .bind(sanction_type)
        .fetch_one(&self.pool)
        .await?;

        let rows = sqlx::query_as::<_, SanctionRow>(
            r#"
            SELECT s.id, s.user_id, u.username, s.sanction_type, s.reason,
                   s.user_visible_reason, s.internal_note, s.restrictions,
                   s.starts_at, s.ends_at, s.is_permanent, s.status,
                   s.issued_by, iu.username AS issuer_username,
                   s.case_id, s.report_id, s.related_content_type, s.related_content_id,
                   s.revoked_by, s.revoked_at, s.revoke_reason, s.created_at, s.updated_at
            FROM user_sanctions s
            JOIN users u ON u.id = s.user_id
            LEFT JOIN users iu ON iu.id = s.issued_by
            WHERE ($1::uuid IS NULL OR s.user_id = $1)
              AND ($2::text IS NULL OR s.status = $2)
              AND ($3::text IS NULL OR s.sanction_type = $3)
            ORDER BY s.created_at DESC
            LIMIT $4 OFFSET $5
            "#,
        )
        .bind(user_id)
        .bind(status)
        .bind(sanction_type)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok((rows.into_iter().map(map_sanction).collect(), total))
    }

    /// Active (enforced) restrictions for a user, if any.
    pub async fn active_restrictions(&self, user_id: Uuid) -> Result<Vec<String>, sqlx::Error> {
        let rows: Vec<String> = sqlx::query_scalar(
            r#"
            SELECT unnest(restrictions)
            FROM user_sanctions
            WHERE user_id = $1
              AND status = 'active'
              AND starts_at <= now()
              AND (is_permanent OR ends_at >= now())
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;
        let mut seen = Vec::new();
        for row in rows {
            if !seen.contains(&row) {
                seen.push(row);
            }
        }
        Ok(seen)
    }

    pub async fn is_topic_locked(&self, topic_id: Uuid) -> Result<bool, sqlx::Error> {
        sqlx::query_scalar::<_, bool>(
            "SELECT is_locked FROM topics WHERE id = $1 AND status IN ('published', 'hidden')",
        )
        .bind(topic_id)
        .fetch_optional(&self.pool)
        .await
        .map(|value| value.unwrap_or(false))
    }

    pub async fn is_comment_replies_locked(&self, comment_id: Uuid) -> Result<bool, sqlx::Error> {
        sqlx::query_scalar::<_, bool>(
            "SELECT replies_locked FROM comments WHERE id = $1 AND status = 'published'",
        )
        .bind(comment_id)
        .fetch_optional(&self.pool)
        .await
        .map(|value| value.unwrap_or(false))
    }

    pub async fn get_target_author(
        &self,
        target_type: &str,
        target_id: Uuid,
    ) -> Result<Option<Uuid>, sqlx::Error> {
        match target_type {
            "topic" => {
                sqlx::query_scalar::<_, Uuid>("SELECT author_id FROM topics WHERE id = $1")
                    .bind(target_id)
                    .fetch_optional(&self.pool)
                    .await
            }
            "comment" => {
                sqlx::query_scalar::<_, Uuid>("SELECT author_id FROM comments WHERE id = $1")
                    .bind(target_id)
                    .fetch_optional(&self.pool)
                    .await
            }
            "user" => Ok(Some(target_id)),
            _ => Ok(None),
        }
    }

    pub async fn count_user_uploads(&self, user_id: Uuid, ids: &[Uuid]) -> Result<i64, sqlx::Error> {
        if ids.is_empty() {
            return Ok(0);
        }
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM uploads WHERE user_id = $1 AND id = ANY($2)",
        )
        .bind(user_id)
        .bind(ids)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn recent_content_count(
        &self,
        user_id: Uuid,
        window_secs: i64,
    ) -> Result<i64, sqlx::Error> {
        sqlx::query_scalar::<_, i64>(
            r#"
            SELECT count(*)
            FROM (
                SELECT id, created_at FROM topics WHERE author_id = $1
                UNION ALL
                SELECT id, created_at FROM comments WHERE author_id = $1
            ) recent
            WHERE created_at >= now() - make_interval(secs => $2)
            "#,
        )
        .bind(user_id)
        .bind(window_secs)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn set_sanction_status(
        &self,
        tx: Option<&mut Transaction<'_, Postgres>>,
        sanction_id: Uuid,
        status: &str,
        revoked_by: Option<Uuid>,
        revoke_reason: Option<&str>,
    ) -> Result<bool, sqlx::Error> {
        let query = r#"
            UPDATE user_sanctions
            SET status = $2,
                revoked_by = $3,
                revoked_at = CASE WHEN $2 = 'revoked' THEN now() ELSE revoked_at END,
                revoke_reason = $4
            WHERE id = $1 AND status IN ('scheduled', 'active')
        "#;
        let result = match tx {
            Some(tx) => {
                sqlx::query(query)
                    .bind(sanction_id)
                    .bind(status)
                    .bind(revoked_by)
                    .bind(revoke_reason)
                    .execute(&mut **tx)
                    .await?
            }
            None => {
                sqlx::query(query)
                    .bind(sanction_id)
                    .bind(status)
                    .bind(revoked_by)
                    .bind(revoke_reason)
                    .execute(&self.pool)
                    .await?
            }
        };
        Ok(result.rows_affected() == 1)
    }

    /// True if the user still has an active suspension/ban after this change.
    pub async fn has_active_account_ban(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        user_id: Uuid,
    ) -> Result<bool, sqlx::Error> {
        sqlx::query_scalar::<_, bool>(
            r#"
            SELECT EXISTS (
                SELECT 1 FROM user_sanctions
                WHERE user_id = $1
                  AND status = 'active'
                  AND sanction_type IN ('suspension', 'ban')
                  AND (is_permanent OR ends_at >= now())
            )
            "#,
        )
        .bind(user_id)
        .fetch_one(&mut **tx)
        .await
    }

    pub async fn set_user_status(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        user_id: Uuid,
        status: &str,
        bump_auth: bool,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE users
            SET status = $2,
                auth_version = CASE WHEN $3 THEN auth_version + 1 ELSE auth_version END
            WHERE id = $1
            "#,
        )
        .bind(user_id)
        .bind(status)
        .bind(bump_auth)
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub async fn revoke_refresh_tokens(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        user_id: Uuid,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE refresh_tokens
            SET revoked_at = now(), revocation_reason = 'moderation_action'
            WHERE user_id = $1 AND revoked_at IS NULL
            "#,
        )
        .bind(user_id)
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    // ------------------------------------------------------------------
    // Appeals
    // ------------------------------------------------------------------

    pub async fn create_appeal(
        &self,
        user_id: Uuid,
        appeal_type: &str,
        sanction_id: Option<Uuid>,
        content_type: Option<&str>,
        content_id: Option<Uuid>,
        reason: &str,
        details: Option<&str>,
        evidence: &[Uuid],
    ) -> Result<Uuid, sqlx::Error> {
        let evidence: Value = serde_json::to_value(evidence)
            .unwrap_or_else(|_| serde_json::json!([]));
        sqlx::query_scalar::<_, Uuid>(
            r#"
            INSERT INTO appeals (
                user_id, appeal_type, sanction_id, content_type, content_id,
                reason, details, evidence
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING id
            "#,
        )
        .bind(user_id)
        .bind(appeal_type)
        .bind(sanction_id)
        .bind(content_type)
        .bind(content_id)
        .bind(reason)
        .bind(details)
        .bind(evidence)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn get_appeal(&self, appeal_id: Uuid) -> Result<Option<AppealItem>, sqlx::Error> {
        let row = sqlx::query_as::<_, AppealRow>(
            r#"
            SELECT a.id, a.user_id, u.username, a.appeal_type, a.sanction_id,
                   a.content_type, a.content_id, a.reason, a.details, a.evidence,
                   a.status, a.reviewer_id, ru.username AS reviewer_username,
                   a.review_note, a.reviewed_at, a.created_at, a.updated_at
            FROM appeals a
            JOIN users u ON u.id = a.user_id
            LEFT JOIN users ru ON ru.id = a.reviewer_id
            WHERE a.id = $1
            "#,
        )
        .bind(appeal_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(map_appeal))
    }

    pub async fn list_appeals(
        &self,
        user_id: Option<Uuid>,
        status: Option<&str>,
        q: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<AppealItem>, i64), sqlx::Error> {
        let total = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT count(*)
            FROM appeals a
            JOIN users u ON u.id = a.user_id
            WHERE ($1::uuid IS NULL OR a.user_id = $1)
              AND ($2::text IS NULL OR a.status = $2)
              AND ($3::text IS NULL OR u.username ILIKE '%' || $3 || '%' OR a.reason ILIKE '%' || $3 || '%')
            "#,
        )
        .bind(user_id)
        .bind(status)
        .bind(q)
        .fetch_one(&self.pool)
        .await?;

        let rows = sqlx::query_as::<_, AppealRow>(
            r#"
            SELECT a.id, a.user_id, u.username, a.appeal_type, a.sanction_id,
                   a.content_type, a.content_id, a.reason, a.details, a.evidence,
                   a.status, a.reviewer_id, ru.username AS reviewer_username,
                   a.review_note, a.reviewed_at, a.created_at, a.updated_at
            FROM appeals a
            JOIN users u ON u.id = a.user_id
            LEFT JOIN users ru ON ru.id = a.reviewer_id
            WHERE ($1::uuid IS NULL OR a.user_id = $1)
              AND ($2::text IS NULL OR a.status = $2)
              AND ($3::text IS NULL OR u.username ILIKE '%' || $3 || '%' OR a.reason ILIKE '%' || $3 || '%')
            ORDER BY a.created_at DESC
            LIMIT $4 OFFSET $5
            "#,
        )
        .bind(user_id)
        .bind(status)
        .bind(q)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok((rows.into_iter().map(map_appeal).collect(), total))
    }

    pub async fn set_appeal_status(
        &self,
        tx: Option<&mut Transaction<'_, Postgres>>,
        appeal_id: Uuid,
        status: &str,
        reviewer_id: Option<Uuid>,
        note: Option<&str>,
    ) -> Result<bool, sqlx::Error> {
        let query = r#"
            UPDATE appeals
            SET status = $2,
                reviewer_id = $3,
                review_note = $4,
                reviewed_at = CASE WHEN $2 IN ('approved', 'rejected') THEN now() ELSE reviewed_at END
            WHERE id = $1 AND status IN ('pending', 'reviewing')
        "#;
        let result = match tx {
            Some(tx) => sqlx::query(query)
                .bind(appeal_id)
                .bind(status)
                .bind(reviewer_id)
                .bind(note)
                .execute(&mut **tx)
                .await?,
            None => sqlx::query(query)
                .bind(appeal_id)
                .bind(status)
                .bind(reviewer_id)
                .bind(note)
                .execute(&self.pool)
                .await?,
        };
        Ok(result.rows_affected() == 1)
    }

    pub async fn insert_appeal_event(
        &self,
        appeal_id: Uuid,
        actor_type: &str,
        actor_id: Option<Uuid>,
        action: &str,
        note: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO appeal_events (appeal_id, actor_type, actor_id, action, note)
            VALUES ($1, $2, $3, $4, $5)
            "#,
        )
        .bind(appeal_id)
        .bind(actor_type)
        .bind(actor_id)
        .bind(action)
        .bind(note)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn count_appeals_for_sanction(
        &self,
        sanction_id: Uuid,
    ) -> Result<i64, sqlx::Error> {
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM appeals WHERE sanction_id = $1",
        )
        .bind(sanction_id)
        .fetch_one(&self.pool)
        .await
    }

    // ------------------------------------------------------------------
    // Rules
    // ------------------------------------------------------------------

    pub async fn list_rules(
        &self,
        enabled_only: bool,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<RuleItem>, i64), sqlx::Error> {
        let total = if enabled_only {
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM moderation_rules WHERE enabled = true")
                .fetch_one(&self.pool)
                .await?
        } else {
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM moderation_rules")
                .fetch_one(&self.pool)
                .await?
        };
        let rows = sqlx::query_as::<_, RuleRow>(
            r#"
            SELECT id, name, rule_type, target_type, priority, enabled, risk_score, action, config, hit_count, created_at, updated_at
            FROM moderation_rules
            WHERE ($1::boolean OR enabled = true)
            ORDER BY priority DESC, created_at ASC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(enabled_only)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;
        Ok((rows.into_iter().map(map_rule).collect(), total))
    }

    pub async fn get_rule(&self, rule_id: Uuid) -> Result<Option<RuleItem>, sqlx::Error> {
        let row = sqlx::query_as::<_, RuleRow>(
            r#"
            SELECT id, name, rule_type, target_type, priority, enabled, risk_score, action, config, hit_count, created_at, updated_at
            FROM moderation_rules
            WHERE id = $1
            "#,
        )
        .bind(rule_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(map_rule))
    }

    pub async fn create_rule(
        &self,
        name: &str,
        rule_type: &str,
        target_type: &str,
        priority: i32,
        enabled: bool,
        risk_score: i32,
        action: &str,
        config: Value,
        created_by: Uuid,
    ) -> Result<Uuid, sqlx::Error> {
        sqlx::query_scalar::<_, Uuid>(
            r#"
            INSERT INTO moderation_rules (
                name, rule_type, target_type, priority, enabled, risk_score, action, config, created_by
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING id
            "#,
        )
        .bind(name)
        .bind(rule_type)
        .bind(target_type)
        .bind(priority)
        .bind(enabled)
        .bind(risk_score)
        .bind(action)
        .bind(config)
        .bind(created_by)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn update_rule(
        &self,
        rule_id: Uuid,
        name: Option<&str>,
        rule_type: Option<&str>,
        target_type: Option<&str>,
        priority: Option<i32>,
        enabled: Option<bool>,
        risk_score: Option<i32>,
        action: Option<&str>,
        config: Option<Value>,
    ) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            r#"
            UPDATE moderation_rules
            SET name = COALESCE($2, name),
                rule_type = COALESCE($3, rule_type),
                target_type = COALESCE($4, target_type),
                priority = COALESCE($5, priority),
                enabled = COALESCE($6, enabled),
                risk_score = COALESCE($7, risk_score),
                action = COALESCE($8, action),
                config = COALESCE($9, config)
            WHERE id = $1
            "#,
        )
        .bind(rule_id)
        .bind(name)
        .bind(rule_type)
        .bind(target_type)
        .bind(priority)
        .bind(enabled)
        .bind(risk_score)
        .bind(action)
        .bind(config)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() == 1)
    }

    pub async fn delete_rule(&self, rule_id: Uuid) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("DELETE FROM moderation_rules WHERE id = $1")
            .bind(rule_id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() == 1)
    }

    pub async fn insert_rule_hit(
        &self,
        rule_id: Uuid,
        target_type: &str,
        target_id: Option<Uuid>,
        user_id: Option<Uuid>,
        snippet: Option<&str>,
        risk_score: i32,
        action: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO moderation_rule_hits (
                rule_id, target_type, target_id, user_id, content_snippet, risk_score, action
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(rule_id)
        .bind(target_type)
        .bind(target_id)
        .bind(user_id)
        .bind(snippet)
        .bind(risk_score)
        .bind(action)
        .execute(&self.pool)
        .await?;
        sqlx::query(
            "UPDATE moderation_rules SET hit_count = hit_count + 1 WHERE id = $1",
        )
        .bind(rule_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn list_rule_hits_by_target(
        &self,
        target_type: &str,
        target_id: Uuid,
    ) -> Result<Vec<RuleHitItem>, sqlx::Error> {
        let rows = sqlx::query_as::<_, RuleHitRow>(
            r#"
            SELECT h.id, h.rule_id, r.name AS rule_name, r.rule_type AS rule_type,
                   h.target_type, h.risk_score, h.action, h.content_snippet, h.created_at
            FROM moderation_rule_hits h
            JOIN moderation_rules r ON r.id = h.rule_id
            WHERE h.target_type = $1 AND h.target_id = $2
            ORDER BY h.created_at DESC
            "#,
        )
        .bind(target_type)
        .bind(target_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|row| RuleHitItem {
                id: row.id,
                rule_id: row.rule_id,
                rule_name: row.rule_name,
                rule_type: row.rule_type,
                target_type: row.target_type,
                risk_score: row.risk_score,
                action: row.action,
                content_snippet: row.content_snippet,
                created_at: row.created_at,
            })
            .collect())
    }

    // ------------------------------------------------------------------
    // Background maintenance
    // ------------------------------------------------------------------

    /// Expire sanctions whose window has passed. Returns (expired ids, restored user ids).
    pub async fn expire_due_sanctions(
        &self,
    ) -> Result<(Vec<Uuid>, Vec<Uuid>), sqlx::Error> {
        let rows: Vec<(Uuid, Uuid)> = sqlx::query_as(
            r#"
            UPDATE user_sanctions s
            SET status = 'expired'
            WHERE s.status = 'active'
              AND s.is_permanent = false
              AND s.ends_at <= now()
            RETURNING s.id, s.user_id
            "#,
        )
        .fetch_all(&self.pool)
        .await?;
        let mut restored = Vec::new();
        for (_, user_id) in &rows {
            let mut tx = self.pool.begin().await?;
            let still_banned = self.has_active_account_ban(&mut tx, *user_id).await?;
            let user = self.lock_target_user(&mut tx, *user_id).await?;
            if !still_banned {
                if let Some(user) = user {
                    if user.status != "active" {
                        self.set_user_status(&mut tx, *user_id, "active", false).await?;
                        restored.push(*user_id);
                    }
                }
            }
            tx.commit().await?;
        }
        Ok((rows.iter().map(|(id, _)| *id).collect(), restored))
    }

    pub async fn sanctions_expiring_within(
        &self,
        hours: i64,
    ) -> Result<Vec<(Uuid, Uuid, SanctionType, Option<DateTime<Utc>>)>, sqlx::Error> {
        sqlx::query_as::<_, (Uuid, Uuid, String, Option<DateTime<Utc>>)>(
            r#"
            SELECT id, user_id, sanction_type, ends_at
            FROM user_sanctions
            WHERE status = 'active'
              AND is_permanent = false
              AND ends_at > now()
              AND ends_at <= now() + make_interval(hours => $1)
            "#,
        )
        .bind(hours)
        .fetch_all(&self.pool)
        .await
        .map(|rows| {
            rows.into_iter()
                .map(|(id, user_id, kind, ends_at)| {
                    (id, user_id, kind.parse().unwrap_or(SanctionType::Warning), ends_at)
                })
                .collect()
        })
    }

    pub async fn purge_snapshots_older_than(&self, days: i64) -> Result<i64, sqlx::Error> {
        let result = sqlx::query(
            "DELETE FROM content_snapshots WHERE created_at < now() - make_interval(days => $1)",
        )
        .bind(days)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() as i64)
    }

    pub async fn purge_rule_hits_older_than(&self, days: i64) -> Result<i64, sqlx::Error> {
        let result = sqlx::query(
            "DELETE FROM moderation_rule_hits WHERE created_at < now() - make_interval(days => $1)",
        )
        .bind(days)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() as i64)
    }

    /// All active staff users (moderators and above), for realtime fan-out.
    pub async fn list_staff_user_ids(&self) -> Result<Vec<Uuid>, sqlx::Error> {
        sqlx::query_scalar::<_, Uuid>(
            r#"
            SELECT u.id
            FROM users u
            JOIN roles r ON r.id = u.role_id
            WHERE u.status = 'active'
              AND r.priority >= 20
            "#,
        )
        .fetch_all(&self.pool)
        .await
    }

    // ------------------------------------------------------------------
    // Governance metrics
    // ------------------------------------------------------------------

    pub async fn governance_metrics(&self) -> Result<GovernanceMetrics, sqlx::Error> {
        let reports_today = sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM reports WHERE created_at >= date_trunc('day', now())",
        )
        .fetch_one(&self.pool)
        .await?;
        let reports_pending = sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM reports WHERE status IN ('open', 'reviewing')",
        )
        .fetch_one(&self.pool)
        .await?;
        let reports_resolved_7d = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT count(*) FROM reports
            WHERE status IN ('resolved', 'rejected', 'duplicate')
              AND handled_at >= now() - interval '7 days'
            "#,
        )
        .fetch_one(&self.pool)
        .await?;
        let avg_review_hours = sqlx::query_scalar::<_, Option<f64>>(
            r#"
            SELECT avg(extract(epoch FROM (handled_at - created_at)) / 3600.0)
            FROM reports
            WHERE handled_at IS NOT NULL
            "#,
        )
        .fetch_one(&self.pool)
        .await?;
        let queue_backlog = sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM moderation_cases WHERE status IN ('open', 'reviewing')",
        )
        .fetch_one(&self.pool)
        .await?;

        let reports_by_reason = sqlx::query_as::<_, (String, i64)>(
            r#"
            SELECT reason, count(*) FROM reports
            GROUP BY reason ORDER BY count(*) DESC LIMIT 12
            "#,
        )
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|(label, count)| CountItem { label, count })
        .collect();

        let reports_by_target = sqlx::query_as::<_, (String, i64)>(
            r#"
            SELECT target_type, count(*) FROM reports
            GROUP BY target_type ORDER BY count(*) DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|(label, count)| CountItem { label, count })
        .collect();

        let auto_hits_7d = sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM moderation_rule_hits WHERE created_at >= now() - interval '7 days'",
        )
        .fetch_one(&self.pool)
        .await?;
        let auto_hidden = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT count(*) FROM moderation_actions
            WHERE action = 'hide' AND metadata->>'source' = 'auto'
            "#,
        )
        .fetch_one(&self.pool)
        .await?;
        let manual_restores = sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM moderation_actions WHERE action = 'restore'",
        )
        .fetch_one(&self.pool)
        .await?;

        let (warnings_total, mutes_total, suspensions_total, bans_total, sanctions_active) = {
            let row = sqlx::query_as::<_, (i64, i64, i64, i64, i64)>(
                r#"
                SELECT
                    count(*) FILTER (WHERE sanction_type = 'warning'),
                    count(*) FILTER (WHERE sanction_type = 'mute'),
                    count(*) FILTER (WHERE sanction_type = 'suspension'),
                    count(*) FILTER (WHERE sanction_type = 'ban'),
                    count(*) FILTER (WHERE status = 'active')
                FROM user_sanctions
                "#,
            )
            .fetch_one(&self.pool)
            .await?;
            row
        };

        let (appeals_total, appeals_pending, appeals_approved, appeals_rejected) = {
            let row = sqlx::query_as::<_, (i64, i64, i64, i64)>(
                r#"
                SELECT
                    count(*),
                    count(*) FILTER (WHERE status = 'pending'),
                    count(*) FILTER (WHERE status = 'approved'),
                    count(*) FILTER (WHERE status = 'rejected')
                FROM appeals
                "#,
            )
            .fetch_one(&self.pool)
            .await?;
            row
        };

        let moderator_actions_7d = sqlx::query_as::<_, (String, i64)>(
            r#"
            SELECT u.username, count(*) AS actions
            FROM moderation_actions a
            JOIN users u ON u.id = a.operator_id
            WHERE a.created_at >= now() - interval '7 days'
              AND a.operator_id IS NOT NULL
            GROUP BY u.username
            ORDER BY actions DESC
            LIMIT 10
            "#,
        )
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|(label, count)| CountItem { label, count })
        .collect();

        let daily_rows = sqlx::query_as::<_, (chrono::NaiveDate, i64, i64, i64)>(
            r#"
            SELECT d::date,
                   count(DISTINCT r.id) FILTER (WHERE r.id IS NOT NULL),
                   count(DISTINCT a.id) FILTER (WHERE a.id IS NOT NULL),
                   count(DISTINCT s.id) FILTER (WHERE s.id IS NOT NULL)
            FROM generate_series(current_date - 13, current_date, interval '1 day') AS d
            LEFT JOIN reports r ON r.created_at::date = d::date
            LEFT JOIN moderation_actions a ON a.created_at::date = d::date
            LEFT JOIN user_sanctions s ON s.created_at::date = d::date
            GROUP BY d
            ORDER BY d
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(GovernanceMetrics {
            reports_today,
            reports_pending,
            reports_resolved_7d,
            avg_review_hours,
            reports_by_reason,
            reports_by_target,
            auto_hits_7d,
            auto_hidden,
            manual_restores,
            warnings_total,
            mutes_total,
            suspensions_total,
            bans_total,
            sanctions_active,
            appeals_total,
            appeals_pending,
            appeals_approved,
            appeals_rejected,
            queue_backlog,
            moderator_actions_7d,
            daily_14d: daily_rows
                .into_iter()
                .map(|(date, reports, actions, sanctions)| DailyMetric {
                    date: date.to_string(),
                    reports,
                    actions,
                    sanctions,
                })
                .collect(),
        })
    }
}

// ---------------------------------------------------------------------------
// Row mappers
// ---------------------------------------------------------------------------

fn map_report(row: ReportRow) -> ReportItemV2 {
    ReportItemV2 {
        id: row.id,
        reporter_id: row.reporter_id,
        reporter_username: row.reporter_username,
        target_type: row.target_type.parse().unwrap_or(ReportTargetType::Topic),
        target_id: row.target_id,
        reason: row.reason,
        details: row.details,
        status: row.status.parse().unwrap_or(ReportStatus::Open),
        priority: row.priority.parse().unwrap_or(ReportPriority::Normal),
        risk_score: row.risk_score,
        case_id: row.case_id,
        duplicate_of: row.duplicate_of,
        handler_id: row.handler_id,
        handler_username: row.handler_username,
        resolution_note: row.resolution_note,
        handled_at: row.handled_at,
        cancelled_at: row.cancelled_at,
        created_at: row.created_at,
        updated_at: row.updated_at,
    }
}

fn map_case(row: CaseRow) -> CaseItem {
    CaseItem {
        id: row.id,
        target_type: row.target_type.parse().unwrap_or(ReportTargetType::Topic),
        target_id: row.target_id,
        status: row.status.parse().unwrap_or(CaseStatus::Open),
        priority: row.priority.parse().unwrap_or(ReportPriority::Normal),
        risk_score: row.risk_score,
        source: row.source.parse().unwrap_or(CaseSource::Report),
        assignee_id: row.assignee_id,
        assignee_username: row.assignee_username,
        report_count: 0,
        content_summary: None,
        author_id: None,
        author_username: None,
        opened_by: row.opened_by,
        opened_at: row.opened_at,
        created_at: row.created_at,
        updated_at: row.updated_at,
        closed_at: row.closed_at,
    }
}

fn map_sanction(row: SanctionRow) -> SanctionItem {
    SanctionItem {
        id: row.id,
        user_id: row.user_id,
        username: row.username,
        sanction_type: row.sanction_type.parse().unwrap_or(SanctionType::Warning),
        reason: row.reason,
        user_visible_reason: row.user_visible_reason,
        internal_note: row.internal_note,
        restrictions: row.restrictions,
        starts_at: row.starts_at,
        ends_at: row.ends_at,
        is_permanent: row.is_permanent,
        status: row.status.parse().unwrap_or(SanctionStatus::Scheduled),
        issued_by: row.issued_by,
        issuer_username: row.issuer_username,
        case_id: row.case_id,
        report_id: row.report_id,
        related_content_type: row.related_content_type,
        related_content_id: row.related_content_id,
        revoked_by: row.revoked_by,
        revoked_at: row.revoked_at,
        revoke_reason: row.revoke_reason,
        created_at: row.created_at,
        updated_at: row.updated_at,
    }
}

fn map_appeal(row: AppealRow) -> AppealItem {
    AppealItem {
        id: row.id,
        user_id: row.user_id,
        username: row.username,
        appeal_type: row.appeal_type.parse().unwrap_or(AppealType::Sanction),
        sanction_id: row.sanction_id,
        content_type: row.content_type,
        content_id: row.content_id,
        reason: row.reason,
        details: row.details,
        evidence: row
            .evidence
            .as_array()
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| item.as_str().and_then(|value| Uuid::parse_str(value).ok()))
                    .collect()
            })
            .unwrap_or_default(),
        status: row.status.parse().unwrap_or(AppealStatus::Pending),
        reviewer_id: row.reviewer_id,
        reviewer_username: row.reviewer_username,
        review_note: row.review_note,
        reviewed_at: row.reviewed_at,
        created_at: row.created_at,
        updated_at: row.updated_at,
    }
}

fn map_rule(row: RuleRow) -> RuleItem {
    RuleItem {
        id: row.id,
        name: row.name,
        rule_type: row.rule_type.parse().unwrap_or(RuleType::Keyword),
        target_type: row.target_type,
        priority: row.priority,
        enabled: row.enabled,
        risk_score: row.risk_score,
        action: row.action.parse().unwrap_or(RuleAction::Flag),
        config: row.config,
        hit_count: row.hit_count,
        created_at: row.created_at,
        updated_at: row.updated_at,
    }
}
