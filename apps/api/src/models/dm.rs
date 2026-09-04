use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{Paginated, RoleSummary};

/// A conversation list entry (inbox row): the other participant plus unread
/// state and the latest message preview.
#[derive(Clone, Debug, Serialize)]
pub struct ConversationSummary {
    pub id: Uuid,
    pub other_user: ConversationUser,
    pub unread_count: i64,
    pub last_message_at: DateTime<Utc>,
    pub last_message_preview: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ConversationUser {
    pub id: Uuid,
    pub username: String,
    pub nickname: Option<String>,
    pub avatar: Option<String>,
    pub role: RoleSummary,
}

#[derive(Clone, Debug, Serialize)]
pub struct ConversationDetail {
    pub id: Uuid,
    pub other_user: ConversationUser,
    pub unread_count: i64,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize)]
pub struct MessageResponse {
    pub id: Uuid,
    pub conversation_id: Uuid,
    pub sender_id: Uuid,
    /// Empty when the message was soft-deleted (rendered as a placeholder).
    pub content: String,
    pub is_deleted: bool,
    pub created_at: DateTime<Utc>,
    pub edited_at: Option<DateTime<Utc>>,
}

/// Request body for starting/opening a conversation with a user.
#[derive(Debug, Deserialize)]
pub struct StartConversationRequest {
    pub username: String,
}

#[derive(Debug, Deserialize)]
pub struct SendMessageRequest {
    pub content: String,
}

#[derive(Default, Deserialize)]
pub struct MessageListQuery {
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

/// Reused envelope: message pages look like every other paginated endpoint.
pub type MessagePage = Paginated<MessageResponse>;
