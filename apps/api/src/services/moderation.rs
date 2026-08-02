//! Phase 13: moderation service — reports, cases, content governance,
//! sanctions, appeals, auto-moderation, enforcement, metrics, maintenance.

use chrono::{Duration, Utc};
use redis::{aio::ConnectionManager, AsyncCommands};
use serde::Serialize;
use serde_json::json;
use sha2::{Digest, Sha256};
use thiserror::Error;
use uuid::Uuid;

use crate::models::{
    AppealItem, AppealListQuery, AppealStatus, AppealType, AuthenticatedPrincipal, CaseDetail,
    CaseItem, CaseQuery, CaseSource, CaseStatus, ContentActionResult, CreateAppealRequest,
    CreateReportRequestV2, CreateSanctionRequest, GovernanceMetrics, ModerationActionKind,
    ModerationReportQuery, NoteRequest, Paginated, PaginationMeta, ReportItemV2, ReportPriority,
    ReportStatus, ReportTargetType, ResolveReportRequestV2, ReviewAppealRequest,
    RevokeSanctionRequest, RuleAction, RuleItem, RuleListQuery, RuleRequest, RuleType,
    SanctionItem, SanctionListQuery, SanctionStatus, SanctionType,
    PERMISSION_MODERATION_APPEAL_READ, PERMISSION_MODERATION_APPEAL_REVIEW,
    PERMISSION_MODERATION_AUDIT_READ, PERMISSION_MODERATION_CONTENT_DELETE,
    PERMISSION_MODERATION_CONTENT_HIDE, PERMISSION_MODERATION_CONTENT_RESTORE,
    PERMISSION_MODERATION_METRICS_READ, PERMISSION_MODERATION_REPORT_ASSIGN,
    PERMISSION_MODERATION_REPORT_READ, PERMISSION_MODERATION_REPORT_REVIEW,
    PERMISSION_MODERATION_RULE_MANAGE, PERMISSION_MODERATION_SANCTION_REVOKE,
    PERMISSION_MODERATION_TOPIC_LOCK, PERMISSION_MODERATION_TOPIC_MOVE,
    PERMISSION_MODERATION_USER_BAN, PERMISSION_MODERATION_USER_MUTE,
    PERMISSION_MODERATION_USER_SUSPEND, PERMISSION_MODERATION_USER_WARN, PERMISSION_REPORT_CREATE,
    PERMISSION_TOPIC_PIN, RESTRICTION_NO_COMMENTS, RESTRICTION_NO_REPORTS, RESTRICTION_NO_TOPICS,
    RESTRICTION_NO_UPLOADS, ROLE_SUPER_ADMINISTRATOR,
};
use crate::realtime::RealtimeBus;
use crate::repositories::{AdminRepository, CategoryRepository, ModerationRepository};
use crate::services::{
    AdminAuditContext, AuthorizationService, MetricsRegistry, NotificationService,
};

const RULES_CACHE_KEY: &str = "mod:rules:v1";
const RULES_CACHE_TTL_SECS: u64 = 60;
const REPORT_RATE_WINDOW_SECS: u64 = 60;
const REPORT_RATE_LIMIT: u64 = 5;
const REPORT_DEDUP_WINDOW_HOURS: i64 = 24;
const MAX_APPEALS_PER_SANCTION: i64 = 2;
const DEFAULT_PAGE_SIZE: u32 = 20;
const MAX_PAGE_SIZE: u32 = 100;
const MAX_PAGE: u32 = 1_000_000;

#[derive(Clone)]
pub struct ModerationService {
    repository: ModerationRepository,
    categories: CategoryRepository,
    notifications: NotificationService,
    admin_logs: AdminRepository,
    authorization: AuthorizationService,
    realtime: RealtimeBus,
    redis: ConnectionManager,
    metrics: MetricsRegistry,
}

