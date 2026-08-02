use std::str::FromStr;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::TopicAuthorSummary;

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CommentStatus {
    #[default]
    Published,
    Deleted,
    Hidden,
    PendingReview,
}

impl CommentStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Published => "published",
            Self::Deleted => "deleted",
            Self::Hidden => "hidden",
            Self::PendingReview => "pending_review",
        }
    }
}

impl FromStr for CommentStatus {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "published" => Ok(Self::Published),
            "deleted" => Ok(Self::Deleted),
            "hidden" => Ok(Self::Hidden),
            "pending_review" => Ok(Self::PendingReview),
            _ => Err("unknown comment status"),
        }
    }
}

impl CommentStatus {
    pub fn from_legacy(value: &str) -> Result<Self, &'static str> {
        match value {
            "published" => Ok(Self::Published),
            "deleted" => Ok(Self::Deleted),
            _ => Err("unknown comment status"),
        }
    }
}

#[derive(sqlx::FromRow)]
pub struct CommentRecord {
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
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct CommentStats {
    pub likes: i64,
    pub replies: i64,
}

#[derive(Clone, Debug, Serialize)]
pub struct CommentNode {
    pub id: Uuid,
    pub topic_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub content: String,
    pub author: TopicAuthorSummary,
    pub stats: CommentStats,
    pub edited_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub liked_by_me: bool,
    pub replies: Vec<CommentNode>,
}

#[derive(Default, Deserialize)]
pub struct CommentListQuery {
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

#[derive(Deserialize)]
pub struct CreateCommentRequest {
    pub content: String,
}

#[derive(Deserialize)]
pub struct UpdateCommentRequest {
    pub content: String,
}
