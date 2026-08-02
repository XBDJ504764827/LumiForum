use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{RoleSummary, UploadCategory, UserStatus};

#[derive(Clone, Debug, Serialize)]
pub struct DailyCount {
    pub date: String,
    pub count: i64,
}

#[derive(Clone, Debug, Serialize)]
pub struct HotTopicStat {
    pub id: Uuid,
    pub title: String,
    pub slug: String,
    pub view_count: i64,
    pub reply_count: i64,
    pub like_count: i64,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AdminDashboardRange {
    #[default]
    #[serde(rename = "today")]
    Today,
    #[serde(rename = "7d")]
    Days7,
    #[serde(rename = "30d")]
    Days30,
}

impl AdminDashboardRange {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Today => "today",
            Self::Days7 => "7d",
            Self::Days30 => "30d",
        }
    }

    pub const fn days(self) -> i64 {
        match self {
            Self::Today => 1,
            Self::Days7 => 7,
            Self::Days30 => 30,
        }
    }
}

impl std::str::FromStr for AdminDashboardRange {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "today" => Ok(Self::Today),
            "7d" => Ok(Self::Days7),
            "30d" => Ok(Self::Days30),
            _ => Err("unknown dashboard range"),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct HotCategoryStat {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub topic_count: i64,
    pub comment_count: i64,
}

#[derive(Clone, Debug, Serialize)]
pub struct SystemStats {
    pub api_requests_total: u64,
    pub ws_connections: usize,
    pub online_users: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct AdminDashboard {
    // Users
    pub users_total: i64,
    pub users_today: i64,
    pub active_users_today: i64,
    pub active_users_7d: i64,
    pub online_users: usize,
    // Content
    pub topics_total: i64,
    pub topics_today: i64,
    pub comments_total: i64,
    pub comments_today: i64,
    pub polls_total: i64,
    pub uploads_total: i64,
    pub storage_bytes: i64,
    // Reports
    pub reports_open: i64,
    pub reports_total: i64,
    // System
    pub api_requests_total: u64,
    pub ws_connections: usize,
    // Trends (range-aware)
    pub range: &'static str,
    pub registrations: Vec<DailyCount>,
    pub topics: Vec<DailyCount>,
    pub comments: Vec<DailyCount>,
    pub hot_topics: Vec<HotTopicStat>,
    pub hot_categories: Vec<HotCategoryStat>,
}

#[derive(Clone, Debug, Serialize)]
pub struct AdminUserItem {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub avatar: Option<String>,
    pub nickname: Option<String>,
    pub role: RoleSummary,
    pub status: UserStatus,
    pub email_verified: bool,
    pub followers_count: i64,
    pub following_count: i64,
    pub last_login_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct AdminUserListQuery {
    pub q: Option<String>,
    pub status: Option<UserStatus>,
    pub role: Option<String>,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct AdminUserUpdateRequest {
    pub status: Option<UserStatus>,
    pub role: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct RoleOption {
    pub code: String,
    pub name: String,
    pub priority: i16,
}

#[derive(Clone, Debug, Serialize)]
pub struct AdminTopicItem {
    pub id: Uuid,
    pub title: String,
    pub slug: String,
    pub status: String,
    pub summary: Option<String>,
    pub category_id: Uuid,
    pub category_name: String,
    pub category_slug: String,
    pub author_id: Uuid,
    pub author_username: String,
    pub view_count: i64,
    pub reply_count: i64,
    pub like_count: i64,
    pub is_pinned: bool,
    pub is_featured: bool,
    pub is_locked: bool,
    pub deleted_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct AdminTopicListQuery {
    pub q: Option<String>,
    pub status: Option<String>,
    pub category_id: Option<Uuid>,
    /// latest | hot | most_reported | violating
    pub sort: Option<String>,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct AdminTopicUpdateRequest {
    pub status: Option<String>,
    pub is_pinned: Option<bool>,
    pub is_featured: Option<bool>,
    pub is_locked: Option<bool>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct AdminCommentListQuery {
    pub q: Option<String>,
    pub status: Option<String>,
    pub topic_id: Option<Uuid>,
    /// reported | high_frequency
    pub filter: Option<String>,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

#[derive(Clone, Debug, Serialize)]
pub struct AdminCommentItem {
    pub id: Uuid,
    pub topic_id: Uuid,
    pub topic_title: String,
    pub topic_slug: String,
    pub parent_id: Option<Uuid>,
    pub content: String,
    pub status: String,
    pub author_id: Uuid,
    pub author_username: String,
    pub like_count: i64,
    pub reply_count: i64,
    pub deleted_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct AdminCategoryListQuery {
    pub include_hidden: Option<bool>,
}

#[derive(Clone, Debug, Serialize)]
pub struct AdminFileItem {
    pub id: Uuid,
    pub user_id: Uuid,
    pub username: String,
    pub filename: String,
    pub original_filename: String,
    pub mime_type: String,
    pub file_size: i64,
    pub category: UploadCategory,
    pub url: Option<String>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct AdminFileListQuery {
    pub q: Option<String>,
    pub category: Option<UploadCategory>,
    pub status: Option<String>,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReportTargetType {
    Topic,
    Comment,
    User,
}

impl ReportTargetType {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Topic => "topic",
            Self::Comment => "comment",
            Self::User => "user",
        }
    }
}

impl std::str::FromStr for ReportTargetType {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "topic" => Ok(Self::Topic),
            "comment" => Ok(Self::Comment),
            "user" => Ok(Self::User),
            _ => Err("unknown report target type"),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReportStatus {
    Open,
    Reviewing,
    Resolved,
    Rejected,
    /// Merged into another report for the same target.
    Duplicate,
    /// Withdrawn by the reporter before handling.
    Cancelled,
}

impl ReportStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Reviewing => "reviewing",
            Self::Resolved => "resolved",
            Self::Rejected => "rejected",
            Self::Duplicate => "duplicate",
            Self::Cancelled => "cancelled",
        }
    }
}

impl std::str::FromStr for ReportStatus {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "open" => Ok(Self::Open),
            "reviewing" => Ok(Self::Reviewing),
            "resolved" => Ok(Self::Resolved),
            "rejected" => Ok(Self::Rejected),
            "duplicate" => Ok(Self::Duplicate),
            "cancelled" => Ok(Self::Cancelled),
            _ => Err("unknown report status"),
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct CreateReportRequest {
    pub target_type: ReportTargetType,
    pub target_id: Uuid,
    pub reason: String,
    pub details: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ResolveReportRequest {
    pub status: ReportStatus,
    pub resolution_note: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ReportItem {
    pub id: Uuid,
    pub reporter_id: Uuid,
    pub reporter_username: String,
    pub target_type: ReportTargetType,
    pub target_id: Uuid,
    pub reason: String,
    pub details: Option<String>,
    pub status: ReportStatus,
    pub handler_id: Option<Uuid>,
    pub handler_username: Option<String>,
    pub resolution_note: Option<String>,
    pub handled_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct ReportListQuery {
    pub status: Option<ReportStatus>,
    pub target_type: Option<ReportTargetType>,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

#[derive(Clone, Debug, Serialize)]
pub struct AdminLogItem {
    pub id: Uuid,
    pub admin_id: Uuid,
    pub admin_username: String,
    pub action: String,
    pub target_type: String,
    pub target_id: Option<Uuid>,
    pub summary: String,
    pub metadata: serde_json::Value,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct AdminLogListQuery {
    pub q: Option<String>,
    pub action: Option<String>,
    pub target_type: Option<String>,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

// ---------------------------------------------------------------------------
// Phase 15: user management enhancement
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize)]
pub struct LoginRecordItem {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub ip: Option<String>,
    pub user_agent: Option<String>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Serialize)]
pub struct AdminUserDetail {
    pub user: AdminUserItem,
    pub steam_id: Option<String>,
    pub steam_persona_name: Option<String>,
    pub login_count: i64,
    pub topics_count: i64,
    pub comments_count: i64,
    pub reports_made: i64,
    pub sanctions_active: i64,
    pub recent_logins: Vec<LoginRecordItem>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct AdminLoginRecordQuery {
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

// ---------------------------------------------------------------------------
// Phase 15: RBAC enhancement
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize)]
pub struct PermissionOption {
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    /// Derived group from the code prefix (e.g. "user.profile.read:self" -> "user").
    pub group: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct RolePermissionView {
    pub role_code: String,
    pub role_name: String,
    pub permissions: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct UpdateRolePermissionsRequest {
    /// Full replacement set of permission codes for the role.
    pub permission_codes: Vec<String>,
}

// ---------------------------------------------------------------------------
// Phase 15: moderation queue / analytics / system settings
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize)]
pub struct QueueReportItem {
    pub id: Uuid,
    pub reporter_username: String,
    pub target_type: ReportTargetType,
    pub target_id: Uuid,
    pub reason: String,
    pub status: ReportStatus,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize)]
pub struct QueueCaseItem {
    pub id: Uuid,
    pub target_type: String,
    pub target_id: Uuid,
    pub priority: String,
    pub source: String,
    pub opened_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize)]
pub struct QueueSummary {
    pub pending_reports: i64,
    pub reviewing_reports: i64,
    pub open_cases: i64,
    pub hidden_topics: i64,
    pub hidden_comments: i64,
    pub pending_uploads: i64,
    pub latest_reports: Vec<QueueReportItem>,
    pub latest_cases: Vec<QueueCaseItem>,
}

#[derive(Clone, Debug, Serialize)]
pub struct AdminAnalytics {
    pub days: i64,
    pub registrations: Vec<DailyCount>,
    pub topics: Vec<DailyCount>,
    pub comments: Vec<DailyCount>,
    pub polls: Vec<DailyCount>,
    /// Cumulative user growth over the window.
    pub cumulative_users: Vec<DailyCount>,
    pub hot_categories: Vec<HotCategoryStat>,
    pub hot_topics: Vec<HotTopicStat>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct AdminAnalyticsQuery {
    pub days: Option<i64>,
}

#[derive(Clone, Debug, Serialize)]
pub struct SystemSettingItem {
    pub key: String,
    pub value: serde_json::Value,
    pub description: Option<String>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct UpdateSettingsRequest {
    pub settings: Vec<SystemSettingValue>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct SystemSettingValue {
    pub key: String,
    pub value: serde_json::Value,
}

/// Public subset of settings readable by the forum frontend.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PublicSettings {
    pub site_name: String,
    pub site_description: Option<String>,
    pub registration_enabled: bool,
    pub topic_create_enabled: bool,
    pub comment_enabled: bool,
    pub upload_enabled: bool,
    pub upload_max_bytes: i64,
}