#[derive(Debug, Error)]
pub enum ModerationError {
    #[error("invalid moderation input: {0}")]
    Validation(&'static str),
    #[error("resource not found")]
    NotFound,
    #[error("permission denied")]
    Forbidden,
    #[error("rate limit exceeded")]
    RateLimited,
    #[error("operation conflicts with current state: {0}")]
    Conflict(&'static str),
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

#[derive(Clone, Debug)]
pub struct ScreeningHit {
    pub rule_id: Uuid,
    pub rule_type: RuleType,
    pub action: RuleAction,
    pub risk_score: i32,
}

#[derive(Clone, Debug, Default)]
pub struct ScreeningDecision {
    pub action: RuleAction,
    pub risk_score: i32,
    pub hits: Vec<ScreeningHit>,
}

impl ScreeningDecision {
    pub fn is_allowed(&self) -> bool {
        self.action == RuleAction::Allow || self.action == RuleAction::Flag
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct BatchResultItem {
    pub id: Uuid,
    pub ok: bool,
    pub code: Option<&'static str>,
}

#[derive(Clone, Debug, Serialize)]
pub struct BatchResult {
    pub succeeded: Vec<Uuid>,
    pub failed: Vec<BatchResultItem>,
}

impl ModerationService {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        repository: ModerationRepository,
        categories: CategoryRepository,
        notifications: NotificationService,
        admin_logs: AdminRepository,
        authorization: AuthorizationService,
        realtime: RealtimeBus,
        redis: ConnectionManager,
        metrics: MetricsRegistry,
    ) -> Self {
        Self {
            repository,
            categories,
            notifications,
            admin_logs,
            authorization,
            realtime,
            redis,
            metrics,
        }
    }

    pub fn metrics(&self) -> &MetricsRegistry {
        &self.metrics
    }

    // ------------------------------------------------------------------
    // ④ Reports
    // ------------------------------------------------------------------

    pub async fn create_report(
        &self,
        principal: &AuthenticatedPrincipal,
        request: CreateReportRequestV2,
    ) -> Result<ReportItemV2, ModerationError> {
        require(principal, PERMISSION_REPORT_CREATE)?;
        self.enforce_report_creation(principal.user_id).await?;
        self.enforce_report_rate_limit(principal.user_id).await?;

        let target_type = request.target_type.as_str();
        let (reason_code, risk_score) = match request.reason_code {
            Some(code) => (code.as_str().to_owned(), code.risk_score()),
            None => {
                let reason = request
                    .reason
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .ok_or(ModerationError::Validation("report reason is required"))?;
                if !(3..=500).contains(&reason.chars().count()) {
                    return Err(ModerationError::Validation(
                        "reason must contain between 3 and 500 characters",
                    ));
                }
                (reason.to_owned(), 20)
            }
        };
        let details = request
            .details
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty());
        if details
            .as_ref()
            .is_some_and(|value| value.chars().count() > 2000)
        {
            return Err(ModerationError::Validation("details are too long"));
        }

        // Self-reporting is forbidden; target must exist and be visible.
        self.verify_report_target(principal, request.target_type, request.target_id)
            .await?;

        // Prevent repeated reports of the same target within the window.
        if let Some(existing) = self
            .repository
            .find_recent_report(
                principal.user_id,
                target_type,
                request.target_id,
                REPORT_DEDUP_WINDOW_HOURS,
            )
            .await
            .map_err(internal)?
        {
            return Ok(existing);
        }

        let priority = if request.reason_code.is_some() {
            reason_code_priority(&reason_code)
        } else {
            ReportPriority::Normal
        };

        let report = self
            .repository
            .create_report(
                principal.user_id,
                target_type,
                request.target_id,
                &reason_code,
                details.as_deref(),
                priority.as_str(),
                risk_score as i32,
            )
            .await
            .map_err(internal)?;

        // Link or create a moderation case for the target.
        let case_id = self
            .repository
            .find_open_case(target_type, request.target_id)
            .await
            .map_err(internal)?;
        let case_id = match case_id {
            Some(id) => {
                self.repository
                    .bump_case_priority(id, priority.as_str(), risk_score as i32)
                    .await
                    .map_err(internal)?;
                id
            }
            None => self
                .repository
                .create_case(
                    target_type,
                    request.target_id,
                    priority.as_str(),
                    risk_score as i32,
                    CaseSource::Report.as_str(),
                    Some(principal.user_id),
                )
                .await
                .map_err(internal)?,
        };
        self.repository
            .link_report_to_case(report.id, case_id)
            .await
            .map_err(internal)?;
        self.repository
            .insert_report_event(
                report.id,
                "reporter",
                Some(principal.user_id),
                "created",
                None,
            )
            .await
            .map_err(internal)?;

        self.metrics
            .inc("moderation_reports_total", &[("status", "open")]);
        self.notify_staff_realtime(
            "moderation.case.created",
            json!({
                "case_id": case_id,
                "target_type": target_type,
                "target_id": request.target_id,
                "priority": priority.as_str(),
            }),
        )
        .await;
        Ok(report)
    }

    async fn verify_report_target(
        &self,
        principal: &AuthenticatedPrincipal,
        target_type: crate::models::ReportTargetType,
        target_id: Uuid,
    ) -> Result<(), ModerationError> {
        match target_type {
            crate::models::ReportTargetType::Topic => {
                let topic = self
                    .repository
                    .get_topic(target_id)
                    .await
                    .map_err(internal)?
                    .ok_or(ModerationError::NotFound)?;
                if topic.author_id == principal.user_id {
                    return Err(ModerationError::Validation(
                        "you cannot report your own content",
                    ));
                }
                if topic.status != "published" {
                    return Err(ModerationError::Validation(
                        "the target content is no longer reportable",
                    ));
                }
            }
            crate::models::ReportTargetType::Comment => {
                let comment = self
                    .repository
                    .get_comment(target_id)
                    .await
                    .map_err(internal)?
                    .ok_or(ModerationError::NotFound)?;
                if comment.author_id == principal.user_id {
                    return Err(ModerationError::Validation(
                        "you cannot report your own content",
                    ));
                }
                if comment.status != "published" {
                    return Err(ModerationError::Validation(
                        "the target content is no longer reportable",
                    ));
                }
            }
            crate::models::ReportTargetType::User => {
                let user = self
                    .repository
                    .get_user(target_id)
                    .await
                    .map_err(internal)?
                    .ok_or(ModerationError::NotFound)?;
                if user.id == principal.user_id {
                    return Err(ModerationError::Validation("you cannot report yourself"));
                }
                if user.status != "active" {
                    return Err(ModerationError::Validation(
                        "the target user is no longer reportable",
                    ));
                }
            }
        }
        Ok(())
    }

    pub async fn list_my_reports(
        &self,
        principal: &AuthenticatedPrincipal,
        page: Option<u32>,
        page_size: Option<u32>,
    ) -> Result<Paginated<ReportItemV2>, ModerationError> {
        require(principal, PERMISSION_REPORT_CREATE)?;
        let (page, page_size, limit, offset) = page_bounds(page, page_size)?;
        let (items, total) = self
            .repository
            .list_reports_by_user(principal.user_id, limit, offset)
            .await
            .map_err(internal)?;
        Ok(paginate(items, page, page_size, total))
    }

    pub async fn get_my_report(
        &self,
        principal: &AuthenticatedPrincipal,
        report_id: Uuid,
    ) -> Result<ReportItemV2, ModerationError> {
        require(principal, PERMISSION_REPORT_CREATE)?;
        let report = self
            .repository
            .get_report(report_id)
            .await
            .map_err(internal)?
            .ok_or(ModerationError::NotFound)?;
        if report.reporter_id == principal.user_id
            || principal.has_permission(PERMISSION_MODERATION_REPORT_READ)
        {
            Ok(report)
        } else {
            Err(ModerationError::NotFound)
        }
    }

    pub async fn cancel_report(
        &self,
        principal: &AuthenticatedPrincipal,
        report_id: Uuid,
    ) -> Result<ReportItemV2, ModerationError> {
        require(principal, PERMISSION_REPORT_CREATE)?;
        let report = self
            .repository
            .get_report(report_id)
            .await
            .map_err(internal)?
            .ok_or(ModerationError::NotFound)?;
        if report.reporter_id != principal.user_id {
            return Err(ModerationError::NotFound);
        }
        if report.status != ReportStatus::Open {
            return Err(ModerationError::Conflict(
                "only open reports can be cancelled",
            ));
        }
        let changed = self
            .repository
            .update_report_status(
                None,
                report_id,
                None,
                ReportStatus::Cancelled.as_str(),
                None,
                None,
            )
            .await
            .map_err(internal)?;
        if !changed {
            return Err(ModerationError::Conflict("report is already handled"));
        }
        self.repository
            .insert_report_event(
                report_id,
                "reporter",
                Some(principal.user_id),
                "cancelled",
                None,
            )
            .await
            .map_err(internal)?;
        self.repository
            .get_report(report_id)
            .await
            .map_err(internal)?
            .ok_or(ModerationError::NotFound)
    }

    // ------------------------------------------------------------------
    // ⑤ Review queue (reports + cases)
    // ------------------------------------------------------------------

    pub async fn list_reports(
        &self,
        principal: &AuthenticatedPrincipal,
        query: ModerationReportQuery,
    ) -> Result<Paginated<ReportItemV2>, ModerationError> {
        require(principal, PERMISSION_MODERATION_REPORT_READ)?;
        let (page, page_size, limit, offset) = page_bounds(query.page, query.page_size)?;
        let q = normalize_search(query.q)?;
        let status = normalize_filter(query.status)?;
        let target_type = normalize_filter(query.target_type)?;
        let reason = normalize_filter(query.reason)?;
        let priority = normalize_filter(query.priority)?;
        if let Some(value) = priority.as_deref() {
            if value.parse::<ReportPriority>().is_err() {
                return Err(ModerationError::Validation("invalid priority filter"));
            }
        }
        let (assignee_id, unassigned) =
            resolve_assignee_filter(query.assignee.as_deref(), principal)?;
        if let (Some(from), Some(to)) = (query.from, query.to) {
            if from > to {
                return Err(ModerationError::Validation("from must be before to"));
            }
        }
        let (items, total) = self
            .repository
            .list_reports(
                q.as_deref(),
                status.as_deref(),
                target_type.as_deref(),
                reason.as_deref(),
                priority.as_deref(),
                assignee_id,
                unassigned,
                query.from,
                query.to,
                limit,
                offset,
            )
            .await
            .map_err(internal)?;
        Ok(paginate(items, page, page_size, total))
    }

    pub async fn get_report_detail(
        &self,
        principal: &AuthenticatedPrincipal,
        report_id: Uuid,
    ) -> Result<(ReportItemV2, Option<CaseDetail>), ModerationError> {
        require(principal, PERMISSION_MODERATION_REPORT_READ)?;
        let report = self
            .repository
            .get_report(report_id)
            .await
            .map_err(internal)?
            .ok_or(ModerationError::NotFound)?;
        let case = match report.case_id {
            Some(case_id) => Some(self.case_detail(principal, case_id).await?),
            None => None,
        };
        Ok((report, case))
    }

    pub async fn list_cases(
        &self,
        principal: &AuthenticatedPrincipal,
        query: CaseQuery,
    ) -> Result<Paginated<CaseItem>, ModerationError> {
        require(principal, PERMISSION_MODERATION_REPORT_READ)?;
        let (page, page_size, limit, offset) = page_bounds(query.page, query.page_size)?;
        let q = normalize_search(query.q)?;
        let status = normalize_filter(query.status)?;
        let priority = normalize_filter(query.priority)?;
        let source = normalize_filter(query.source)?;
        let target_type = normalize_filter(query.target_type)?;
        let (assignee_id, unassigned) =
            resolve_assignee_filter(query.assignee.as_deref(), principal)?;
        let (rows, total) = self
            .repository
            .list_cases(
                q.as_deref(),
                status.as_deref(),
                priority.as_deref(),
                source.as_deref(),
                target_type.as_deref(),
                assignee_id,
                unassigned,
                limit,
                offset,
            )
            .await
            .map_err(internal)?;
        let items = rows
            .into_iter()
            .map(|row| CaseItem {
                id: row.id,
                target_type: row.target_type.parse().unwrap_or(ReportTargetType::Topic),
                target_id: row.target_id,
                status: row.status.parse().unwrap_or(CaseStatus::Open),
                priority: row.priority.parse().unwrap_or(ReportPriority::Normal),
                risk_score: row.risk_score,
                source: row.source.parse().unwrap_or(CaseSource::Report),
                assignee_id: row.assignee_id,
                assignee_username: row.assignee_username,
                report_count: row.report_count,
                content_summary: row.content_summary,
                author_id: row.author_id,
                author_username: row.author_username,
                opened_by: row.opened_by,
                opened_at: row.opened_at,
                created_at: row.created_at,
                updated_at: row.updated_at,
                closed_at: row.closed_at,
            })
            .collect();
        Ok(paginate(items, page, page_size, total))
    }

    pub async fn case_detail(
        &self,
        principal: &AuthenticatedPrincipal,
        case_id: Uuid,
    ) -> Result<CaseDetail, ModerationError> {
        require(principal, PERMISSION_MODERATION_REPORT_READ)?;
        let case = self
            .repository
            .get_case(case_id)
            .await
            .map_err(internal)?
            .ok_or(ModerationError::NotFound)?;
        let related_reports = self
            .repository
            .list_reports_by_case(case_id)
            .await
            .map_err(internal)?;
        let actions = self
            .repository
            .list_actions_by_case(case_id)
            .await
            .map_err(internal)?;
        let snapshots = self
            .repository
            .list_snapshots_by_case(case_id)
            .await
            .map_err(internal)?;
        let notes = self
            .repository
            .list_notes_by_case(case_id)
            .await
            .map_err(internal)?;
        let rule_hits = self
            .repository
            .list_rule_hits_by_target(case.target_type.as_str(), case.target_id)
            .await
            .map_err(internal)?;
        let author_violations = match self
            .repository
            .get_target_author(case.target_type.as_str(), case.target_id)
            .await
            .map_err(internal)?
        {
            Some(author_id) => self.user_violations(author_id).await,
            None => Vec::new(),
        };
        Ok(CaseDetail {
            id: case.id,
            target_type: case.target_type,
            target_id: case.target_id,
            status: case.status,
            priority: case.priority,
            risk_score: case.risk_score,
            source: case.source,
            assignee_id: case.assignee_id,
            assignee_username: case.assignee_username,
            opened_by: case.opened_by,
            opened_at: case.opened_at,
            closed_at: case.closed_at,
            created_at: case.created_at,
            updated_at: case.updated_at,
            related_reports,
            actions,
            snapshots,
            rule_hits,
            notes,
            author_violations,
        })
    }

    pub async fn assign_case(
        &self,
        principal: &AuthenticatedPrincipal,
        case_id: Uuid,
        assignee_id: Option<Uuid>,
    ) -> Result<CaseItem, ModerationError> {
        require(principal, PERMISSION_MODERATION_REPORT_ASSIGN)?;
        let assignee_id = assignee_id.unwrap_or(principal.user_id);
        if !self
            .repository
            .update_case_assignment(case_id, Some(assignee_id), CaseStatus::Reviewing.as_str())
            .await
            .map_err(internal)?
        {
            return Err(ModerationError::Conflict(
                "case is closed or already being handled",
            ));
        }
        let case = self
            .repository
            .get_case(case_id)
            .await
            .map_err(internal)?
            .ok_or(ModerationError::NotFound)?;
        self.repository
            .insert_action(
                None,
                Some(case_id),
                "assign",
                "case",
                case_id,
                Some(CaseStatus::Open.as_str()),
                Some(CaseStatus::Reviewing.as_str()),
                None,
                Some(principal.user_id),
                None,
                json!({}),
            )
            .await
            .map_err(internal)?;
        self.repository
            .insert_report_event_from_case(case_id, principal.user_id, "assigned")
            .await
            .map_err(internal)?;
        Ok(case)
    }

    pub async fn release_case(
        &self,
        principal: &AuthenticatedPrincipal,
        case_id: Uuid,
    ) -> Result<CaseItem, ModerationError> {
        require(principal, PERMISSION_MODERATION_REPORT_ASSIGN)?;
        if !self
            .repository
            .update_case_assignment(case_id, None, CaseStatus::Open.as_str())
            .await
            .map_err(internal)?
        {
            return Err(ModerationError::Conflict(
                "case is closed or already being handled",
            ));
        }
        let case = self
            .repository
            .get_case(case_id)
            .await
            .map_err(internal)?
            .ok_or(ModerationError::NotFound)?;
        self.repository
            .insert_action(
                None,
                Some(case_id),
                "release",
                "case",
                case_id,
                Some(CaseStatus::Reviewing.as_str()),
                Some(CaseStatus::Open.as_str()),
                None,
                Some(principal.user_id),
                None,
                json!({}),
            )
            .await
            .map_err(internal)?;
        self.repository
            .insert_report_event_from_case(case_id, principal.user_id, "released")
            .await
            .map_err(internal)?;
        Ok(case)
    }

    pub async fn transfer_case(
        &self,
        principal: &AuthenticatedPrincipal,
        case_id: Uuid,
        target_assignee: Uuid,
    ) -> Result<CaseItem, ModerationError> {
        require(principal, PERMISSION_MODERATION_REPORT_ASSIGN)?;
        let target = self
            .repository
            .get_user(target_assignee)
            .await
            .map_err(internal)?
            .ok_or(ModerationError::NotFound)?;
        if target.status != "active" {
            return Err(ModerationError::Validation("target assignee is not active"));
        }
        if !self
            .repository
            .update_case_assignment(
                case_id,
                Some(target_assignee),
                CaseStatus::Reviewing.as_str(),
            )
            .await
            .map_err(internal)?
        {
            return Err(ModerationError::Conflict(
                "case is closed or already being handled",
            ));
        }
        let case = self
            .repository
            .get_case(case_id)
            .await
            .map_err(internal)?
            .ok_or(ModerationError::NotFound)?;
        self.repository
            .insert_action(
                None,
                Some(case_id),
                "transfer",
                "case",
                case_id,
                None,
                None,
                None,
                Some(principal.user_id),
                None,
                json!({ "to": target_assignee }),
            )
            .await
            .map_err(internal)?;
        self.repository
            .insert_report_event_from_case(case_id, principal.user_id, "transferred")
            .await
            .map_err(internal)?;
        Ok(case)
    }

    pub async fn close_case(
        &self,
        principal: &AuthenticatedPrincipal,
        case_id: Uuid,
        reason: Option<String>,
    ) -> Result<CaseItem, ModerationError> {
        require(principal, PERMISSION_MODERATION_REPORT_REVIEW)?;
        if !self
            .repository
            .close_case(case_id)
            .await
            .map_err(internal)?
        {
            return Err(ModerationError::Conflict("case is already closed"));
        }
        let reason = reason
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty());
        self.repository
            .insert_action(
                None,
                Some(case_id),
                "close_case",
                "case",
                case_id,
                Some(CaseStatus::Open.as_str()),
                Some(CaseStatus::Closed.as_str()),
                reason.as_deref(),
                Some(principal.user_id),
                None,
                json!({}),
            )
            .await
            .map_err(internal)?;
        self.repository
            .get_case(case_id)
            .await
            .map_err(internal)?
            .ok_or(ModerationError::NotFound)
    }

    pub async fn add_note(
        &self,
        principal: &AuthenticatedPrincipal,
        case_id: Uuid,
        request: NoteRequest,
    ) -> Result<Vec<crate::models::ModerationNoteItem>, ModerationError> {
        require(principal, PERMISSION_MODERATION_REPORT_REVIEW)?;
        let note = request.note.trim().to_owned();
        if note.is_empty() || note.chars().count() > 2000 {
            return Err(ModerationError::Validation(
                "note must contain between 1 and 2000 characters",
            ));
        }
        let _ = self
            .repository
            .get_case(case_id)
            .await
            .map_err(internal)?
            .ok_or(ModerationError::NotFound)?;
        self.repository
            .insert_note(case_id, principal.user_id, &note)
            .await
            .map_err(internal)?;
        self.repository
            .insert_action(
                None,
                Some(case_id),
                "note",
                "case",
                case_id,
                None,
                None,
                None,
                Some(principal.user_id),
                None,
                json!({}),
            )
            .await
            .map_err(internal)?;
        self.repository
            .list_notes_by_case(case_id)
            .await
            .map_err(internal)
    }

    /// Handle a report: resolve / reject / mark duplicate, optionally taking
    /// a content action in the same request.
    pub async fn handle_report(
        &self,
        principal: &AuthenticatedPrincipal,
        report_id: Uuid,
        request: ResolveReportRequestV2,
        audit: &AdminAuditContext,
    ) -> Result<ReportItemV2, ModerationError> {
        require(principal, PERMISSION_MODERATION_REPORT_REVIEW)?;
        let report = self
            .repository
            .get_report(report_id)
            .await
            .map_err(internal)?
            .ok_or(ModerationError::NotFound)?;
        if !matches!(report.status, ReportStatus::Open | ReportStatus::Reviewing) {
            return Err(ModerationError::Conflict("report is already handled"));
        }

        // Optional content action performed as part of resolution.
        if let Some(action) = request.action {
            let action_result = match report.target_type {
                crate::models::ReportTargetType::Topic => {
                    self.topic_action_internal(
                        principal,
                        report.target_id,
                        &action,
                        request.action_reason.clone(),
                        report.case_id,
                        Some(report_id),
                        audit,
                    )
                    .await?
                }
                crate::models::ReportTargetType::Comment => {
                    self.comment_action(
                        principal,
                        report.target_id,
                        crate::models::CaseActionRequest {
                            action,
                            reason: request.action_reason.clone(),
                            case_id: report.case_id,
                            target_category_id: None,
                        },
                        audit,
                    )
                    .await?
                }
                crate::models::ReportTargetType::User => {
                    return Err(ModerationError::Validation(
                        "user targets cannot be handled with a content action",
                    ));
                }
            };
            if action_result.case_id.is_some() {
                // link case to report
                if let Some(case_id) = action_result.case_id {
                    self.repository
                        .link_report_to_case(report_id, case_id)
                        .await
                        .map_err(internal)?;
                }
            }
        }

        let status = ReportStatus::Resolved;
        let note = request
            .resolution_note
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty());
        let changed = self
            .repository
            .update_report_status(
                None,
                report_id,
                Some(principal.user_id),
                status.as_str(),
                note.as_deref(),
                None,
            )
            .await
            .map_err(internal)?;
        if !changed {
            return Err(ModerationError::Conflict("report is already handled"));
        }
        let handled_at = Utc::now();
        self.repository
            .insert_report_event(
                report_id,
                "moderator",
                Some(principal.user_id),
                "resolved",
                note.as_deref(),
            )
            .await
            .map_err(internal)?;
        self.log_report_outcome(principal, &report, "resolved", note.as_deref(), audit)
            .await?;
        self.metrics
            .inc("moderation_reports_total", &[("status", "resolved")]);
        self.metrics
            .observe_review_duration((handled_at - report.created_at).num_seconds() as f64);

        // Auto-close the case when no open reports remain.
        self.maybe_close_case(report.case_id).await?;

        self.notify_reporter(
            report.reporter_id,
            report_id,
            "report_processed",
            "举报处理结果",
            "你的举报已处理完成，感谢你的反馈。",
            json!({
                "report_id": report_id,
                "status": status.as_str(),
                "href": "/profile/reports",
            }),
        )
        .await;
        self.repository
            .get_report(report_id)
            .await
            .map_err(internal)?
            .ok_or(ModerationError::NotFound)
    }

    pub async fn reject_report(
        &self,
        principal: &AuthenticatedPrincipal,
        report_id: Uuid,
        note: Option<String>,
        audit: &AdminAuditContext,
    ) -> Result<ReportItemV2, ModerationError> {
        require(principal, PERMISSION_MODERATION_REPORT_REVIEW)?;
        let report = self
            .repository
            .get_report(report_id)
            .await
            .map_err(internal)?
            .ok_or(ModerationError::NotFound)?;
        if !matches!(report.status, ReportStatus::Open | ReportStatus::Reviewing) {
            return Err(ModerationError::Conflict("report is already handled"));
        }
        let note = note
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty());
        let changed = self
            .repository
            .update_report_status(
                None,
                report_id,
                Some(principal.user_id),
                ReportStatus::Rejected.as_str(),
                note.as_deref(),
                None,
            )
            .await
            .map_err(internal)?;
        if !changed {
            return Err(ModerationError::Conflict("report is already handled"));
        }
        self.repository
            .insert_report_event(
                report_id,
                "moderator",
                Some(principal.user_id),
                "rejected",
                note.as_deref(),
            )
            .await
            .map_err(internal)?;
        self.log_report_outcome(principal, &report, "rejected", note.as_deref(), audit)
            .await?;
        self.metrics
            .inc("moderation_reports_total", &[("status", "rejected")]);
        self.maybe_close_case(report.case_id).await?;
        self.notify_reporter(
            report.reporter_id,
            report_id,
            "report_processed",
            "举报处理结果",
            "你的举报经核实后未被采纳。",
            json!({
                "report_id": report_id,
                "status": "rejected",
                "href": "/profile/reports",
            }),
        )
        .await;
        self.repository
            .get_report(report_id)
            .await
            .map_err(internal)?
            .ok_or(ModerationError::NotFound)
    }

