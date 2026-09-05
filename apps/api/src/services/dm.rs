use thiserror::Error;
use uuid::Uuid;

use crate::models::{
    AuthenticatedPrincipal, ConversationDetail, ConversationSummary, ConversationUser,
    MessageListQuery, MessagePage, MessageResponse, Paginated, PaginationMeta, SendMessageRequest,
    StartConversationRequest, UserStatus, PERMISSION_DM_READ_SELF, PERMISSION_DM_WRITE,
};
use crate::repositories::{ConversationRepository, ParticipantContext, UserRepository};

use super::NotificationService;

const DEFAULT_PAGE_SIZE: u32 = 30;
const MAX_PAGE_SIZE: u32 = 50;
const MAX_PAGE: u32 = 1_000_000;
const MAX_MESSAGE_LENGTH: usize = 2000;
const NOTIFICATION_PREVIEW_CHARS: usize = 80;

#[derive(Clone)]
pub struct DmService {
    conversations: ConversationRepository,
    users: UserRepository,
    notifications: NotificationService,
}

#[derive(Debug, Error)]
pub enum DmError {
    #[error("invalid dm input: {0}")]
    Validation(&'static str),
    #[error("conversation not found")]
    NotFound,
    #[error("recipient not found or inactive")]
    RecipientUnavailable,
    #[error("cannot message yourself")]
    SelfMessage,
    #[error("permission denied")]
    Forbidden,
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

impl DmService {
    pub fn new(
        conversations: ConversationRepository,
        users: UserRepository,
        notifications: NotificationService,
    ) -> Self {
        Self {
            conversations,
            users,
            notifications,
        }
    }

    /// Inbox: conversations of the acting user, newest activity first.
    pub async fn list_conversations(
        &self,
        principal: &AuthenticatedPrincipal,
        page: u32,
        page_size: u32,
    ) -> Result<Paginated<ConversationSummary>, DmError> {
        require(principal, PERMISSION_DM_READ_SELF)?;
        validate_page(page, page_size)?;
        let offset = i64::from(page - 1) * i64::from(page_size);
        let (items, total) = self
            .conversations
            .list_for_user(principal.user_id, i64::from(page_size), offset)
            .await
            .map_err(internal)?;
        let total = u64::try_from(total)
            .map_err(|_| internal(anyhow::anyhow!("negative conversation count")))?;
        Ok(Paginated {
            items,
            pagination: PaginationMeta::new(page, page_size, total),
        })
    }

    /// Open (or create) a conversation with the given username. Requires the
    /// recipient to be an active account other than the sender.
    pub async fn start_conversation(
        &self,
        principal: &AuthenticatedPrincipal,
        request: StartConversationRequest,
    ) -> Result<ConversationDetail, DmError> {
        require(principal, PERMISSION_DM_WRITE)?;
        let username = request.username.trim();
        if username.is_empty() || username.chars().count() > 32 {
            return Err(DmError::Validation("username is required"));
        }
        let recipient = self
            .users
            .find_by_username(username)
            .await
            .map_err(internal)?
            .ok_or(DmError::RecipientUnavailable)?;
        if recipient.status != UserStatus::Active.as_str() {
            return Err(DmError::RecipientUnavailable);
        }
        if recipient.id == principal.user_id {
            return Err(DmError::SelfMessage);
        }
        let conversation_id = self
            .conversations
            .find_or_create_pair(principal.user_id, recipient.id)
            .await
            .map_err(internal)?;
        Ok(ConversationDetail {
            id: conversation_id,
            other_user: ConversationUser {
                id: recipient.id,
                username: recipient.username,
                nickname: recipient.nickname,
                avatar: recipient.avatar,
                role: crate::models::RoleSummary {
                    code: recipient.role_code,
                    name: recipient.role_name,
                },
            },
            unread_count: 0,
            created_at: chrono::Utc::now(),
        })
    }

    /// Conversation metadata for a member (participant + unread state).
    /// Online status is intentionally NOT part of DM delivery: messages are
    /// durable and the recipient reads them whenever they return, so an
    /// offline participant never changes these responses.
    pub async fn get_conversation(
        &self,
        principal: &AuthenticatedPrincipal,
        conversation_id: Uuid,
    ) -> Result<ConversationDetail, DmError> {
        require(principal, PERMISSION_DM_READ_SELF)?;
        let context = self
            .conversations
            .find_participant(conversation_id, principal.user_id)
            .await
            .map_err(internal)?
            .ok_or(DmError::NotFound)?;
        let other = self
            .users
            .find_by_id(context.other_user_id)
            .await
            .map_err(internal)?
            .ok_or(DmError::NotFound)?;
        let unread = self
            .conversations
            .unread_for(conversation_id, principal.user_id)
            .await
            .unwrap_or(0);
        Ok(ConversationDetail {
            id: conversation_id,
            other_user: ConversationUser {
                id: other.id,
                username: other.username,
                nickname: other.nickname,
                avatar: other.avatar,
                role: crate::models::RoleSummary {
                    code: other.role_code,
                    name: other.role_name,
                },
            },
            unread_count: unread,
            created_at: chrono::Utc::now(),
        })
    }

    /// Messages of one conversation (newest first), verifying membership.
    /// Reads stay available even when the partner was deactivated so history
    /// is never hidden from the viewer.
    pub async fn list_messages(
        &self,
        principal: &AuthenticatedPrincipal,
        conversation_id: Uuid,
        query: MessageListQuery,
    ) -> Result<MessagePage, DmError> {
        require(principal, PERMISSION_DM_READ_SELF)?;
        self.ensure_participant(conversation_id, principal.user_id)
            .await?;
        let page = query.page.unwrap_or(1);
        let page_size = query.page_size.unwrap_or(DEFAULT_PAGE_SIZE);
        validate_page(page, page_size)?;
        let offset = i64::from(page - 1) * i64::from(page_size);
        let (items, total) = self
            .conversations
            .messages(conversation_id, i64::from(page_size), offset)
            .await
            .map_err(internal)?;
        let total = u64::try_from(total)
            .map_err(|_| internal(anyhow::anyhow!("negative message count")))?;
        Ok(Paginated {
            items,
            pagination: PaginationMeta::new(page, page_size, total),
        })
    }

    /// Send a message into a conversation the sender belongs to.
    pub async fn send_message(
        &self,
        principal: &AuthenticatedPrincipal,
        conversation_id: Uuid,
        request: SendMessageRequest,
    ) -> Result<MessageResponse, DmError> {
        require(principal, PERMISSION_DM_WRITE)?;
        let content = request.content.trim();
        if content.is_empty() {
            return Err(DmError::Validation("message content is empty"));
        }
        if content.chars().count() > MAX_MESSAGE_LENGTH {
            return Err(DmError::Validation("message content is too long"));
        }
        // Sends require both participants to be active.
        let context = self
            .ensure_active_participant(conversation_id, principal.user_id)
            .await?;
        let message = self
            .conversations
            .insert_message(conversation_id, principal.user_id, content)
            .await
            .map_err(internal)?;
        self.notify_recipient(principal.user_id, context, &message)
            .await;
        Ok(message)
    }

    /// Mark every message of a conversation as read for the acting user.
    pub async fn mark_read(
        &self,
        principal: &AuthenticatedPrincipal,
        conversation_id: Uuid,
    ) -> Result<(), DmError> {
        require(principal, PERMISSION_DM_READ_SELF)?;
        self.ensure_participant(conversation_id, principal.user_id)
            .await?;
        self.conversations
            .mark_read(conversation_id, principal.user_id)
            .await
            .map_err(internal)?;
        Ok(())
    }

    /// Unread private-message total for the acting user.
    pub async fn unread_count(&self, principal: &AuthenticatedPrincipal) -> Result<i64, DmError> {
        require(principal, PERMISSION_DM_READ_SELF)?;
        Ok(self
            .conversations
            .total_unread(principal.user_id)
            .await
            .unwrap_or(0))
    }

    /// Soft-delete a message the sender wrote.
    pub async fn delete_message(
        &self,
        principal: &AuthenticatedPrincipal,
        message_id: Uuid,
    ) -> Result<(), DmError> {
        require(principal, PERMISSION_DM_WRITE)?;
        let deleted = self
            .conversations
            .soft_delete_message(message_id, principal.user_id)
            .await
            .map_err(internal)?;
        if !deleted {
            return Err(DmError::NotFound);
        }
        Ok(())
    }

    /// Membership check without a status gate (reads never hide history).
    async fn ensure_participant(
        &self,
        conversation_id: Uuid,
        viewer_id: Uuid,
    ) -> Result<(), DmError> {
        self.conversations
            .find_participant(conversation_id, viewer_id)
            .await
            .map_err(internal)?
            .ok_or(DmError::NotFound)?;
        Ok(())
    }

    /// Membership plus active-recipient check (mutating paths).
    async fn ensure_active_participant(
        &self,
        conversation_id: Uuid,
        viewer_id: Uuid,
    ) -> Result<ParticipantContext, DmError> {
        let context = self
            .conversations
            .find_participant(conversation_id, viewer_id)
            .await
            .map_err(internal)?
            .ok_or(DmError::NotFound)?;
        if context.other_status != "active" {
            return Err(DmError::RecipientUnavailable);
        }
        Ok(context)
    }

    /// Best-effort inbox notification for the recipient.
    async fn notify_recipient(
        &self,
        sender_id: Uuid,
        context: ParticipantContext,
        message: &MessageResponse,
    ) {
        let preview: String = message
            .content
            .chars()
            .take(NOTIFICATION_PREVIEW_CHARS)
            .collect();
        if let Err(error) = self
            .notifications
            .send_private_message_notification(
                context.other_user_id,
                sender_id,
                message.id,
                &preview,
            )
            .await
        {
            tracing::warn!(%error, "failed to deliver private message notification");
        }
    }
}

fn require(principal: &AuthenticatedPrincipal, permission: &str) -> Result<(), DmError> {
    if principal.has_permission(permission) {
        Ok(())
    } else {
        Err(DmError::Forbidden)
    }
}

fn validate_page(page: u32, page_size: u32) -> Result<(), DmError> {
    if page == 0 || page > MAX_PAGE {
        return Err(DmError::Validation("page is out of range"));
    }
    if page_size == 0 || page_size > MAX_PAGE_SIZE {
        return Err(DmError::Validation("page size must be between 1 and 50"));
    }
    Ok(())
}

fn internal(error: impl Into<anyhow::Error>) -> DmError {
    DmError::Internal(error.into())
}
