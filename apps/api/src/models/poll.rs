//! Phase 14: poll models — polls attach to topics (1:1), options, votes,
//! results, and admin list DTOs.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::CategorySummary;

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PollStatus {
    #[default]
    Active,
    Closed,
}

impl PollStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Closed => "closed",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PollType {
    #[default]
    Standard,
}

impl PollType {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Standard => "standard",
        }
    }
}

/// Draft payload embedded in `CreateTopicRequest` — created atomically with the
/// topic so a poll never references a topic that failed to publish.
#[derive(Clone, Debug, Deserialize)]
pub struct CreatePollDraft {
    pub title: String,
    pub description: Option<String>,
    pub multiple_choice: Option<bool>,
    pub anonymous: Option<bool>,
    /// Whether voters may cancel cast votes (default true).
    pub allow_cancel: Option<bool>,
    pub max_choices: Option<i32>,
    pub expires_at: Option<DateTime<Utc>>,
    /// 2..=20 options, each 1..=500 chars, deduplicated.
    pub options: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct UpdatePollRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub expires_at: Option<Option<DateTime<Utc>>>,
    pub allow_cancel: Option<bool>,
    /// New options appended to the poll (each 1..=500 chars).
    #[serde(default)]
    pub options_to_add: Vec<String>,
    /// Existing options to remove — only options with zero votes may be removed.
    #[serde(default)]
    pub option_ids_to_remove: Vec<Uuid>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct VotePollRequest {
    /// Single choice: exactly one option. Multi choice: 1..=max_choices options.
    #[serde(default)]
    pub option_ids: Vec<Uuid>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct CancelVoteRequest {
    /// Optional: remove only this option (multi choice). Omit to cancel all.
    pub option_id: Option<Uuid>,
}

#[derive(sqlx::FromRow)]
pub struct PollRecord {
    pub id: Uuid,
    pub topic_id: Uuid,
    pub author_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub poll_type: String,
    pub status: String,
    pub multiple_choice: bool,
    pub anonymous: bool,
    pub allow_cancel: bool,
    pub max_choices: i32,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
pub struct PollOptionRecord {
    pub id: Uuid,
    pub poll_id: Uuid,
    pub content: String,
    pub sort_order: i32,
    pub vote_count: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize)]
pub struct PollOptionItem {
    pub id: Uuid,
    pub content: String,
    pub sort_order: i32,
    pub vote_count: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PollVoterItem {
    pub user_id: Uuid,
    pub username: String,
    pub nickname: Option<String>,
    pub avatar: Option<String>,
    pub option_id: Uuid,
}

/// Raw join row used by the repository for public voter lists.
#[derive(sqlx::FromRow)]
pub struct PollVoterRow {
    pub option_id: Uuid,
    pub user_id: Uuid,
    pub username: String,
    pub nickname: Option<String>,
    pub avatar: Option<String>,
}
/// Full poll as returned by GET /topics/{topic_id}/poll and GET /polls/{id}.
#[derive(Clone, Debug, Serialize)]
pub struct PollDetail {
    pub id: Uuid,
    pub topic_id: Uuid,
    pub topic_slug: String,
    pub topic_title: String,
    pub author_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub poll_type: PollType,
    pub status: PollStatus,
    pub multiple_choice: bool,
    pub anonymous: bool,
    /// Whether voters may cancel cast votes.
    pub allow_cancel: bool,
    pub max_choices: i32,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub options: Vec<PollOptionItem>,
    pub total_votes: i64,
    pub participant_count: i64,
    /// Option ids this viewer has voted for (empty for anonymous viewers).
    #[serde(default)]
    pub my_votes: Vec<Uuid>,
    /// Viewer may cast / change votes.
    pub can_vote: bool,
    /// Viewer may close / edit the poll (author or elevated role).
    pub can_manage: bool,
}

/// GET /polls/{id}/results — aggregated statistics.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PollResults {
    pub poll_id: Uuid,
    pub topic_id: Uuid,
    pub topic_slug: String,
    pub topic_title: String,
    pub title: String,
    pub status: PollStatus,
    pub multiple_choice: bool,
    pub anonymous: bool,
    pub expires_at: Option<DateTime<Utc>>,
    pub total_votes: i64,
    pub participant_count: i64,
    pub options: Vec<PollResultOption>,
    /// Present only when the poll is public (anonymous = false).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub voters: Option<Vec<PollVoterItem>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PollResultOption {
    pub option_id: Uuid,
    pub content: String,
    pub vote_count: i64,
    /// 0..=100, rounded to 1 decimal.
    pub percentage: f64,
}

/// GET /polls/hot — cached popular polls.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HotPollItem {
    pub poll_id: Uuid,
    pub topic_id: Uuid,
    pub topic_slug: String,
    pub topic_title: String,
    pub poll_title: String,
    pub participant_count: i64,
    pub option_count: i64,
    pub is_closed: bool,
    pub category: CategorySummary,
    pub created_at: DateTime<Utc>,
}

/// Admin list row.
#[derive(Clone, Debug, Serialize)]
pub struct AdminPollItem {
    pub id: Uuid,
    pub topic_id: Uuid,
    pub topic_title: String,
    pub topic_slug: String,
    pub title: String,
    pub status: String,
    pub multiple_choice: bool,
    pub anonymous: bool,
    pub max_choices: i32,
    pub option_count: i64,
    pub participant_count: i64,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub author_id: Uuid,
    pub author_username: String,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct AdminPollListQuery {
    pub q: Option<String>,
    pub status: Option<String>,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}