    pub async fn duplicate_report(
        &self,
        principal: &AuthenticatedPrincipal,
        report_id: Uuid,
        duplicate_of: Uuid,
        audit: &AdminAuditContext,
    ) -> Result<ReportItemV2, ModerationError> {
        require(principal, PERMISSION_MODERATION_REPORT_REVIEW)?;
        let report = self
            .repository
            .get_report(report_id)
            .await
            .map_err(internal)?
            .ok_or(ModerationError::NotFound)?;
        let target = self
            .repository
            .get_report(duplicate_of)
            .await
            .map_err(internal)?
            .ok_or(ModerationError::NotFound)?;
        if report.target_type != target.target_type || report.target_id != target.target_id {
            return Err(ModerationError::Validation(
                "duplicate target must refer to the same object",
            ));
        }
        let changed = self
            .repository
            .update_report_status(
                None,
                report_id,
                Some(principal.user_id),
                ReportStatus::Duplicate.as_str(),
                None,
                Some(duplicate_of),
            )
            .await
            .map_err(internal)?;
        if !changed {
            return Err(ModerationError::Conflict("report is already handled"));
        }
        self.repository
            .insert_report_event(
                report_id,
                "moderator",
                Some(principal.user_id),
                "duplicated",
                None,
            )
            .await
            .map_err(internal)?;
        self.log_report_outcome(principal, &report, "duplicate", None, audit)
            .await?;
        self.metrics
            .inc("moderation_reports_total", &[("status", "duplicate")]);
        self.maybe_close_case(report.case_id).await?;
        self.repository
            .get_report(report_id)
            .await
            .map_err(internal)?
            .ok_or(ModerationError::NotFound)
    }

    /// Batch report handling with per-item permission/state checks.
    pub async fn batch_reports(
        &self,
        principal: &AuthenticatedPrincipal,
        items: Vec<crate::models::BatchReportItem>,
        audit: &AdminAuditContext,
    ) -> Result<BatchResult, ModerationError> {
        require(principal, PERMISSION_MODERATION_REPORT_REVIEW)?;
        let mut succeeded = Vec::new();
        let mut failed = Vec::new();
        for item in items {
            let outcome = match item.action.as_str() {
                "resolve" => {
                    self.handle_report(
                        principal,
                        item.id,
                        ResolveReportRequestV2 {
                            action: item.content_action,
                            action_reason: item.action_reason.clone(),
                            resolution_note: item.note.clone(),
                        },
                        audit,
                    )
                    .await
                }
                "reject" => {
                    self.reject_report(principal, item.id, item.note.clone(), audit)
                        .await
                }
                "duplicate" => match item.duplicate_of {
                    Some(target) => {
                        self.duplicate_report(principal, item.id, target, audit)
                            .await
                    }
                    None => Err(ModerationError::Validation("duplicate_of is required")),
                },
                _ => Err(ModerationError::Validation("unknown batch action")),
            };
            match outcome {
                Ok(_) => succeeded.push(item.id),
                Err(error) => failed.push(BatchResultItem {
                    id: item.id,
                    ok: false,
                    code: Some(batch_error_code(&error)),
                }),
            }
        }
        Ok(BatchResult { succeeded, failed })
    }

    async fn log_report_outcome(
        &self,
        principal: &AuthenticatedPrincipal,
        report: &ReportItemV2,
        status: &str,
        note: Option<&str>,
        audit: &AdminAuditContext,
    ) -> Result<(), ModerationError> {
        self.admin_logs
            .insert_log(
                None,
                principal.user_id,
                &format!("moderation.report.{status}"),
                "report",
                Some(report.id),
                &format!("report {} set to {status}", report.id),
                json!({
                    "target_type": report.target_type.as_str(),
                    "target_id": report.target_id,
                    "case_id": report.case_id,
                    "note": note,
                }),
                audit.ip,
                audit.user_agent.as_deref(),
            )
            .await
            .map_err(internal)
    }

