//! Phase 13: community moderation domain types.
//! Enums mirror DB CHECK constraints; responses follow the existing ApiResponse contract.

use std::str::FromStr;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{ReportStatus, ReportTargetType};

// ---------------------------------------------------------------------------
// Report reasons & priority
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReportReason {
    Spam,
    Harassment,
    HateSpeech,
    Violence,
    SexualContent,
    IllegalContent,
    PrivacyViolation,
    Misinformation,
    Copyright,
    Other,
}

impl ReportReason {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Spam => "spam",
            Self::Harassment => "harassment",
            Self::HateSpeech => "hate_speech",
            Self::Violence => "violence",
            Self::SexualContent => "sexual_content",
            Self::IllegalContent => "illegal_content",
            Self::PrivacyViolation => "privacy_violation",
            Self::Misinformation => "misinformation",
            Self::Copyright => "copyright",
            Self::Other => "other",
        }
    }

    /// High-risk reasons escalate the report/case priority automatically.
    pub const fn priority(self) -> ReportPriority {
        match self {
            Self::IllegalContent | Self::Violence => ReportPriority::Urgent,
            Self::HateSpeech | Self::SexualContent => ReportPriority::High,
            Self::Harassment | Self::PrivacyViolation => ReportPriority::Normal,
            _ => ReportPriority::Normal,
        }
    }

    pub const fn risk_score(self) -> u32 {
        match self.priority() {
            ReportPriority::Urgent => 80,
            ReportPriority::High => 60,
            ReportPriority::Normal => 30,
            ReportPriority::Low => 10,
        }
    }
}

impl FromStr for ReportReason {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "spam" => Ok(Self::Spam),
            "harassment" => Ok(Self::Harassment),
            "hate_speech" => Ok(Self::HateSpeech),
            "violence" => Ok(Self::Violence),
            "sexual_content" => Ok(Self::SexualContent),
            "illegal_content" => Ok(Self::IllegalContent),
            "privacy_violation" => Ok(Self::PrivacyViolation),
            "misinformation" => Ok(Self::Misinformation),
            "copyright" => Ok(Self::Copyright),
            "other" => Ok(Self::Other),
            _ => Err("unknown report reason"),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReportPriority {
    Low,
    Normal,
    High,
    Urgent,
}

impl ReportPriority {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Normal => "normal",
            Self::High => "high",
            Self::Urgent => "urgent",
        }
    }
}

impl FromStr for ReportPriority {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "low" => Ok(Self::Low),
            "normal" => Ok(Self::Normal),
            "high" => Ok(Self::High),
            "urgent" => Ok(Self::Urgent),
            _ => Err("unknown report priority"),
        }
    }
}

impl std::cmp::Ord for ReportPriority {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.rank().cmp(&other.rank())
    }
}

impl std::cmp::PartialOrd for ReportPriority {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl ReportPriority {
    fn rank(self) -> u8 {
        match self {
            Self::Low => 0,
            Self::Normal => 1,
            Self::High => 2,
            Self::Urgent => 3,
        }
    }
}

// ---------------------------------------------------------------------------
// Moderation cases
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CaseStatus {
    Open,
    Reviewing,
    Closed,
}

impl CaseStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Reviewing => "reviewing",
            Self::Closed => "closed",
        }
    }
}

impl FromStr for CaseStatus {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "open" => Ok(Self::Open),
            "reviewing" => Ok(Self::Reviewing),
            "closed" => Ok(Self::Closed),
            _ => Err("unknown case status"),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CaseSource {
    Report,
    Auto,
    Manual,
}

impl CaseSource {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Report => "report",
            Self::Auto => "auto",
            Self::Manual => "manual",
        }
    }
}

impl FromStr for CaseSource {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "report" => Ok(Self::Report),
            "auto" => Ok(Self::Auto),
            "manual" => Ok(Self::Manual),
            _ => Err("unknown case source"),
        }
    }
}

// ---------------------------------------------------------------------------
// Sanctions
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SanctionType {
    Warning,
    ContentRestriction,
    Mute,
    Suspension,
    Ban,
}

