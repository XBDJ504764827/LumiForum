use std::str::FromStr;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{CategorySummary, PatchField, RoleSummary};

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TopicStatus {
    #[default]
    Published,
    Hidden,
    Deleted,
}

impl TopicStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Published => "published",
            Self::Hidden => "hidden",
            Self::Deleted => "deleted",
        }
    }
}

impl FromStr for TopicStatus {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "published" => Ok(Self::Published),
            "hidden" => Ok(Self::Hidden),
            "deleted" => Ok(Self::Deleted),
            _ => Err("unknown topic status"),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TopicListSort {
    #[default]
    Latest,
    Hot,
    Featured,
    Pinned,
}

#[derive(sqlx::FromRow)]
pub struct TopicRecord {
    pub id: Uuid,
    pub category_id: Uuid,
    pub author_id: Uuid,
    pub title: String,
    pub slug: String,
    pub content: String,
    pub summary: Option<String>,
    pub status: String,
    pub view_count: i64,
    pub reply_count: i64,
    pub like_count: i64,
    pub is_pinned: bool,
    pub is_featured: bool,
    pub last_reply_at: Option<DateTime<Utc>>,
    pub last_reply_user_id: Option<Uuid>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize)]
pub struct TopicAuthorSummary {
    pub id: Uuid,
    pub username: String,
    pub nickname: Option<String>,
    pub avatar: Option<String>,
    pub role: RoleSummary,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct TopicStats {
    pub views: i64,
    pub replies: i64,
    pub likes: i64,
}

#[derive(Clone, Debug, Serialize)]
pub struct TopicSummary {
    pub id: Uuid,
    pub title: String,
    pub slug: String,
    pub summary: Option<String>,
    pub category: CategorySummary,
    pub author: TopicAuthorSummary,
    pub stats: TopicStats,
    pub is_pinned: bool,
    pub is_featured: bool,
    pub last_reply_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize)]
pub struct TopicDetail {
    pub id: Uuid,
    pub title: String,
    pub slug: String,
    pub content: String,
    pub summary: Option<String>,
    pub category: CategorySummary,
    pub author: TopicAuthorSummary,
    pub stats: TopicStats,
    pub is_pinned: bool,
    pub is_featured: bool,
    pub last_reply_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub liked_by_me: bool,
    pub favorited_by_me: bool,
    pub following_author: bool,
}

#[derive(Default, Deserialize)]
pub struct TopicListQuery {
    pub category: Option<String>,
    pub sort: Option<TopicListSort>,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

#[derive(Deserialize)]
pub struct CreateTopicRequest {
    pub category_id: Uuid,
    pub title: String,
    pub content: String,
    pub summary: Option<String>,
}

#[derive(Default, Deserialize)]
pub struct UpdateTopicRequest {
    pub category_id: Option<Uuid>,
    pub title: Option<String>,
    pub content: Option<String>,
    #[serde(default)]
    pub summary: PatchField<String>,
}

#[derive(Default, Deserialize)]
pub struct ModerateTopicRequest {
    pub is_pinned: Option<bool>,
    pub is_featured: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::{TopicListSort, TopicStatus};

    #[test]
    fn parses_topic_status() {
        assert_eq!("published".parse(), Ok(TopicStatus::Published));
        assert!("unknown".parse::<TopicStatus>().is_err());
    }

    #[test]
    fn defaults_topic_sort_to_latest() {
        assert_eq!(TopicListSort::default(), TopicListSort::Latest);
    }
}