    async fn maybe_close_case(&self, case_id: Option<Uuid>) -> Result<(), ModerationError> {
        let Some(case_id) = case_id else {
            return Ok(());
        };
        let open = sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM reports WHERE case_id = $1 AND status IN ('open', 'reviewing')",
        )
        .bind(case_id)
        .fetch_one(self.repository.pool())
        .await
        .map_err(internal)?;
        if open == 0 {
            let _ = self.repository.close_case(case_id).await;
        }
        Ok(())
    }

    async fn notify_reporter(
        &self,
        reporter_id: Uuid,
        report_id: Uuid,
        dedup_suffix: &str,
        title: &'static str,
        content: &'static str,
        metadata: serde_json::Value,
    ) {
        let dedup_key = format!("{dedup_suffix}:{report_id}");
        let _ = self
            .notifications
            .send(crate::repositories::NewNotification {
                user_id: reporter_id,
                actor_id: None,
                notification_type: crate::models::NotificationType::ReportProcessed,
                title,
                content,
                target_type: None,
                target_id: None,
                metadata,
                dedup_key: Some(&dedup_key),
            })
            .await;
    }

    // ------------------------------------------------------------------
    // ⑥ Content governance
    // ------------------------------------------------------------------

    pub async fn topic_action(
        &self,
        principal: &AuthenticatedPrincipal,
        topic_id: Uuid,
        request: crate::models::CaseActionRequest,
        audit: &AdminAuditContext,
    ) -> Result<ContentActionResult, ModerationError> {
        if request.action == ModerationActionKind::MoveCategory {
            return self.topic_move(principal, topic_id, request, audit).await;
        }
        self.topic_action_internal(
            principal,
            topic_id,
            &request.action,
            request.reason,
            request.case_id,
            None,
            audit,
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    async fn topic_action_internal(
        &self,
        principal: &AuthenticatedPrincipal,
        topic_id: Uuid,
        action: &ModerationActionKind,
        reason: Option<String>,
        case_id: Option<Uuid>,
        report_id: Option<Uuid>,
        audit: &AdminAuditContext,
    ) -> Result<ContentActionResult, ModerationError> {
        self.require_content_action(principal, action)?;
        let topic = self
            .repository
            .get_topic(topic_id)
            .await
            .map_err(internal)?
            .ok_or(ModerationError::NotFound)?;
        let before = topic.status.clone();
        let reason = normalize_reason(reason)?;

        let (after, before_flag, after_flag) = match action {
            ModerationActionKind::Hide => {
                if topic.status != "published" {
                    return Err(ModerationError::Conflict(
                        "only published topics can be hidden",
                    ));
                }
                ("hidden", "", "")
            }
            ModerationActionKind::Restore => {
                if topic.status == "published" {
                    return Err(ModerationError::Conflict("topic is already published"));
                }
                ("published", "", "")
            }
            ModerationActionKind::Delete => {
                if topic.status == "deleted" {
                    return Err(ModerationError::Conflict("topic is already deleted"));
                }
                ("deleted", "", "")
            }
            ModerationActionKind::Lock => ("published", "locked", "locked"),
            ModerationActionKind::Unlock => ("published", "locked", "unlocked"),
            ModerationActionKind::Pin => ("published", "pinned", "pinned"),
            ModerationActionKind::Unpin => ("published", "pinned", "unpinned"),
            ModerationActionKind::MarkSensitive => ("published", "sensitive", "sensitive"),
            ModerationActionKind::UnmarkSensitive => ("published", "sensitive", "normal"),
            ModerationActionKind::RestrictInteractions => {
                ("published", "interactions", "restricted")
            }
            ModerationActionKind::UnrestrictInteractions => ("published", "interactions", "normal"),
            ModerationActionKind::MoveCategory => {
                return Err(ModerationError::Validation(
                    "move_category requires a target category",
                ));
            }
            _ => {
                return Err(ModerationError::Validation(
                    "action is not valid for topics",
                ));
            }
        };

        let mut tx = self.repository.pool().begin().await.map_err(internal)?;

        // Snapshot before destructive actions.
        if matches!(
            action,
            ModerationActionKind::Hide | ModerationActionKind::Delete
        ) {
            self.repository
                .insert_snapshot(
                    Some(&mut tx),
                    case_id,
                    "topic",
                    topic.id,
                    Some(&topic.title),
                    Some(&topic.content),
                    topic.summary.as_deref(),
                    Some(&topic.status),
                    reason.as_deref(),
                    Some(principal.user_id),
                )
                .await
                .map_err(internal)?;
        }

        let status = if matches!(action, ModerationActionKind::Hide) {
            Some("hidden")
        } else if matches!(
            action,
            ModerationActionKind::Restore | ModerationActionKind::Delete
        ) {
            Some(after)
        } else {
            None
        };
        let updated = self
            .repository
            .set_topic_governance(
                Some(&mut tx),
                topic_id,
                status,
                locked_flag(action),
                sensitive_flag(action),
                interactions_flag(action),
                pinned_flag(action),
                None,
            )
            .await
            .map_err(internal)?
            .ok_or(ModerationError::NotFound)?;

        let after_status = format!("{};{}={}", updated.status, before_flag, after_flag);
        self.repository
            .insert_action(
                Some(&mut tx),
                case_id,
                action.as_str(),
                "topic",
                topic_id,
                Some(&before),
                Some(&after_status),
                reason.as_deref(),
                Some(principal.user_id),
                report_id,
                json!({ "source": "manual" }),
            )
            .await
            .map_err(internal)?;
        self.admin_logs
            .insert_log(
                Some(&mut tx),
                principal.user_id,
                &format!("moderation.{}", action.as_str()),
                "topic",
                Some(topic_id),
                &format!("{} topic {}", action.as_str(), updated.title),
                json!({
                    "before": before,
                    "after": after_status,
                    "reason": reason,
                    "case_id": case_id,
                    "report_id": report_id,
                }),
                audit.ip,
                audit.user_agent.as_deref(),
            )
            .await
            .map_err(internal)?;
        tx.commit().await.map_err(internal)?;

        self.metrics.inc(
            "moderation_actions_total",
            &[("action", action.as_str()), ("target_type", "topic")],
        );
        self.notify_content_author(
            &topic.author_id,
            action,
            "topic",
            topic_id,
            topic.title.as_str(),
        )
        .await;
        Ok(ContentActionResult {
            action: action.as_str(),
            target_type: "topic",
            target_id: topic_id,
            before_status: before,
            after_status,
            case_id,
        })
    }

    pub async fn topic_move(
        &self,
        principal: &AuthenticatedPrincipal,
        topic_id: Uuid,
        request: crate::models::CaseActionRequest,
        audit: &AdminAuditContext,
    ) -> Result<ContentActionResult, ModerationError> {
        require(principal, PERMISSION_MODERATION_TOPIC_MOVE)?;
        let reason = normalize_reason(request.reason)?;
        let Some(target_category) = request.target_category_id else {
            return Err(ModerationError::Validation(
                "move_category requires a target category",
            ));
        };
        let category = self
            .categories
            .find_by_id(target_category)
            .await
            .map_err(internal)?
            .ok_or(ModerationError::NotFound)?;
        if !category.is_visible && !principal.has_permission(PERMISSION_TOPIC_PIN) {
            return Err(ModerationError::Validation(
                "target category is not available",
            ));
        }
        let topic = self
            .repository
            .get_topic(topic_id)
            .await
            .map_err(internal)?
            .ok_or(ModerationError::NotFound)?;
        let before = topic.status.clone();
        let updated = self
            .repository
            .set_topic_governance(
                None,
                topic_id,
                None,
                None,
                None,
                None,
                None,
                Some(target_category),
            )
            .await
            .map_err(internal)?
            .ok_or(ModerationError::NotFound)?;
        let after_status = format!("category={}", updated.category_id);
        self.repository
            .insert_action(
                None,
                request.case_id,
                "move_category",
                "topic",
                topic_id,
                Some(&before),
                Some(&after_status),
                reason.as_deref(),
                Some(principal.user_id),
                None,
                json!({ "category_id": target_category }),
            )
            .await
            .map_err(internal)?;
        self.admin_logs
            .insert_log(
                None,
                principal.user_id,
                "moderation.topic.move",
                "topic",
                Some(topic_id),
                &format!(
                    "moved topic {} to category {}",
                    updated.title, target_category
                ),
                json!({ "reason": reason }),
                audit.ip,
                audit.user_agent.as_deref(),
            )
            .await
            .map_err(internal)?;
        self.metrics.inc(
            "moderation_actions_total",
            &[("action", "move_category"), ("target_type", "topic")],
        );
        Ok(ContentActionResult {
            action: "move_category",
            target_type: "topic",
            target_id: topic_id,
            before_status: before,
            after_status,
            case_id: request.case_id,
        })
    }

    pub async fn comment_action(
        &self,
        principal: &AuthenticatedPrincipal,
        comment_id: Uuid,
        request: crate::models::CaseActionRequest,
        audit: &AdminAuditContext,
    ) -> Result<ContentActionResult, ModerationError> {
        self.require_content_action(principal, &request.action)?;
        let comment = self
            .repository
            .get_comment(comment_id)
            .await
            .map_err(internal)?
            .ok_or(ModerationError::NotFound)?;
        let before = comment.status.clone();
        let reason = normalize_reason(request.reason)?;

        let (status, collapse, sensitive, replies_locked, valid) = match request.action {
            ModerationActionKind::Hide => (
                Some("hidden"),
                None,
                None,
                None,
                comment.status == "published",
            ),
            ModerationActionKind::Restore => (
                Some("published"),
                None,
                None,
                None,
                comment.status != "published",
            ),
            ModerationActionKind::Delete => (
                Some("deleted"),
                None,
                None,
                None,
                comment.status != "deleted",
            ),
            ModerationActionKind::Collapse => {
                (None, Some(true), None, None, comment.status == "published")
            }
            ModerationActionKind::Uncollapse => {
                (None, Some(false), None, None, comment.status == "published")
            }
            ModerationActionKind::MarkSensitive => {
                (None, None, Some(true), None, comment.status == "published")
            }
            ModerationActionKind::UnmarkSensitive => {
                (None, None, Some(false), None, comment.status == "published")
            }
            ModerationActionKind::RestrictReplies => {
                (None, None, None, Some(true), comment.status == "published")
            }
            ModerationActionKind::UnrestrictReplies => {
                (None, None, None, Some(false), comment.status == "published")
            }
            _ => {
                return Err(ModerationError::Validation(
                    "action is not valid for comments",
                ));
            }
        };
        if !valid {
            return Err(ModerationError::Conflict(
                "comment is not in the required state for this action",
            ));
        }

        let mut tx = self.repository.pool().begin().await.map_err(internal)?;
        if matches!(
            request.action,
            ModerationActionKind::Hide | ModerationActionKind::Delete
        ) {
            self.repository
                .insert_snapshot(
                    Some(&mut tx),
                    request.case_id,
                    "comment",
                    comment.id,
                    None,
                    Some(&comment.content),
                    None,
                    Some(&comment.status),
                    reason.as_deref(),
                    Some(principal.user_id),
                )
                .await
                .map_err(internal)?;
        }
        let updated = self
            .repository
            .set_comment_governance(
                Some(&mut tx),
                comment_id,
                status,
                collapse,
                sensitive,
                replies_locked,
            )
            .await
            .map_err(internal)?
            .ok_or(ModerationError::NotFound)?;
        let after_status = format!(
            "{};collapsed={};sensitive={};replies_locked={}",
            updated.status, updated.is_collapsed, updated.is_sensitive, updated.replies_locked
        );
        self.repository
            .insert_action(
                Some(&mut tx),
                request.case_id,
                request.action.as_str(),
                "comment",
                comment_id,
                Some(&before),
                Some(&after_status),
                reason.as_deref(),
                Some(principal.user_id),
                None,
                json!({ "source": "manual" }),
            )
            .await
            .map_err(internal)?;
        self.admin_logs
            .insert_log(
                Some(&mut tx),
                principal.user_id,
                &format!("moderation.{}", request.action.as_str()),
                "comment",
                Some(comment_id),
                &format!(
                    "{} comment {}",
                    request.action.as_str(),
                    comment.content.chars().take(60).collect::<String>()
                ),
                json!({ "before": before, "after": after_status, "reason": reason }),
                audit.ip,
                audit.user_agent.as_deref(),
            )
            .await
            .map_err(internal)?;
        tx.commit().await.map_err(internal)?;

        self.metrics.inc(
            "moderation_actions_total",
            &[
                ("action", request.action.as_str()),
                ("target_type", "comment"),
            ],
        );
        self.notify_content_author(
            &comment.author_id,
            &request.action,
            "comment",
            comment_id,
            "",
        )
        .await;
        Ok(ContentActionResult {
            action: request.action.as_str(),
            target_type: "comment",
            target_id: comment_id,
            before_status: before,
            after_status,
            case_id: request.case_id,
        })
    }

    fn require_content_action(
        &self,
        principal: &AuthenticatedPrincipal,
        action: &ModerationActionKind,
    ) -> Result<(), ModerationError> {
        match action {
            ModerationActionKind::Hide
            | ModerationActionKind::MarkSensitive
            | ModerationActionKind::UnmarkSensitive
            | ModerationActionKind::RestrictInteractions
            | ModerationActionKind::UnrestrictInteractions
            | ModerationActionKind::Collapse
            | ModerationActionKind::Uncollapse
            | ModerationActionKind::RestrictReplies
            | ModerationActionKind::UnrestrictReplies => {
                require(principal, PERMISSION_MODERATION_CONTENT_HIDE)
            }
            ModerationActionKind::Restore => {
                require(principal, PERMISSION_MODERATION_CONTENT_RESTORE)
            }
            ModerationActionKind::Delete => {
                require(principal, PERMISSION_MODERATION_CONTENT_DELETE)
            }
            ModerationActionKind::Lock
            | ModerationActionKind::Unlock
            | ModerationActionKind::Pin
            | ModerationActionKind::Unpin => require_any(
                principal,
                &[PERMISSION_MODERATION_TOPIC_LOCK, PERMISSION_TOPIC_PIN],
            ),
            ModerationActionKind::MoveCategory => {
                require(principal, PERMISSION_MODERATION_TOPIC_MOVE)
            }
        }
    }

    async fn notify_content_author(
        &self,
        author_id: &Uuid,
        action: &ModerationActionKind,
        target_type: &str,
        target_id: Uuid,
        title_hint: &str,
    ) {
        let (notification_type, title, content, href) = match (action, target_type) {
            (ModerationActionKind::Hide, "topic") => (
                crate::models::NotificationType::ContentHidden,
                "内容已被隐藏",
                format!("你的帖子《{title_hint}》因违反社区规范已被隐藏"),
                format!("/topics/{target_id}"),
            ),
            (ModerationActionKind::Hide, _) => (
                crate::models::NotificationType::ContentHidden,
                "评论已被隐藏",
                "你的评论因违反社区规范已被隐藏".to_owned(),
                format!("/topics/{target_id}"),
            ),
            (ModerationActionKind::Delete, "topic") => (
                crate::models::NotificationType::ContentDeleted,
                "内容已被删除",
                format!("你的帖子《{title_hint}》因违反社区规范已被删除"),
                "/".into(),
            ),
            (ModerationActionKind::Delete, _) => (
                crate::models::NotificationType::ContentDeleted,
                "评论已被删除",
                "你的评论因违反社区规范已被删除".to_owned(),
                "/".into(),
            ),
            (ModerationActionKind::Lock, _) => (
                crate::models::NotificationType::TopicLocked,
                "帖子已锁定",
                format!("你的帖子《{title_hint}》已被锁定，无法继续评论"),
                format!("/topics/{target_id}"),
            ),
            _ => return,
        };
        let dedup_key = format!(
            "{}-{}:{target_id}",
            notification_type.as_str(),
            action.as_str()
        );
        let _ = self
            .notifications
            .send(crate::repositories::NewNotification {
                user_id: *author_id,
                actor_id: None,
                notification_type,
                title,
                content: &content,
                target_type: Some(crate::models::NotificationTargetType::Topic),
                target_id: Some(target_id),
                metadata: json!({ "href": href }),
                dedup_key: Some(&dedup_key),
            })
            .await;
    }

    // ------------------------------------------------------------------
    // ⑦ Sanctions
    // ------------------------------------------------------------------

    pub async fn issue_sanction(
        &self,
        principal: &AuthenticatedPrincipal,
        user_id: Uuid,
        request: CreateSanctionRequest,
        audit: &AdminAuditContext,
    ) -> Result<SanctionItem, ModerationError> {
        require_sanction_permission(principal, request.sanction_type)?;
        let reason = normalize_reason(Some(request.reason))?
            .ok_or(ModerationError::Validation("sanction reason is required"))?;
        let user_visible_reason = normalize_reason(request.user_visible_reason)?;
        let internal_note = request
            .internal_note
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty());
        if internal_note
            .as_ref()
            .is_some_and(|value| value.chars().count() > 2000)
        {
            return Err(ModerationError::Validation("internal note is too long"));
        }
        let mut restrictions = request.restrictions.clone();
        restrictions.retain(|value| {
            matches!(
                value.as_str(),
                RESTRICTION_NO_TOPICS
                    | RESTRICTION_NO_COMMENTS
                    | RESTRICTION_NO_REPORTS
                    | RESTRICTION_NO_UPLOADS
            )
        });
        if restrictions.is_empty() && request.restrictions.is_empty() {
            restrictions = request
                .sanction_type
                .default_restrictions()
                .iter()
                .map(|value| (*value).to_owned())
                .collect();
        }

        let now = Utc::now();
        let starts_at = request.starts_at.unwrap_or(now);
        let (ends_at, is_permanent) = match request.sanction_type {
            SanctionType::Ban => (None, true),
            SanctionType::Warning => (
                Some(request.ends_at.unwrap_or(now + Duration::days(30))),
                false,
            ),
            SanctionType::ContentRestriction => (
                Some(request.ends_at.unwrap_or(now + Duration::days(7))),
                false,
            ),
            SanctionType::Mute => (
                Some(request.ends_at.unwrap_or(now + Duration::days(3))),
                false,
            ),
            SanctionType::Suspension => (
                Some(request.ends_at.unwrap_or(now + Duration::days(7))),
                false,
            ),
        };
        if let Some(ends_at) = ends_at {
            if ends_at <= starts_at {
                return Err(ModerationError::Validation(
                    "ends_at must be after starts_at",
                ));
            }
        }
        let status = if starts_at <= now {
            SanctionStatus::Active
        } else {
            SanctionStatus::Scheduled
        };

        let mut tx = self.repository.pool().begin().await.map_err(internal)?;
        let target = self
            .repository
            .lock_target_user(&mut tx, user_id)
            .await
            .map_err(internal)?
            .ok_or(ModerationError::NotFound)?;
        if target.id == principal.user_id {
            return Err(ModerationError::Validation("you cannot sanction yourself"));
        }
        if target.role_priority >= principal_role_priority(principal) {
            return Err(ModerationError::Forbidden);
        }
        if target.role_code == ROLE_SUPER_ADMINISTRATOR {
            return Err(ModerationError::Forbidden);
        }

        let sanction_id = self
            .repository
            .create_sanction(
                &mut tx,
                user_id,
                request.sanction_type.as_str(),
                &reason,
                user_visible_reason.as_deref(),
                internal_note.as_deref(),
                &restrictions,
                starts_at,
                ends_at,
                is_permanent,
                status.as_str(),
                principal.user_id,
                request.case_id,
                request.report_id,
                request.related_content_type.as_deref(),
                request.related_content_id,
            )
            .await
            .map_err(internal)?;

        // Suspension / permanent ban block authentication entirely.
        if matches!(
            request.sanction_type,
            SanctionType::Suspension | SanctionType::Ban
        ) && status == SanctionStatus::Active
            && target.status == "active"
        {
            let user_status = if request.sanction_type == SanctionType::Ban {
                "disabled"
            } else {
                "suspended"
            };
            self.repository
                .set_user_status(&mut tx, user_id, user_status, true)
                .await
                .map_err(internal)?;
            self.repository
                .revoke_refresh_tokens(&mut tx, user_id)
                .await
                .map_err(internal)?;
        }

        self.repository
            .insert_action(
                Some(&mut tx),
                request.case_id,
                "sanction",
                "user",
                user_id,
                None,
                Some(status.as_str()),
                Some(&reason),
                Some(principal.user_id),
                request.report_id,
                json!({ "sanction_id": sanction_id, "sanction_type": request.sanction_type.as_str() }),
            )
            .await
            .map_err(internal)?;
        self.admin_logs
            .insert_log(
                Some(&mut tx),
                principal.user_id,
                &format!("moderation.{}", request.sanction_type.as_str()),
                "user",
                Some(user_id),
                &format!(
                    "issued {} sanction to {}",
                    request.sanction_type.as_str(),
                    target.username
                ),
                json!({
                    "sanction_id": sanction_id,
                    "reason": reason,
                    "restrictions": restrictions,
                    "starts_at": starts_at,
                    "ends_at": ends_at,
                    "is_permanent": is_permanent,
                }),
                audit.ip,
                audit.user_agent.as_deref(),
            )
            .await
            .map_err(internal)?;
        tx.commit().await.map_err(internal)?;
        self.authorization.invalidate(user_id).await;
        self.metrics.inc(
            "moderation_actions_total",
            &[("action", "sanction"), ("target_type", "user")],
        );
        self.notify_sanctioned_user(user_id, &request.sanction_type, sanction_id, ends_at)
            .await;
        self.repository
            .get_sanction(sanction_id)
            .await
            .map_err(internal)?
            .ok_or(ModerationError::NotFound)
    }

    async fn notify_sanctioned_user(
        &self,
        user_id: Uuid,
        sanction_type: &SanctionType,
        sanction_id: Uuid,
        ends_at: Option<chrono::DateTime<Utc>>,
    ) {
        let (notification_type, title, content) = match sanction_type {
            SanctionType::Warning => (
                crate::models::NotificationType::UserWarned,
                "你收到一次警告",
                "你因违反社区规范收到一次警告，请遵守社区规则。",
            ),
            SanctionType::Mute | SanctionType::ContentRestriction => (
                crate::models::NotificationType::UserMuted,
                "你的发言权限受限",
                "你因违反社区规范被限制发言，请在处罚期内遵守社区规则。",
            ),
            SanctionType::Suspension => (
                crate::models::NotificationType::UserBanned,
                "你的账号已被临时封禁",
                "你因违反社区规范被临时封禁，封禁结束后可重新登录。",
            ),
            SanctionType::Ban => (
                crate::models::NotificationType::UserBanned,
                "你的账号已被封禁",
                "你因严重违反社区规范被永久封禁。",
            ),
        };
        let mut metadata = json!({ "sanction_id": sanction_id, "href": "/profile/sanctions" });
        if let Some(ends_at) = ends_at {
            metadata["ends_at"] = json!(ends_at);
            metadata["note"] = json!(format!(
                "处罚将于 {} 结束",
                ends_at.format("%Y-%m-%d %H:%M")
            ));
        }
        let dedup_key = format!("{}:{sanction_id}", notification_type.as_str());
        let _ = self
            .notifications
            .send(crate::repositories::NewNotification {
                user_id,
                actor_id: None,
                notification_type,
                title,
                content,
                target_type: None,
                target_id: None,
                metadata,
                dedup_key: Some(&dedup_key),
            })
            .await;
    }

    pub async fn list_sanctions(
        &self,
        principal: &AuthenticatedPrincipal,
        query: SanctionListQuery,
    ) -> Result<Paginated<SanctionItem>, ModerationError> {
        require(principal, PERMISSION_MODERATION_REPORT_READ)?;
        let (page, page_size, limit, offset) = page_bounds(query.page, query.page_size)?;
        let status = normalize_filter(query.status)?;
        let sanction_type = normalize_filter(query.sanction_type)?;
        let (items, total) = self
            .repository
            .list_sanctions(
                query.user_id,
                status.as_deref(),
                sanction_type.as_deref(),
                limit,
                offset,
            )
            .await
            .map_err(internal)?;
        Ok(paginate(items, page, page_size, total))
    }

    pub async fn list_my_sanctions(
        &self,
        principal: &AuthenticatedPrincipal,
        page: Option<u32>,
        page_size: Option<u32>,
    ) -> Result<Paginated<SanctionItem>, ModerationError> {
        let (page, page_size, limit, offset) = page_bounds(page, page_size)?;
        let (items, total) = self
            .repository
            .list_sanctions(Some(principal.user_id), None, None, limit, offset)
            .await
            .map_err(internal)?;
        let items = items.into_iter().map(strip_internal_note).collect();
        Ok(paginate(items, page, page_size, total))
    }

    pub async fn get_my_sanction(
        &self,
        principal: &AuthenticatedPrincipal,
        sanction_id: Uuid,
    ) -> Result<SanctionItem, ModerationError> {
        let sanction = self
            .repository
            .get_sanction(sanction_id)
            .await
            .map_err(internal)?
            .ok_or(ModerationError::NotFound)?;
        if sanction.user_id == principal.user_id {
            Ok(strip_internal_note(sanction))
        } else if principal.has_permission(PERMISSION_MODERATION_REPORT_READ) {
            Ok(sanction)
        } else {
            Err(ModerationError::NotFound)
        }
    }

    pub async fn revoke_sanction(
        &self,
        principal: &AuthenticatedPrincipal,
        sanction_id: Uuid,
        request: RevokeSanctionRequest,
        audit: &AdminAuditContext,
    ) -> Result<SanctionItem, ModerationError> {
        require(principal, PERMISSION_MODERATION_SANCTION_REVOKE)?;
        let sanction = self
            .repository
            .get_sanction(sanction_id)
            .await
            .map_err(internal)?
            .ok_or(ModerationError::NotFound)?;
        if !matches!(
            sanction.status,
            SanctionStatus::Scheduled | SanctionStatus::Active
        ) {
            return Err(ModerationError::Conflict("sanction is already resolved"));
        }
        let reason = normalize_reason(request.reason)?;
        if sanction.is_permanent {
            if principal.role != ROLE_SUPER_ADMINISTRATOR {
                return Err(ModerationError::Forbidden);
            }
            if !request.confirm.unwrap_or(false) {
                return Err(ModerationError::Validation(
                    "revoking a permanent ban requires explicit confirmation",
                ));
            }
        }

        let mut tx = self.repository.pool().begin().await.map_err(internal)?;
        let target = self
            .repository
            .lock_target_user(&mut tx, sanction.user_id)
            .await
            .map_err(internal)?
            .ok_or(ModerationError::NotFound)?;
        if target.role_priority >= principal_role_priority(principal)
            && target.id != principal.user_id
        {
            return Err(ModerationError::Forbidden);
        }

        let changed = self
            .repository
            .set_sanction_status(
                Some(&mut tx),
                sanction_id,
                SanctionStatus::Revoked.as_str(),
                Some(principal.user_id),
                reason.as_deref(),
            )
            .await
            .map_err(internal)?;
        if !changed {
            return Err(ModerationError::Conflict("sanction is already resolved"));
        }

        // Restore account access when this was the only active suspension/ban.
        if matches!(
            sanction.sanction_type,
            SanctionType::Suspension | SanctionType::Ban
        ) {
            let still_banned = self
                .repository
                .has_active_account_ban(&mut tx, sanction.user_id)
                .await
                .map_err(internal)?;
            if !still_banned && target.status != "active" {
                self.repository
                    .set_user_status(&mut tx, sanction.user_id, "active", false)
                    .await
                    .map_err(internal)?;
            }
        }

        self.repository
            .insert_action(
                Some(&mut tx),
                sanction.case_id,
                "revoke_sanction",
                "sanction",
                sanction_id,
                Some(sanction.status.as_str()),
                Some(SanctionStatus::Revoked.as_str()),
                reason.as_deref(),
                Some(principal.user_id),
                sanction.report_id,
                json!({ "user_id": sanction.user_id }),
            )
            .await
            .map_err(internal)?;
        self.admin_logs
            .insert_log(
                Some(&mut tx),
                principal.user_id,
                "moderation.sanction.revoke",
                "user",
                Some(sanction.user_id),
                &format!("revoked sanction {} for {}", sanction_id, target.username),
                json!({ "sanction_id": sanction_id, "reason": reason }),
                audit.ip,
                audit.user_agent.as_deref(),
            )
            .await
            .map_err(internal)?;
        tx.commit().await.map_err(internal)?;
        self.authorization.invalidate(sanction.user_id).await;

        let dedup_key = format!("sanction_revoked:{sanction_id}");
        let _ = self
            .notifications
            .send(crate::repositories::NewNotification {
                user_id: sanction.user_id,
                actor_id: None,
                notification_type: crate::models::NotificationType::SanctionRevoked,
                title: "处罚已解除",
                content: "你的处罚已被解除。",
                target_type: None,
                target_id: None,
                metadata: json!({ "sanction_id": sanction_id, "href": "/profile/sanctions" }),
                dedup_key: Some(&dedup_key),
            })
            .await;
        self.repository
            .get_sanction(sanction_id)
            .await
            .map_err(internal)?
            .ok_or(ModerationError::NotFound)
    }

    async fn user_violations(&self, user_id: Uuid) -> Vec<crate::models::ViolationSummary> {
        match self
            .repository
            .list_sanctions(Some(user_id), None, None, 50, 0)
            .await
        {
            Ok((items, _)) => items
                .into_iter()
                .map(|item| crate::models::ViolationSummary {
                    sanction_type: item.sanction_type,
                    status: item.status,
                    reason: item.reason,
                    issued_at: item.created_at,
                })
                .collect(),
            Err(error) => {
                tracing::warn!(%error, %user_id, "failed to load user violations");
                Vec::new()
            }
        }
    }

    // ------------------------------------------------------------------
    // ⑧ Auto-moderation
    // ------------------------------------------------------------------

    /// Screen content before it is persisted. Redis counters (rate, duplicate,
    /// frequency) are consumed here; DB hit rows are recorded after the
    /// target row exists via `record_screening`.
    pub async fn screen_content(
        &self,
        principal: &AuthenticatedPrincipal,
        target_type: &str,
        title: &str,
        content: &str,
    ) -> Result<ScreeningDecision, ModerationError> {
        let rules = self.cached_rules().await?;
        let user = self
            .repository
            .get_user(principal.user_id)
            .await
            .map_err(internal)?
            .ok_or(ModerationError::NotFound)?;
        let normalized = normalize_text(&format!("{title} {content}"));
        let mut decision = ScreeningDecision::default();

        for rule in rules {
            if !rule.enabled || !rule_applies_to(&rule, target_type) {
                continue;
            }
            if rule.rule_type == RuleType::NewUser {
                // new_user is a booster, not a standalone match
                let created_within =
                    user_created_within_hours(&user, rule.config_i64("min_age_hours", 48));
                if created_within {
                    decision.hits.push(ScreeningHit {
                        rule_id: rule.id,
                        rule_type: rule.rule_type,
                        action: RuleAction::Flag,
                        risk_score: rule.risk_score,
                    });
                }
                continue;
            }
            let matched = match rule.rule_type {
                RuleType::Keyword => {
                    let keywords: Vec<String> = rule.config_array("keywords");
                    keywords.iter().any(|keyword| {
                        let normalized_keyword = normalize_text(keyword);
                        !normalized_keyword.is_empty() && normalized.contains(&normalized_keyword)
                    })
                }
                RuleType::UrlDomain => {
                    let domains: Vec<String> = rule.config_array("domains");
                    extract_domains(&normalized).iter().any(|found| {
                        domains
                            .iter()
                            .any(|domain| found == domain || found.ends_with(&format!(".{domain}")))
                    })
                }
                RuleType::Rate => {
                    let limit = rule.config_i64("limit", 5);
                    let window = rule.config_i64("window_secs", 60);
                    self.incr_window(
                        &format!("mod:rate:{}:{target_type}:{}", principal.user_id, rule.id),
                        window,
                    )
                    .await
                        > limit.max(0) as u64
                }
                RuleType::Duplicate => {
                    let window = rule.config_i64("window_secs", 3600);
                    self.check_duplicate(principal.user_id, target_type, &normalized, window)
                        .await
                }
                RuleType::HighFrequency => {
                    let limit = rule.config_i64("limit", 10);
                    let window_secs = rule.config_i64("window_secs", 300);
                    let count = self
                        .repository
                        .recent_content_count(principal.user_id, window_secs)
                        .await
                        .map_err(internal)?;
                    count >= limit
                }
                _ => false,
            };
            if matched {
                decision.hits.push(ScreeningHit {
                    rule_id: rule.id,
                    rule_type: rule.rule_type,
                    action: rule.action,
                    risk_score: rule.risk_score,
                });
            }
        }

        // Pick the strongest matched action.
        let mut strongest = RuleAction::Allow;
        let mut risk = 0i32;
        for hit in &decision.hits {
            if hit.action.strength() > strongest.strength() {
                strongest = hit.action;
            }
            risk += hit.risk_score;
        }
        if strongest == RuleAction::Allow {
            decision.action = RuleAction::Allow;
        } else {
            decision.action = strongest;
            decision.risk_score = risk.clamp(0, 100);
        }
        Ok(decision)
    }

    /// Persist screening results after the content row exists.
    pub async fn record_screening(
        &self,
        target_type: &str,
        target_id: Uuid,
        user_id: Uuid,
        decision: &ScreeningDecision,
        content_preview: &str,
    ) -> Result<(), ModerationError> {
        if decision.hits.is_empty() {
            return Ok(());
        }
        let snippet: String = content_preview.chars().take(500).collect();
        for hit in &decision.hits {
            self.metrics.inc(
                "moderation_auto_rules_triggered_total",
                &[
                    ("rule_type", hit.rule_type.as_str()),
                    ("action", hit.action.as_str()),
                ],
            );
            self.repository
                .insert_rule_hit(
                    hit.rule_id,
                    target_type,
                    Some(target_id),
                    Some(user_id),
                    Some(&snippet),
                    hit.risk_score,
                    hit.action.as_str(),
                )
                .await
                .map_err(internal)?;
        }
        if decision.action.strength() >= RuleAction::Queue.strength() {
            let priority = risk_to_priority(decision.risk_score);
            let case_id = match self
                .repository
                .find_open_case(target_type, target_id)
                .await
                .map_err(internal)?
            {
                Some(id) => id,
                None => self
                    .repository
                    .create_case(
                        target_type,
                        target_id,
                        priority.as_str(),
                        decision.risk_score,
                        CaseSource::Auto.as_str(),
                        None,
                    )
                    .await
                    .map_err(internal)?,
            };
            self.repository
                .insert_snapshot(
                    None,
                    Some(case_id),
                    target_type,
                    target_id,
                    None,
                    Some(&snippet),
                    None,
                    None,
                    Some("auto-moderation"),
                    None,
                )
                .await
                .map_err(internal)?;
            self.repository
                .insert_action(
                    None,
                    Some(case_id),
                    decision.action.as_str(),
                    target_type,
                    target_id,
                    None,
                    None,
                    Some("auto-moderation"),
                    None,
                    None,
                    json!({ "source": "auto", "risk_score": decision.risk_score }),
                )
                .await
                .map_err(internal)?;
            self.notify_staff_realtime(
                "moderation.case.created",
                json!({
                    "case_id": case_id,
                    "target_type": target_type,
                    "target_id": target_id,
                    "priority": priority.as_str(),
                    "source": "auto",
                }),
            )
            .await;
        }
        Ok(())
    }

    async fn cached_rules(&self) -> Result<Vec<RuleItem>, ModerationError> {
        let mut redis = self.redis.clone();
        if let Ok(Some(value)) = redis.get::<_, Option<String>>(RULES_CACHE_KEY).await {
            if let Ok(rules) = serde_json::from_str::<Vec<RuleItem>>(&value) {
                return Ok(rules);
            }
        }
        let (rules, _) = self
            .repository
            .list_rules(true, 1000, 0)
            .await
            .map_err(internal)?;
        if let Ok(value) = serde_json::to_string(&rules) {
            if let Err(error) = redis
                .set_ex::<_, _, ()>(RULES_CACHE_KEY, value, RULES_CACHE_TTL_SECS)
                .await
            {
                tracing::warn!(%error, "failed to cache moderation rules");
            }
        }
        Ok(rules)
    }

    pub async fn invalidate_rules_cache(&self) {
        let mut redis = self.redis.clone();
        if let Err(error) = redis.del::<_, ()>(RULES_CACHE_KEY).await {
            tracing::warn!(%error, "failed to invalidate moderation rules cache");
        }
    }

    async fn incr_window(&self, key: &str, window_secs: i64) -> u64 {
        let mut redis = self.redis.clone();
        match redis.incr::<_, u64, u64>(key, 1_u64).await {
            Ok(count) => {
                if count == 1 {
                    let _: Result<(), _> = redis.expire(key, window_secs).await;
                }
                count
            }
            Err(error) => {
                tracing::warn!(%error, "moderation rate counter unavailable");
                0
            }
        }
    }

    async fn check_duplicate(
        &self,
        user_id: Uuid,
        target_type: &str,
        normalized: &str,
        window: i64,
    ) -> bool {
        let mut hasher = Sha256::new();
        hasher.update(normalized.as_bytes());
        let digest = format!("{:x}", hasher.finalize());
        let key = format!("mod:dup:{user_id}:{target_type}");
        let mut redis = self.redis.clone();
        let member: bool = match redis.sadd::<_, _, bool>(&key, &digest).await {
            Ok(member) => member,
            Err(error) => {
                tracing::warn!(%error, "duplicate check unavailable");
                false
            }
        };
        let _: Result<(), _> = redis.expire(&key, window).await;
        // sadd returns false when the member already existed.
        !member
    }

    // ------------------------------------------------------------------
    // ⑨ Appeals
    // ------------------------------------------------------------------

    pub async fn create_appeal(
        &self,
        principal: &AuthenticatedPrincipal,
        request: CreateAppealRequest,
    ) -> Result<AppealItem, ModerationError> {
        let reason = request.reason.trim().to_owned();
        if !(3..=2000).contains(&reason.chars().count()) {
            return Err(ModerationError::Validation(
                "reason must contain between 3 and 2000 characters",
            ));
        }
        let details = request
            .details
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty());
        if details
            .as_ref()
            .is_some_and(|value| value.chars().count() > 2000)
        {
            return Err(ModerationError::Validation("details are too long"));
        }
        let has_sanction = request.sanction_id.is_some();
        let has_content = request.content_type.is_some() || request.content_id.is_some();
        if has_sanction == has_content {
            return Err(ModerationError::Validation(
                "appeal must target exactly one sanction or one content item",
            ));
        }

        let (appeal_type, sanction_id, content_type, content_id) = if has_sanction {
            let sanction = self
                .repository
                .get_sanction(request.sanction_id.unwrap())
                .await
                .map_err(internal)?
                .ok_or(ModerationError::NotFound)?;
            if sanction.user_id != principal.user_id {
                return Err(ModerationError::NotFound);
            }
            if matches!(sanction.status, SanctionStatus::Revoked) {
                return Err(ModerationError::Validation(
                    "this sanction is already resolved",
                ));
            }
            let count = self
                .repository
                .count_appeals_for_sanction(sanction.id)
                .await
                .map_err(internal)?;
            if count >= MAX_APPEALS_PER_SANCTION {
                return Err(ModerationError::Conflict(
                    "the appeal limit for this sanction has been reached",
                ));
            }
            (AppealType::Sanction.as_str(), Some(sanction.id), None, None)
        } else {
            let content_type = request
                .content_type
                .as_deref()
                .ok_or(ModerationError::Validation("content_type is required"))?;
            if !matches!(content_type, "topic" | "comment") {
                return Err(ModerationError::Validation("invalid content_type"));
            }
            let content_id = request
                .content_id
                .ok_or(ModerationError::Validation("content_id is required"))?;
            let owner = match content_type {
                "topic" => self
                    .repository
                    .get_topic(content_id)
                    .await
                    .map_err(internal)?
                    .map(|topic| topic.author_id),
                _ => self
                    .repository
                    .get_comment(content_id)
                    .await
                    .map_err(internal)?
                    .map(|comment| comment.author_id),
            }
            .ok_or(ModerationError::NotFound)?;
            if owner != principal.user_id {
                return Err(ModerationError::NotFound);
            }
            (
                AppealType::Content.as_str(),
                None,
                Some(content_type),
                Some(content_id),
            )
        };

        // Evidence uploads must belong to the user.
        if !request.evidence.is_empty() {
            let owned = self
                .repository
                .count_user_uploads(principal.user_id, &request.evidence)
                .await
                .map_err(internal)?;
            if owned != request.evidence.len() as i64 {
                return Err(ModerationError::Validation("evidence uploads are invalid"));
            }
        }

        let appeal_id = self
            .repository
            .create_appeal(
                principal.user_id,
                appeal_type,
                sanction_id,
                content_type,
                content_id,
                &reason,
                details.as_deref(),
                &request.evidence,
            )
            .await
            .map_err(internal)?;
        self.repository
            .insert_appeal_event(
                appeal_id,
                "user",
                Some(principal.user_id),
                "submitted",
                None,
            )
            .await
            .map_err(internal)?;
        self.metrics
            .inc("moderation_appeals_total", &[("status", "pending")]);
        self.notify_staff_realtime(
            "moderation.appeal.submitted",
            json!({ "appeal_id": appeal_id }),
        )
        .await;
        self.repository
            .get_appeal(appeal_id)
            .await
            .map_err(internal)?
            .ok_or(ModerationError::NotFound)
    }

    pub async fn list_my_appeals(
        &self,
        principal: &AuthenticatedPrincipal,
        page: Option<u32>,
        page_size: Option<u32>,
    ) -> Result<Paginated<AppealItem>, ModerationError> {
        let (page, page_size, limit, offset) = page_bounds(page, page_size)?;
        let (items, total) = self
            .repository
            .list_appeals(Some(principal.user_id), None, None, limit, offset)
            .await
            .map_err(internal)?;
        Ok(paginate(items, page, page_size, total))
    }

    pub async fn get_my_appeal(
        &self,
        principal: &AuthenticatedPrincipal,
        appeal_id: Uuid,
    ) -> Result<AppealItem, ModerationError> {
        let appeal = self
            .repository
            .get_appeal(appeal_id)
            .await
            .map_err(internal)?
            .ok_or(ModerationError::NotFound)?;
        if appeal.user_id == principal.user_id
            || principal.has_permission(PERMISSION_MODERATION_APPEAL_READ)
        {
            Ok(appeal)
        } else {
            Err(ModerationError::NotFound)
        }
    }

    pub async fn list_appeals(
        &self,
        principal: &AuthenticatedPrincipal,
        query: AppealListQuery,
    ) -> Result<Paginated<AppealItem>, ModerationError> {
        require(principal, PERMISSION_MODERATION_APPEAL_READ)?;
        let (page, page_size, limit, offset) = page_bounds(query.page, query.page_size)?;
        let status = normalize_filter(query.status)?;
        let q = normalize_search(query.q)?;
        let (items, total) = self
            .repository
            .list_appeals(None, status.as_deref(), q.as_deref(), limit, offset)
            .await
            .map_err(internal)?;
        Ok(paginate(items, page, page_size, total))
    }

    pub async fn review_appeal(
        &self,
        principal: &AuthenticatedPrincipal,
        appeal_id: Uuid,
        request: ReviewAppealRequest,
        audit: &AdminAuditContext,
    ) -> Result<AppealItem, ModerationError> {
        require(principal, PERMISSION_MODERATION_APPEAL_REVIEW)?;
        let appeal = self
            .repository
            .get_appeal(appeal_id)
            .await
            .map_err(internal)?
            .ok_or(ModerationError::NotFound)?;
        if !matches!(
            appeal.status,
            AppealStatus::Pending | AppealStatus::Reviewing
        ) {
            return Err(ModerationError::Conflict("appeal is already handled"));
        }
        let note = request
            .note
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty());
        if matches!(request.decision, AppealStatus::Rejected)
            && note.as_ref().is_none_or(|value| value.chars().count() < 3)
        {
            return Err(ModerationError::Validation(
                "rejecting an appeal requires a reason",
            ));
        }

        // The original sanction issuer cannot review the appeal alone.
        if let Some(sanction_id) = appeal.sanction_id {
            if let Some(sanction) = self
                .repository
                .get_sanction(sanction_id)
                .await
                .map_err(internal)?
            {
                if sanction.issued_by == Some(principal.user_id) {
                    return Err(ModerationError::Forbidden);
                }
            }
        }

        match request.decision {
            AppealStatus::Approved => {
                let mut tx = self.repository.pool().begin().await.map_err(internal)?;
                let changed = self
                    .repository
                    .set_appeal_status(
                        Some(&mut tx),
                        appeal_id,
                        AppealStatus::Approved.as_str(),
                        Some(principal.user_id),
                        note.as_deref(),
                    )
                    .await
                    .map_err(internal)?;
                if !changed {
                    return Err(ModerationError::Conflict("appeal is already handled"));
                }
                self.apply_appeal_restoration(&mut tx, principal, &appeal, note.as_deref())
                    .await?;
                tx.commit().await.map_err(internal)?;
            }
            AppealStatus::Rejected => {
                let changed = self
                    .repository
                    .set_appeal_status(
                        None,
                        appeal_id,
                        AppealStatus::Rejected.as_str(),
                        Some(principal.user_id),
                        note.as_deref(),
                    )
                    .await
                    .map_err(internal)?;
                if !changed {
                    return Err(ModerationError::Conflict("appeal is already handled"));
                }
            }
            AppealStatus::Reviewing => {
                let changed = self
                    .repository
                    .set_appeal_status(
                        None,
                        appeal_id,
                        AppealStatus::Reviewing.as_str(),
                        Some(principal.user_id),
                        None,
                    )
                    .await
                    .map_err(internal)?;
                if !changed {
                    return Err(ModerationError::Conflict("appeal is already handled"));
                }
            }
            _ => {
                return Err(ModerationError::Validation("invalid appeal decision"));
            }
        }

        self.repository
            .insert_appeal_event(
                appeal_id,
                "moderator",
                Some(principal.user_id),
                request.decision.as_str(),
                note.as_deref(),
            )
            .await
            .map_err(internal)?;
        self.metrics.inc(
            "moderation_appeals_total",
            &[("status", request.decision.as_str())],
        );
        self.admin_logs
            .insert_log(
                None,
                principal.user_id,
                &format!("moderation.appeal.{}", request.decision.as_str()),
                "appeal",
                Some(appeal_id),
                &format!("appeal {} set to {}", appeal_id, request.decision.as_str()),
                json!({ "note": note, "user_id": appeal.user_id }),
                audit.ip,
                audit.user_agent.as_deref(),
            )
            .await
            .map_err(internal)?;

        if request.decision != AppealStatus::Reviewing {
            let (notification_type, title, content) = match request.decision {
                AppealStatus::Approved => (
                    crate::models::NotificationType::AppealApproved,
                    "申诉已通过",
                    "你的申诉已通过，相关处罚或内容处置已被撤销。",
                ),
                _ => (
                    crate::models::NotificationType::AppealRejected,
                    "申诉未通过",
                    "你的申诉经复核后未被采纳。",
                ),
            };
            let dedup_key = format!("{}:{appeal_id}", notification_type.as_str());
            let _ = self
                .notifications
                .send(crate::repositories::NewNotification {
                    user_id: appeal.user_id,
                    actor_id: None,
                    notification_type,
                    title,
                    content,
                    target_type: None,
                    target_id: None,
                    metadata: json!({ "appeal_id": appeal_id, "href": "/profile/appeals" }),
                    dedup_key: Some(&dedup_key),
                })
                .await;
        }

        self.repository
            .get_appeal(appeal_id)
            .await
            .map_err(internal)?
            .ok_or(ModerationError::NotFound)
    }

    async fn apply_appeal_restoration(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        principal: &AuthenticatedPrincipal,
        appeal: &AppealItem,
        note: Option<&str>,
    ) -> Result<(), ModerationError> {
        match appeal.appeal_type {
            AppealType::Sanction => {
                let Some(sanction_id) = appeal.sanction_id else {
                    return Err(ModerationError::Internal(anyhow::anyhow!(
                        "sanction appeal without sanction_id"
                    )));
                };
                let sanction = self
                    .repository
                    .get_sanction(sanction_id)
                    .await
                    .map_err(internal)?
                    .ok_or(ModerationError::NotFound)?;
                if !matches!(
                    sanction.status,
                    SanctionStatus::Scheduled | SanctionStatus::Active
                ) {
                    return Err(ModerationError::Conflict("sanction is already resolved"));
                }
                let _ = self
                    .repository
                    .set_sanction_status(
                        Some(tx),
                        sanction_id,
                        SanctionStatus::Revoked.as_str(),
                        Some(principal.user_id),
                        note.or(Some("appeal approved")),
                    )
                    .await
                    .map_err(internal)?;
                if matches!(
                    sanction.sanction_type,
                    SanctionType::Suspension | SanctionType::Ban
                ) {
                    let still_banned = self
                        .repository
                        .has_active_account_ban(tx, sanction.user_id)
                        .await
                        .map_err(internal)?;
                    if !still_banned {
                        let user = self
                            .repository
                            .lock_target_user(tx, sanction.user_id)
                            .await
                            .map_err(internal)?;
                        if let Some(user) = user {
                            if user.status != "active" {
                                self.repository
                                    .set_user_status(tx, sanction.user_id, "active", false)
                                    .await
                                    .map_err(internal)?;
                            }
                        }
                    }
                }
                self.repository
                    .insert_action(
                        Some(tx),
                        sanction.case_id,
                        "revoke_sanction",
                        "sanction",
                        sanction_id,
                        Some(sanction.status.as_str()),
                        Some(SanctionStatus::Revoked.as_str()),
                        Some("appeal approved"),
                        Some(principal.user_id),
                        sanction.report_id,
                        json!({ "appeal_id": appeal.id }),
                    )
                    .await
                    .map_err(internal)?;
                self.authorization.invalidate(sanction.user_id).await;
            }
            AppealType::Content => {
                let Some(content_type) = appeal.content_type.as_deref() else {
                    return Err(ModerationError::Internal(anyhow::anyhow!(
                        "content appeal without content_type"
                    )));
                };
                let content_id =
                    appeal
                        .content_id
                        .ok_or(ModerationError::Internal(anyhow::anyhow!(
                            "content appeal without content_id"
                        )))?;
                match content_type {
                    "topic" => {
                        let topic = self
                            .repository
                            .get_topic(content_id)
                            .await
                            .map_err(internal)?
                            .ok_or(ModerationError::NotFound)?;
                        if topic.status != "published" {
                            self.repository
                                .set_topic_governance(
                                    Some(tx),
                                    content_id,
                                    Some("published"),
                                    None,
                                    None,
                                    None,
                                    None,
                                    None,
                                )
                                .await
                                .map_err(internal)?;
                        }
                        self.repository
                            .insert_action(
                                Some(tx),
                                None,
                                "restore",
                                "topic",
                                content_id,
                                Some(&topic.status),
                                Some("published"),
                                Some("appeal approved"),
                                Some(principal.user_id),
                                None,
                                json!({ "appeal_id": appeal.id }),
                            )
                            .await
                            .map_err(internal)?;
                    }
                    "comment" => {
                        let comment = self
                            .repository
                            .get_comment(content_id)
                            .await
                            .map_err(internal)?
                            .ok_or(ModerationError::NotFound)?;
                        if comment.status != "published" {
                            self.repository
                                .set_comment_governance(
                                    Some(tx),
                                    content_id,
                                    Some("published"),
                                    None,
                                    None,
                                    None,
                                )
                                .await
                                .map_err(internal)?;
                        }
                        self.repository
                            .insert_action(
                                Some(tx),
                                None,
                                "restore",
                                "comment",
                                content_id,
                                Some(&comment.status),
                                Some("published"),
                                Some("appeal approved"),
                                Some(principal.user_id),
                                None,
                                json!({ "appeal_id": appeal.id }),
                            )
                            .await
                            .map_err(internal)?;
                    }
                    _ => {
                        return Err(ModerationError::Validation("invalid content type"));
                    }
                }
            }
        }
        Ok(())
    }

    // ------------------------------------------------------------------
    // Rules management (⑧ admin)
    // ------------------------------------------------------------------

    pub async fn list_rules(
        &self,
        principal: &AuthenticatedPrincipal,
        query: RuleListQuery,
    ) -> Result<Paginated<RuleItem>, ModerationError> {
        require(principal, PERMISSION_MODERATION_RULE_MANAGE)?;
        let (page, page_size, limit, offset) = page_bounds(query.page, query.page_size)?;
        let enabled_only = query.enabled.unwrap_or(false);
        let (items, total) = self
            .repository
            .list_rules(enabled_only, limit, offset)
            .await
            .map_err(internal)?;
        Ok(paginate(items, page, page_size, total))
    }

    pub async fn create_rule(
        &self,
        principal: &AuthenticatedPrincipal,
        request: RuleRequest,
        audit: &AdminAuditContext,
    ) -> Result<RuleItem, ModerationError> {
        require(principal, PERMISSION_MODERATION_RULE_MANAGE)?;
        let name = request.name.trim().to_owned();
        if name.is_empty() || name.chars().count() > 100 {
            return Err(ModerationError::Validation(
                "rule name must contain between 1 and 100 characters",
            ));
        }
        if !matches!(
            request.target_type.as_str(),
            "topic" | "comment" | "user" | "all"
        ) {
            return Err(ModerationError::Validation("invalid target type"));
        }
        let risk_score = request.risk_score.unwrap_or(5).clamp(1, 100);
        let priority = request.priority.unwrap_or(0).max(0);
        let config = request.config.unwrap_or_else(|| json!({}));
        let id = self
            .repository
            .create_rule(
                &name,
                request.rule_type.as_str(),
                &request.target_type,
                priority,
                request.enabled.unwrap_or(true),
                risk_score,
                request.action.as_str(),
                config,
                principal.user_id,
            )
            .await
            .map_err(internal)?;
        self.admin_logs
            .insert_log(
                None,
                principal.user_id,
                "moderation.rule.create",
                "rule",
                Some(id),
                &format!("created moderation rule {name}"),
                json!({ "rule_type": request.rule_type.as_str() }),
                audit.ip,
                audit.user_agent.as_deref(),
            )
            .await
            .map_err(internal)?;
        self.invalidate_rules_cache().await;
        self.repository
            .get_rule(id)
            .await
            .map_err(internal)?
            .ok_or(ModerationError::NotFound)
    }

    pub async fn update_rule(
        &self,
        principal: &AuthenticatedPrincipal,
        rule_id: Uuid,
        request: RuleRequest,
        audit: &AdminAuditContext,
    ) -> Result<RuleItem, ModerationError> {
        require(principal, PERMISSION_MODERATION_RULE_MANAGE)?;
        let name = request.name.trim().to_owned();
        if name.is_empty() || name.chars().count() > 100 {
            return Err(ModerationError::Validation(
                "rule name must contain between 1 and 100 characters",
            ));
        }
        let changed = self
            .repository
            .update_rule(
                rule_id,
                Some(&name),
                Some(request.rule_type.as_str()),
                Some(&request.target_type),
                Some(request.priority.unwrap_or(0).max(0)),
                request.enabled,
                request.risk_score.map(|value| value.clamp(1, 100)),
                Some(request.action.as_str()),
                request.config,
            )
            .await
            .map_err(internal)?;
        if !changed {
            return Err(ModerationError::NotFound);
        }
        self.admin_logs
            .insert_log(
                None,
                principal.user_id,
                "moderation.rule.update",
                "rule",
                Some(rule_id),
                &format!("updated moderation rule {name}"),
                json!({ "rule_type": request.rule_type.as_str() }),
                audit.ip,
                audit.user_agent.as_deref(),
            )
            .await
            .map_err(internal)?;
        self.invalidate_rules_cache().await;
        self.repository
            .get_rule(rule_id)
            .await
            .map_err(internal)?
            .ok_or(ModerationError::NotFound)
    }

    pub async fn delete_rule(
        &self,
        principal: &AuthenticatedPrincipal,
        rule_id: Uuid,
        audit: &AdminAuditContext,
    ) -> Result<(), ModerationError> {
        require(principal, PERMISSION_MODERATION_RULE_MANAGE)?;
        if !self
            .repository
            .delete_rule(rule_id)
            .await
            .map_err(internal)?
        {
            return Err(ModerationError::NotFound);
        }
        self.admin_logs
            .insert_log(
                None,
                principal.user_id,
                "moderation.rule.delete",
                "rule",
                Some(rule_id),
                "deleted moderation rule",
                json!({}),
                audit.ip,
                audit.user_agent.as_deref(),
            )
            .await
            .map_err(internal)?;
        self.invalidate_rules_cache().await;
        Ok(())
    }

    // ------------------------------------------------------------------
    // Enforcement (used by topic/comment/upload services)
    // ------------------------------------------------------------------

    pub async fn enforce_topic_creation(&self, user_id: Uuid) -> Result<(), ModerationError> {
        let restrictions = self
            .repository
            .active_restrictions(user_id)
            .await
            .map_err(internal)?;
        if restrictions
            .iter()
            .any(|value| value == RESTRICTION_NO_TOPICS)
        {
            return Err(ModerationError::Validation("你的账号当前被限制发布内容"));
        }
        Ok(())
    }

    pub async fn enforce_comment_creation(
        &self,
        user_id: Uuid,
        topic_id: Uuid,
    ) -> Result<(), ModerationError> {
        let restrictions = self
            .repository
            .active_restrictions(user_id)
            .await
            .map_err(internal)?;
        if restrictions
            .iter()
            .any(|value| value == RESTRICTION_NO_COMMENTS)
        {
            return Err(ModerationError::Validation("你的账号当前被限制发表评论"));
        }
        if self
            .repository
            .is_topic_locked(topic_id)
            .await
            .map_err(internal)?
        {
            return Err(ModerationError::Validation("该帖子已被锁定，无法评论"));
        }
        Ok(())
    }

    pub async fn enforce_reply_creation(
        &self,
        user_id: Uuid,
        topic_id: Uuid,
        parent_comment_id: Uuid,
    ) -> Result<(), ModerationError> {
        self.enforce_comment_creation(user_id, topic_id).await?;
        if self
            .repository
            .is_comment_replies_locked(parent_comment_id)
            .await
            .map_err(internal)?
        {
            return Err(ModerationError::Validation("该评论已限制回复"));
        }
        Ok(())
    }

    pub async fn enforce_report_creation(&self, user_id: Uuid) -> Result<(), ModerationError> {
        let restrictions = self
            .repository
            .active_restrictions(user_id)
            .await
            .map_err(internal)?;
        if restrictions
            .iter()
            .any(|value| value == RESTRICTION_NO_REPORTS)
        {
            return Err(ModerationError::Validation("你的账号当前被限制提交举报"));
        }
        Ok(())
    }

    pub async fn enforce_upload_creation(&self, user_id: Uuid) -> Result<(), ModerationError> {
        let restrictions = self
            .repository
            .active_restrictions(user_id)
            .await
            .map_err(internal)?;
        if restrictions
            .iter()
            .any(|value| value == RESTRICTION_NO_UPLOADS)
        {
            return Err(ModerationError::Validation("你的账号当前被限制上传文件"));
        }
        Ok(())
    }

    async fn enforce_report_rate_limit(&self, user_id: Uuid) -> Result<(), ModerationError> {
        let key = format!("rate:report:{user_id}");
        let mut redis = self.redis.clone();
        match redis.incr::<_, u64, u64>(&key, 1_u64).await {
            Ok(count) => {
                if count == 1 {
                    let _: Result<(), _> = redis.expire(&key, REPORT_RATE_WINDOW_SECS as i64).await;
                }
                if count > REPORT_RATE_LIMIT {
                    Err(ModerationError::RateLimited)
                } else {
                    Ok(())
                }
            }
            Err(error) => {
                tracing::warn!(%error, %user_id, "report rate limit unavailable; allowing request");
                Ok(())
            }
        }
    }

    // ------------------------------------------------------------------
    // ⑮ Governance metrics
    // ------------------------------------------------------------------

    pub async fn governance_metrics(
        &self,
        principal: &AuthenticatedPrincipal,
    ) -> Result<GovernanceMetrics, ModerationError> {
        require(principal, PERMISSION_MODERATION_METRICS_READ)?;
        self.metrics_snapshot().await
    }

    /// Unauthorized snapshot used by the /metrics scrape endpoint (network-restricted).
    pub async fn metrics_snapshot(&self) -> Result<GovernanceMetrics, ModerationError> {
        self.repository.governance_metrics().await.map_err(internal)
    }

    // ------------------------------------------------------------------
    // ⑭ Audit logs
    // ------------------------------------------------------------------

    pub async fn list_audit_logs(
        &self,
        principal: &AuthenticatedPrincipal,
        query: crate::models::AdminLogListQuery,
    ) -> Result<Paginated<crate::models::AdminLogItem>, ModerationError> {
        require(principal, PERMISSION_MODERATION_AUDIT_READ)?;
        let (page, page_size, limit, offset) = page_bounds(query.page, query.page_size)?;
        let q = normalize_search(query.q)?;
        let action = query
            .action
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty());
        let (items, total) = self
            .admin_logs
            .list_logs(q.as_deref(), action.as_deref(), None, limit, offset)
            .await
            .map_err(internal)?;
        let items = items
            .into_iter()
            .filter(|item| item.action.starts_with("moderation."))
            .collect();
        Ok(paginate(items, page, page_size, total))
    }

    // ------------------------------------------------------------------
    // Realtime + maintenance
    // ------------------------------------------------------------------

    pub async fn notify_staff_realtime(&self, event_type: &str, data: serde_json::Value) {
        let staff = match self.repository.list_staff_user_ids().await {
            Ok(staff) => staff,
            Err(error) => {
                tracing::warn!(%error, "failed to load staff for realtime fan-out");
                return;
            }
        };
        for user_id in staff {
            self.metrics.inc(
                "moderation_websocket_events_total",
                &[("event_type", event_type)],
            );
            if let Err(error) = self
                .realtime
                .publish_to_user(user_id, event_type, data.clone())
                .await
            {
                tracing::debug!(%error, %user_id, "moderation realtime publish failed");
            }
        }
    }

    /// Periodic maintenance: expire sanctions, expiry reminders, purge snapshots.
    pub async fn run_maintenance(&self) -> Result<MaintenanceSummary, ModerationError> {
        let (expired, restored_users) = self
            .repository
            .expire_due_sanctions()
            .await
            .map_err(internal)?;
        for user_id in &restored_users {
            self.authorization.invalidate(*user_id).await;
        }

        let mut reminders = 0;
        for (sanction_id, user_id, _sanction_type, ends_at) in self
            .repository
            .sanctions_expiring_within(24)
            .await
            .map_err(internal)?
        {
            let dedup_key = format!("sanction_expiring:{sanction_id}");
            let ends_text = ends_at
                .map(|value| value.format("%Y-%m-%d %H:%M").to_string())
                .unwrap_or_default();
            let result = self
                .notifications
                .send(crate::repositories::NewNotification {
                    user_id,
                    actor_id: None,
                    notification_type: crate::models::NotificationType::SanctionExpiring,
                    title: "处罚即将结束",
                    content: &format!("你的处罚将于 {ends_text} 结束。"),
                    target_type: None,
                    target_id: None,
                    metadata: json!({ "sanction_id": sanction_id, "href": "/profile/sanctions" }),
                    dedup_key: Some(&dedup_key),
                })
                .await;
            if result.is_ok() {
                reminders += 1;
            }
        }

        let purged_snapshots = self
            .repository
            .purge_snapshots_older_than(90)
            .await
            .map_err(internal)?;
        let purged_hits = self
            .repository
            .purge_rule_hits_older_than(180)
            .await
            .map_err(internal)?;
        Ok(MaintenanceSummary {
            expired_sanctions: expired.len(),
            restored_users: restored_users.len(),
            expiry_reminders: reminders,
            purged_snapshots,
            purged_rule_hits: purged_hits,
        })
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct MaintenanceSummary {
    pub expired_sanctions: usize,
    pub restored_users: usize,
    pub expiry_reminders: usize,
    pub purged_snapshots: i64,
    pub purged_rule_hits: i64,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn require(
    principal: &AuthenticatedPrincipal,
    permission: &'static str,
) -> Result<(), ModerationError> {
    if principal.has_permission(permission) {
        Ok(())
    } else {
        Err(ModerationError::Forbidden)
    }
}

fn require_any(
    principal: &AuthenticatedPrincipal,
    permissions: &[&'static str],
) -> Result<(), ModerationError> {
    if permissions
        .iter()
        .any(|permission| principal.has_permission(permission))
    {
        Ok(())
    } else {
        Err(ModerationError::Forbidden)
    }
}

fn require_sanction_permission(
    principal: &AuthenticatedPrincipal,
    sanction_type: SanctionType,
) -> Result<(), ModerationError> {
    let permission = match sanction_type {
        SanctionType::Warning => PERMISSION_MODERATION_USER_WARN,
        SanctionType::Mute | SanctionType::ContentRestriction => PERMISSION_MODERATION_USER_MUTE,
        SanctionType::Suspension => PERMISSION_MODERATION_USER_SUSPEND,
        SanctionType::Ban => PERMISSION_MODERATION_USER_BAN,
    };
    require(principal, permission)
}

fn principal_role_priority(principal: &AuthenticatedPrincipal) -> i16 {
    match principal.role.as_str() {
        "super_administrator" => 40,
        "administrator" => 30,
        "senior_moderator" => 25,
        "moderator" => 20,
        _ => 10,
    }
}

fn reason_code_priority(reason_code: &str) -> ReportPriority {
    match reason_code {
        "illegal_content" | "violence" => ReportPriority::Urgent,
        "hate_speech" | "sexual_content" => ReportPriority::High,
        _ => ReportPriority::Normal,
    }
}

fn risk_to_priority(risk: i32) -> ReportPriority {
    if risk >= 80 {
        ReportPriority::Urgent
    } else if risk >= 60 {
        ReportPriority::High
    } else if risk >= 30 {
        ReportPriority::Normal
    } else {
        ReportPriority::Low
    }
}

fn page_bounds(
    page: Option<u32>,
    page_size: Option<u32>,
) -> Result<(u32, u32, i64, i64), ModerationError> {
    let page = page.unwrap_or(1);
    let page_size = page_size.unwrap_or(DEFAULT_PAGE_SIZE);
    if page == 0 || page > MAX_PAGE {
        return Err(ModerationError::Validation("page is out of range"));
    }
    if page_size == 0 || page_size > MAX_PAGE_SIZE {
        return Err(ModerationError::Validation(
            "page size must be between 1 and 100",
        ));
    }
    let offset = i64::from((page - 1).saturating_mul(page_size));
    Ok((page, page_size, i64::from(page_size), offset))
}

fn paginate<T>(items: Vec<T>, page: u32, page_size: u32, total: i64) -> Paginated<T> {
    Paginated {
        items,
        pagination: PaginationMeta::new(page, page_size, u64::try_from(total.max(0)).unwrap_or(0)),
    }
}

fn normalize_search(value: Option<String>) -> Result<Option<String>, ModerationError> {
    let value = value
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty());
    if value
        .as_ref()
        .is_some_and(|value| value.chars().count() > 100)
    {
        return Err(ModerationError::Validation("search query is too long"));
    }
    Ok(value)
}

fn normalize_filter(value: Option<String>) -> Result<Option<String>, ModerationError> {
    Ok(value
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty()))
}

fn normalize_reason(value: Option<String>) -> Result<Option<String>, ModerationError> {
    let value = value
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty());
    if value
        .as_ref()
        .is_some_and(|value| value.chars().count() > 500)
    {
        return Err(ModerationError::Validation("reason is too long"));
    }
    Ok(value)
}

