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
    // Phase 13: moderation notifications
    ReportSubmitted,
    ReportProcessed,
    ContentHidden,
    ContentDeleted,
    TopicLocked,
    UserWarned,
    UserMuted,
    UserBanned,
    SanctionExpiring,
    SanctionRevoked,
    AppealSubmitted,
    AppealApproved,
    AppealRejected,
    ModerationInbox,
    // Phase 14: poll notifications
    PollVoted,
    PollEnded,
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
            Self::ReportSubmitted => "report_submitted",
            Self::ReportProcessed => "report_processed",
            Self::ContentHidden => "content_hidden",
            Self::ContentDeleted => "content_deleted",
            Self::TopicLocked => "topic_locked",
            Self::UserWarned => "user_warned",
            Self::UserMuted => "user_muted",
            Self::UserBanned => "user_banned",
            Self::SanctionExpiring => "sanction_expiring",
            Self::SanctionRevoked => "sanction_revoked",
            Self::AppealSubmitted => "appeal_submitted",
            Self::AppealApproved => "appeal_approved",
            Self::AppealRejected => "appeal_rejected",
            Self::ModerationInbox => "moderation_inbox",
            Self::PollVoted => "poll_voted",
            Self::PollEnded => "poll_ended",
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
            "report_submitted" => Ok(Self::ReportSubmitted),
            "report_processed" => Ok(Self::ReportProcessed),
            "content_hidden" => Ok(Self::ContentHidden),
            "content_deleted" => Ok(Self::ContentDeleted),
            "topic_locked" => Ok(Self::TopicLocked),
            "user_warned" => Ok(Self::UserWarned),
            "user_muted" => Ok(Self::UserMuted),
            "user_banned" => Ok(Self::UserBanned),
            "sanction_expiring" => Ok(Self::SanctionExpiring),
            "sanction_revoked" => Ok(Self::SanctionRevoked),
            "appeal_submitted" => Ok(Self::AppealSubmitted),
            "appeal_approved" => Ok(Self::AppealApproved),
            "appeal_rejected" => Ok(Self::AppealRejected),
            "moderation_inbox" => Ok(Self::ModerationInbox),
            "poll_voted" => Ok(Self::PollVoted),
            "poll_ended" => Ok(Self::PollEnded),
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
