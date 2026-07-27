use std::str::FromStr;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use uuid::Uuid;

use super::RoleSummary;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationType {
    PostLiked,
    CommentLiked,
    CommentCreated,
    CommentReplied,
    TopicFavorited,
    UserFollowed,
    Mentioned,
    SystemMessage,
}

impl NotificationType {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PostLiked => "post_liked",
            Self::CommentLiked => "comment_liked",
            Self::CommentCreated => "comment_created",
            Self::CommentReplied => "comment_replied",
            Self::TopicFavorited => "topic_favorited",
            Self::UserFollowed => "user_followed",
            Self::Mentioned => "mentioned",
            Self::SystemMessage => "system_message",
        }
    }
}

impl FromStr for NotificationType {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "post_liked" => Ok(Self::PostLiked),
            "comment_liked" => Ok(Self::CommentLiked),
            "comment_created" => Ok(Self::CommentCreated),
            "comment_replied" => Ok(Self::CommentReplied),
            "topic_favorited" => Ok(Self::TopicFavorited),
            "user_followed" => Ok(Self::UserFollowed),
            "mentioned" => Ok(Self::Mentioned),
            "system_message" => Ok(Self::SystemMessage),
            _ => Err("unknown notification type"),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationTargetType {
    Topic,
    Comment,
    User,
    System,
}

impl NotificationTargetType {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Topic => "topic",
            Self::Comment => "comment",
            Self::User => "user",
            Self::System => "system",
        }
    }
}

impl FromStr for NotificationTargetType {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "topic" => Ok(Self::Topic),
            "comment" => Ok(Self::Comment),
            "user" => Ok(Self::User),
            "system" => Ok(Self::System),
            _ => Err("unknown notification target type"),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct NotificationActor {
    pub id: Uuid,
    pub username: String,
    pub nickname: Option<String>,
    pub avatar: Option<String>,
    pub role: RoleSummary,
}

#[derive(Clone, Debug, Serialize)]
pub struct NotificationResponse {
    pub id: Uuid,
    #[serde(rename = "type")]
    pub notification_type: NotificationType,
    pub title: String,
    pub content: String,
    pub target_type: Option<NotificationTargetType>,
    pub target_id: Option<Uuid>,
    pub metadata: JsonValue,
    pub is_read: bool,
    pub actor: Option<NotificationActor>,
    pub created_at: DateTime<Utc>,
    /// Reserved for future WebSocket/SSE fan-out routing.
    pub stream_hint: &'static str,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct UnreadCountResponse {
    pub count: i64,
}

#[derive(Default, Deserialize)]
pub struct NotificationQuery {
    pub page: Option<u32>,
    pub page_size: Option<u32>,
    pub is_read: Option<bool>,
    #[serde(rename = "type")]
    pub notification_type: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::NotificationType;

    #[test]
    fn parses_notification_types() {
        assert_eq!(
            "post_liked".parse::<NotificationType>().unwrap(),
            NotificationType::PostLiked
        );
        assert!("unknown".parse::<NotificationType>().is_err());
    }
}