fn resolve_assignee_filter(
    value: Option<&str>,
    principal: &AuthenticatedPrincipal,
) -> Result<(Option<Uuid>, bool), ModerationError> {
    match value {
        None => Ok((None, false)),
        Some("me") => Ok((Some(principal.user_id), false)),
        Some("unassigned") => Ok((None, true)),
        Some(value) => Uuid::parse_str(value)
            .map(|id| (Some(id), false))
            .map_err(|_| ModerationError::Validation("invalid assignee filter")),
    }
}

fn strip_internal_note(mut sanction: SanctionItem) -> SanctionItem {
    sanction.internal_note = None;
    sanction
}

fn batch_error_code(error: &ModerationError) -> &'static str {
    match error {
        ModerationError::Validation(_) => "validation_error",
        ModerationError::NotFound => "not_found",
        ModerationError::Forbidden => "permission_denied",
        ModerationError::RateLimited => "rate_limited",
        ModerationError::Conflict(_) => "conflict",
        ModerationError::Internal(_) => "internal_error",
    }
}

fn locked_flag(action: &ModerationActionKind) -> Option<bool> {
    match action {
        ModerationActionKind::Lock => Some(true),
        ModerationActionKind::Unlock => Some(false),
        _ => None,
    }
}

fn pinned_flag(action: &ModerationActionKind) -> Option<bool> {
    match action {
        ModerationActionKind::Pin => Some(true),
        ModerationActionKind::Unpin => Some(false),
        _ => None,
    }
}