impl SanctionType {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Warning => "warning",
            Self::ContentRestriction => "content_restriction",
            Self::Mute => "mute",
            Self::Suspension => "suspension",
            Self::Ban => "ban",
        }
    }

    /// Restrictions implied when the requester does not override them.
    pub const fn default_restrictions(self) -> &'static [&'static str] {
        match self {
            Self::Warning => &[],
            Self::ContentRestriction => &[RESTRICTION_NO_TOPICS, RESTRICTION_NO_COMMENTS],
            Self::Mute => &[RESTRICTION_NO_TOPICS, RESTRICTION_NO_COMMENTS],
            Self::Suspension => &[
                RESTRICTION_NO_TOPICS,
                RESTRICTION_NO_COMMENTS,
                RESTRICTION_NO_REPORTS,
                RESTRICTION_NO_UPLOADS,
            ],
            Self::Ban => &[
                RESTRICTION_NO_TOPICS,
                RESTRICTION_NO_COMMENTS,
                RESTRICTION_NO_REPORTS,
                RESTRICTION_NO_UPLOADS,
            ],
        }
    }
}

impl FromStr for SanctionType {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "warning" => Ok(Self::Warning),
            "content_restriction" => Ok(Self::ContentRestriction),
            "mute" => Ok(Self::Mute),
            "suspension" => Ok(Self::Suspension),
            "ban" => Ok(Self::Ban),
            _ => Err("unknown sanction type"),
        }
    }
}

pub const RESTRICTION_NO_TOPICS: &str = "no_topics";
pub const RESTRICTION_NO_COMMENTS: &str = "no_comments";
pub const RESTRICTION_NO_REPORTS: &str = "no_reports";
pub const RESTRICTION_NO_UPLOADS: &str = "no_uploads";

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SanctionStatus {
    Scheduled,
    Active,
    Expired,
    Revoked,
}

impl SanctionStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Scheduled => "scheduled",
            Self::Active => "active",
            Self::Expired => "expired",
            Self::Revoked => "revoked",
        }
    }
}

impl FromStr for SanctionStatus {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "scheduled" => Ok(Self::Scheduled),
            "active" => Ok(Self::Active),
            "expired" => Ok(Self::Expired),
            "revoked" => Ok(Self::Revoked),
            _ => Err("unknown sanction status"),
        }
    }
}

// ---------------------------------------------------------------------------
// Appeals
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AppealType {
    Sanction,
    Content,
}

impl AppealType {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Sanction => "sanction",
            Self::Content => "content",
        }
    }
}

impl FromStr for AppealType {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "sanction" => Ok(Self::Sanction),
            "content" => Ok(Self::Content),
            _ => Err("unknown appeal type"),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AppealStatus {
    Pending,
    Reviewing,
    Approved,
    Rejected,
    Cancelled,
}

impl AppealStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Reviewing => "reviewing",
            Self::Approved => "approved",
            Self::Rejected => "rejected",
            Self::Cancelled => "cancelled",
        }
    }
}

impl FromStr for AppealStatus {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "pending" => Ok(Self::Pending),
            "reviewing" => Ok(Self::Reviewing),
            "approved" => Ok(Self::Approved),
            "rejected" => Ok(Self::Rejected),
            "cancelled" => Ok(Self::Cancelled),
            _ => Err("unknown appeal status"),
        }
    }
}

// ---------------------------------------------------------------------------
// Auto-moderation rules
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleType {
    Keyword,
    UrlDomain,
    Rate,
    Duplicate,
    NewUser,
    HighFrequency,
}

impl RuleType {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Keyword => "keyword",
            Self::UrlDomain => "url_domain",
            Self::Rate => "rate",
            Self::Duplicate => "duplicate",
            Self::NewUser => "new_user",
            Self::HighFrequency => "high_frequency",
        }
    }
}

impl FromStr for RuleType {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "keyword" => Ok(Self::Keyword),
            "url_domain" => Ok(Self::UrlDomain),
            "rate" => Ok(Self::Rate),
            "duplicate" => Ok(Self::Duplicate),
            "new_user" => Ok(Self::NewUser),
            "high_frequency" => Ok(Self::HighFrequency),
            _ => Err("unknown rule type"),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleAction {
    #[default]
    Allow,
    Flag,
    Queue,
    Collapse,
    Hide,
    Reject,
    RateLimit,
}

impl RuleAction {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Allow => "allow",
            Self::Flag => "flag",
            Self::Queue => "queue",
            Self::Collapse => "collapse",
            Self::Hide => "hide",
            Self::Reject => "reject",
            Self::RateLimit => "rate_limit",
        }
    }

    /// Strength ordering used to pick the final decision.
    pub const fn strength(self) -> u8 {
        match self {
            Self::Allow => 0,
            Self::Flag => 1,
            Self::Queue => 2,
            Self::Collapse => 3,
            Self::Hide => 4,
            Self::RateLimit => 5,
            Self::Reject => 6,
        }
    }
}

impl FromStr for RuleAction {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "allow" => Ok(Self::Allow),
            "flag" => Ok(Self::Flag),
            "queue" => Ok(Self::Queue),
            "collapse" => Ok(Self::Collapse),
            "hide" => Ok(Self::Hide),
            "reject" => Ok(Self::Reject),
            "rate_limit" => Ok(Self::RateLimit),
            _ => Err("unknown rule action"),
        }
    }
}