fn sensitive_flag(action: &ModerationActionKind) -> Option<bool> {
    match action {
        ModerationActionKind::MarkSensitive => Some(true),
        ModerationActionKind::UnmarkSensitive => Some(false),
        _ => None,
    }
}

fn interactions_flag(action: &ModerationActionKind) -> Option<bool> {
    match action {
        ModerationActionKind::RestrictInteractions => Some(true),
        ModerationActionKind::UnrestrictInteractions => Some(false),
        _ => None,
    }
}

fn internal(error: impl Into<anyhow::Error>) -> ModerationError {
    ModerationError::Internal(error.into())
}

// --- auto-moderation text helpers ---

fn rule_applies_to(rule: &RuleItem, target_type: &str) -> bool {
    rule.target_type == "all" || rule.target_type == target_type
}

impl RuleItem {
    fn config_array(&self, key: &str) -> Vec<String> {
        self.config
            .get(key)
            .and_then(|value| value.as_array())
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| item.as_str())
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default()
    }

    fn config_i64(&self, key: &str, default: i64) -> i64 {
        self.config
            .get(key)
            .and_then(|value| value.as_i64())
            .unwrap_or(default)
    }
}

/// Normalize text to defeat common keyword-evasion tricks:
/// lowercase, full-width→half-width, strip whitespace and punctuation.
fn normalize_text(value: &str) -> String {
    value
        .chars()
        .map(|ch| match ch {
            'Ａ'..='Ｚ' => char::from_u32(ch as u32 - 0xFEE0).unwrap_or(ch),
            'ａ'..='ｚ' => char::from_u32(ch as u32 - 0xFEE0).unwrap_or(ch),
            '０'..='９' => char::from_u32(ch as u32 - 0xFEE0).unwrap_or(ch),
            '！'..='～' => char::from_u32(ch as u32 - 0xFEE0).unwrap_or(ch),
            _ => ch,
        })
        .collect::<String>()
        .to_lowercase()
        .chars()
        .filter(|ch| !ch.is_whitespace() && !ch.is_ascii_punctuation() && !ch.is_control())
        .collect()
}

fn extract_domains(normalized: &str) -> Vec<String> {
    // crude URL/domain extraction on the normalized (punctuation-stripped) text
    let mut domains = Vec::new();
    for token in normalized.split(['/', ' ', '。']) {
        if token.contains("http") || token.contains("www") {
            let domain = token
                .trim_start_matches("http")
                .trim_start_matches("https")
                .trim_start_matches(':')
                .trim_start_matches('/')
                .trim_start_matches("www");
            if domain.len() >= 4 && domain.contains('.') {
                domains.push(domain.to_owned());
            }
        }
    }
    domains
}

fn user_created_within_hours(user: &crate::repositories::ModUserRow, hours: i64) -> bool {
    let age = Utc::now() - user.created_at;
    age < Duration::hours(hours)
}

#[cfg(test)]
mod tests {
    use super::{
        normalize_text, reason_code_priority, ReportPriority, RuleAction, ScreeningDecision,
    };

    #[test]
    fn normalizes_evasion_attempts() {
        assert_eq!(normalize_text("Fuck You"), "fuckyou");
        assert_eq!(normalize_text("Ｈｅｌｌｏ 世界！"), "hello世界");
        assert_eq!(normalize_text("s e x 视频"), "sex视频");
    }

    #[test]
    fn reason_priorities_escalate() {
        assert_eq!(
            reason_code_priority("illegal_content"),
            ReportPriority::Urgent
        );
        assert_eq!(reason_code_priority("hate_speech"), ReportPriority::High);
        assert_eq!(reason_code_priority("spam"), ReportPriority::Normal);
    }

    #[test]
    fn screening_decision_defaults_to_allow() {
        let decision = ScreeningDecision::default();
        assert!(decision.is_allowed());
        assert_eq!(decision.action, RuleAction::Allow);
    }
}