// ---------------------------------------------------------------------------
// Requests
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Deserialize)]
pub struct CreateReportRequestV2 {
    pub target_type: ReportTargetType,
    pub target_id: Uuid,
    /// New-style enum reason. When absent, falls back to the legacy `reason` text.
    pub reason_code: Option<ReportReason>,
    /// Legacy free-text reason (kept for older clients).
    pub reason: Option<String>,
    pub details: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct ModerationReportQuery {
    pub q: Option<String>,
    pub status: Option<String>,
    pub target_type: Option<String>,
    pub reason: Option<String>,
    pub priority: Option<String>,
    pub assignee: Option<String>,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct CaseQuery {
    pub q: Option<String>,
    pub status: Option<String>,
    pub priority: Option<String>,
    pub source: Option<String>,
    pub target_type: Option<String>,
    pub assignee: Option<String>,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct CaseActionRequest {
    pub action: ModerationActionKind,
    pub reason: Option<String>,
    pub case_id: Option<Uuid>,
    /// Required for move_category actions.
    pub target_category_id: Option<Uuid>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct BatchReportItem {
    pub id: Uuid,
    pub action: String,
    pub note: Option<String>,
    pub content_action: Option<ModerationActionKind>,
    pub action_reason: Option<String>,
    pub duplicate_of: Option<Uuid>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ModerationActionKind {
    Hide,
    Restore,
    Delete,
    Lock,
    Unlock,
    Pin,
    Unpin,
    MoveCategory,
    MarkSensitive,
    UnmarkSensitive,
    RestrictInteractions,
    UnrestrictInteractions,
    Collapse,
    Uncollapse,
    RestrictReplies,
    UnrestrictReplies,
}

impl ModerationActionKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Hide => "hide",
            Self::Restore => "restore",
            Self::Delete => "delete",
            Self::Lock => "lock",
            Self::Unlock => "unlock",
            Self::Pin => "pin",
            Self::Unpin => "unpin",
            Self::MoveCategory => "move_category",
            Self::MarkSensitive => "mark_sensitive",
            Self::UnmarkSensitive => "unmark_sensitive",
            Self::RestrictInteractions => "restrict_interactions",
            Self::UnrestrictInteractions => "unrestrict_interactions",
            Self::Collapse => "collapse",
            Self::Uncollapse => "uncollapse",
            Self::RestrictReplies => "restrict_replies",
            Self::UnrestrictReplies => "unrestrict_replies",
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct ContentActionResult {
    pub action: &'static str,
    pub target_type: &'static str,
    pub target_id: Uuid,
    pub before_status: String,
    pub after_status: String,
    pub case_id: Option<Uuid>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct CreateSanctionRequest {
    pub sanction_type: SanctionType,
    pub reason: String,
    pub user_visible_reason: Option<String>,
    pub internal_note: Option<String>,
    #[serde(default)]
    pub restrictions: Vec<String>,
    pub starts_at: Option<DateTime<Utc>>,
    pub ends_at: Option<DateTime<Utc>>,
    pub case_id: Option<Uuid>,
    pub report_id: Option<Uuid>,
    pub related_content_type: Option<String>,
    pub related_content_id: Option<Uuid>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct RevokeSanctionRequest {
    pub reason: Option<String>,
    /// Required when revoking a permanent ban.
    pub confirm: Option<bool>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct SanctionListQuery {
    pub user_id: Option<Uuid>,
    pub status: Option<String>,
    pub sanction_type: Option<String>,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct CreateAppealRequest {
    pub sanction_id: Option<Uuid>,
    pub content_type: Option<String>,
    pub content_id: Option<Uuid>,
    pub reason: String,
    pub details: Option<String>,
    #[serde(default)]
    pub evidence: Vec<Uuid>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ReviewAppealRequest {
    pub decision: AppealStatus,
    pub note: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct AppealListQuery {
    pub status: Option<String>,
    pub q: Option<String>,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct RuleRequest {
    pub name: String,
    pub rule_type: RuleType,
    #[serde(default = "default_target_all")]
    pub target_type: String,
    pub priority: Option<i32>,
    pub enabled: Option<bool>,
    pub risk_score: Option<i32>,
    pub action: RuleAction,
    pub config: Option<serde_json::Value>,
}

fn default_target_all() -> String {
    "all".into()
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct RuleListQuery {
    pub enabled: Option<bool>,
    pub rule_type: Option<String>,
    pub action: Option<String>,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct NoteRequest {
    pub note: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ResolveReportRequestV2 {
    /// Content action to take alongside resolution (optional).
    pub action: Option<ModerationActionKind>,
    pub action_reason: Option<String>,
    pub resolution_note: Option<String>,
}

// ---------------------------------------------------------------------------
// Responses
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize)]
pub struct ReportItemV2 {
    pub id: Uuid,
    pub reporter_id: Uuid,
    pub reporter_username: String,
    pub target_type: ReportTargetType,
    pub target_id: Uuid,
    pub reason: String,
    pub details: Option<String>,
    pub status: ReportStatus,
    pub priority: ReportPriority,
    pub risk_score: i32,
    pub case_id: Option<Uuid>,
    pub duplicate_of: Option<Uuid>,
    pub handler_id: Option<Uuid>,
    pub handler_username: Option<String>,
    pub resolution_note: Option<String>,
    pub handled_at: Option<DateTime<Utc>>,
    pub cancelled_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CaseItem {
    pub id: Uuid,
    pub target_type: ReportTargetType,
    pub target_id: Uuid,
    pub status: CaseStatus,
    pub priority: ReportPriority,
    pub risk_score: i32,
    pub source: CaseSource,
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

#[derive(Clone, Debug, Serialize)]
pub struct CaseDetail {
    pub id: Uuid,
    pub target_type: ReportTargetType,
    pub target_id: Uuid,
    pub status: CaseStatus,
    pub priority: ReportPriority,
    pub risk_score: i32,
    pub source: CaseSource,
    pub assignee_id: Option<Uuid>,
    pub assignee_username: Option<String>,
    pub opened_by: Option<Uuid>,
    pub opened_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub related_reports: Vec<ReportItemV2>,
    pub actions: Vec<ModerationActionItem>,
    pub snapshots: Vec<ContentSnapshotItem>,
    pub rule_hits: Vec<RuleHitItem>,
    pub notes: Vec<ModerationNoteItem>,
    pub author_violations: Vec<ViolationSummary>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ModerationActionItem {
    pub id: Uuid,
    pub case_id: Option<Uuid>,
    pub action: String,
    pub target_type: String,
    pub target_id: Uuid,
    pub before_status: Option<String>,
    pub after_status: Option<String>,
    pub reason: Option<String>,
    pub operator_id: Option<Uuid>,
    pub operator_username: Option<String>,
    pub report_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ContentSnapshotItem {
    pub id: Uuid,
    pub target_type: String,
    pub target_id: Uuid,
    pub title: Option<String>,
    pub content: Option<String>,
    pub summary: Option<String>,
    pub status: Option<String>,
    pub reason: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize)]
pub struct RuleHitItem {
    pub id: Uuid,
    pub rule_id: Uuid,
    pub rule_name: String,
    pub rule_type: String,
    pub target_type: String,
    pub risk_score: i32,
    pub action: String,
    pub content_snippet: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ModerationNoteItem {
    pub id: Uuid,
    pub case_id: Uuid,
    pub author_id: Uuid,
    pub author_username: String,
    pub note: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ViolationSummary {
    pub sanction_type: SanctionType,
    pub status: SanctionStatus,
    pub reason: String,
    pub issued_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize)]
pub struct SanctionItem {
    pub id: Uuid,
    pub user_id: Uuid,
    pub username: String,
    pub sanction_type: SanctionType,
    pub reason: String,
    pub user_visible_reason: Option<String>,
    /// Internal note is only returned to moderators (admin endpoints).
    pub internal_note: Option<String>,
    pub restrictions: Vec<String>,
    pub starts_at: DateTime<Utc>,
    pub ends_at: Option<DateTime<Utc>>,
    pub is_permanent: bool,
    pub status: SanctionStatus,
    pub issued_by: Option<Uuid>,
    pub issuer_username: Option<String>,
    pub case_id: Option<Uuid>,
    pub report_id: Option<Uuid>,
    pub related_content_type: Option<String>,
    pub related_content_id: Option<Uuid>,
    pub revoked_by: Option<Uuid>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub revoke_reason: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize)]
pub struct AppealItem {
    pub id: Uuid,
    pub user_id: Uuid,
    pub username: String,
    pub appeal_type: AppealType,
    pub sanction_id: Option<Uuid>,
    pub content_type: Option<String>,
    pub content_id: Option<Uuid>,
    pub reason: String,
    pub details: Option<String>,
    pub evidence: Vec<Uuid>,
    pub status: AppealStatus,
    pub reviewer_id: Option<Uuid>,
    pub reviewer_username: Option<String>,
    pub review_note: Option<String>,
    pub reviewed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RuleItem {
    pub id: Uuid,
    pub name: String,
    pub rule_type: RuleType,
    pub target_type: String,
    pub priority: i32,
    pub enabled: bool,
    pub risk_score: i32,
    pub action: RuleAction,
    pub config: serde_json::Value,
    pub hit_count: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Governance metrics (admin JSON endpoint)
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize)]
pub struct CountItem {
    pub label: String,
    pub count: i64,
}

#[derive(Clone, Debug, Serialize)]
pub struct DailyMetric {
    pub date: String,
    pub reports: i64,
    pub actions: i64,
    pub sanctions: i64,
}

#[derive(Clone, Debug, Serialize)]
pub struct GovernanceMetrics {
    pub reports_today: i64,
    pub reports_pending: i64,
    pub reports_resolved_7d: i64,
    pub avg_review_hours: Option<f64>,
    pub reports_by_reason: Vec<CountItem>,
    pub reports_by_target: Vec<CountItem>,
    pub auto_hits_7d: i64,
    pub auto_hidden: i64,
    pub manual_restores: i64,
    pub warnings_total: i64,
    pub mutes_total: i64,
    pub suspensions_total: i64,
    pub bans_total: i64,
    pub sanctions_active: i64,
    pub appeals_total: i64,
    pub appeals_pending: i64,
    pub appeals_approved: i64,
    pub appeals_rejected: i64,
    pub queue_backlog: i64,
    pub moderator_actions_7d: Vec<CountItem>,
    pub daily_14d: Vec<DailyMetric>,
}
