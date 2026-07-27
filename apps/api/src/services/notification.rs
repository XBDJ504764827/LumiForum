use redis::{aio::ConnectionManager, AsyncCommands};
use serde_json::json;
use thiserror::Error;
use uuid::Uuid;

use crate::events::{
    CommentCreatedEvent, CommentLikedEvent, CommentRepliedEvent, NotificationEvent,
    TopicFavoritedEvent, TopicLikedEvent, UserFollowedEvent,
};
use crate::models::{
    AuthenticatedPrincipal, NotificationQuery, NotificationResponse, NotificationTargetType,
    NotificationType, Paginated, PaginationMeta, UnreadCountResponse,
    PERMISSION_NOTIFICATION_READ_SELF, PERMISSION_NOTIFICATION_UPDATE_SELF,
};
use crate::repositories::{NewNotification, NotificationListFilter, NotificationRepository};

const DEFAULT_PAGE_SIZE: u32 = 20;
const MAX_PAGE_SIZE: u32 = 50;
const MAX_PAGE: u32 = 1_000_000;
const UNREAD_CACHE_TTL_SECS: u64 = 300;

#[derive(Clone)]
pub struct NotificationService {
    notifications: NotificationRepository,
    redis: ConnectionManager,
}

#[derive(Debug, Error)]
pub enum NotificationError {
    #[error("invalid notification input: {0}")]
    Validation(&'static str),
    #[error("notification not found")]
    NotFound,
    #[error("permission denied")]
    Forbidden,
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

impl NotificationService {
    pub fn new(notifications: NotificationRepository, redis: ConnectionManager) -> Self {
        Self {
            notifications,
            redis,
        }
    }

    pub async fn list(
        &self,
        principal: &AuthenticatedPrincipal,
        query: NotificationQuery,
    ) -> Result<Paginated<NotificationResponse>, NotificationError> {
        require(principal, PERMISSION_NOTIFICATION_READ_SELF)?;
        let page = query.page.unwrap_or(1);
        let page_size = query.page_size.unwrap_or(DEFAULT_PAGE_SIZE);
        if page == 0 || page > MAX_PAGE {
            return Err(NotificationError::Validation("page is out of range"));
        }
        if page_size == 0 || page_size > MAX_PAGE_SIZE {
            return Err(NotificationError::Validation(
                "page size must be between 1 and 50",
            ));
        }
        let type_filter = match query.notification_type.as_deref() {
            None => None,
            Some(value) => {
                let parsed = value
                    .parse::<NotificationType>()
                    .map_err(|_| NotificationError::Validation("unknown notification type"))?;
                Some(parsed.as_str().to_owned())
            }
        };
        let offset = i64::from(page - 1) * i64::from(page_size);
        let (items, total) = self
            .notifications
            .list(NotificationListFilter {
                user_id: principal.user_id,
                is_read: query.is_read,
                notification_type: type_filter.as_deref(),
                limit: i64::from(page_size),
                offset,
            })
            .await
            .map_err(internal)?;
        let total = u64::try_from(total)
            .map_err(|_| internal(anyhow::anyhow!("negative notification count")))?;
        Ok(Paginated {
            items,
            pagination: PaginationMeta::new(page, page_size, total),
        })
    }

    pub async fn unread_count(
        &self,
        principal: &AuthenticatedPrincipal,
    ) -> Result<UnreadCountResponse, NotificationError> {
        require(principal, PERMISSION_NOTIFICATION_READ_SELF)?;
        if let Some(count) = self.cached_unread(principal.user_id).await {
            return Ok(UnreadCountResponse { count });
        }
        let count = self
            .notifications
            .count_unread(principal.user_id)
            .await
            .map_err(internal)?;
        self.set_unread_cache(principal.user_id, count).await;
        Ok(UnreadCountResponse { count })
    }

    pub async fn mark_read(
        &self,
        principal: &AuthenticatedPrincipal,
        notification_id: Uuid,
    ) -> Result<(), NotificationError> {
        require(principal, PERMISSION_NOTIFICATION_UPDATE_SELF)?;
        if !self
            .notifications
            .belongs_to_user(principal.user_id, notification_id)
            .await
            .map_err(internal)?
        {
            return Err(NotificationError::NotFound);
        }
        let changed = self
            .notifications
            .mark_read(principal.user_id, notification_id)
            .await
            .map_err(internal)?;
        if changed {
            self.invalidate_unread_cache(principal.user_id).await;
        }
        Ok(())
    }

    pub async fn mark_all_read(
        &self,
        principal: &AuthenticatedPrincipal,
    ) -> Result<u64, NotificationError> {
        require(principal, PERMISSION_NOTIFICATION_UPDATE_SELF)?;
        let updated = self
            .notifications
            .mark_all_read(principal.user_id)
            .await
            .map_err(internal)?;
        self.set_unread_cache(principal.user_id, 0).await;
        Ok(updated)
    }

    /// Ingest a domain event into the inbox. Failures are logged by callers; never block UX hard.
    pub async fn handle_event(&self, event: NotificationEvent) -> Result<(), NotificationError> {
        match event {
            NotificationEvent::TopicLiked(event) => self.on_topic_liked(event).await,
            NotificationEvent::CommentLiked(event) => self.on_comment_liked(event).await,
            NotificationEvent::CommentCreated(event) => self.on_comment_created(event).await,
            NotificationEvent::CommentReplied(event) => self.on_comment_replied(event).await,
            NotificationEvent::TopicFavorited(event) => self.on_topic_favorited(event).await,
            NotificationEvent::UserFollowed(event) => self.on_user_followed(event).await,
        }
    }

    async fn on_topic_liked(&self, event: TopicLikedEvent) -> Result<(), NotificationError> {
        if event.actor_id == event.recipient_id {
            return Ok(());
        }
        self.create_inbox(NewNotification {
            user_id: event.recipient_id,
            actor_id: Some(event.actor_id),
            notification_type: NotificationType::PostLiked,
            title: "帖子被点赞",
            content: &format!("有人赞了你的帖子《{}》", event.topic_title),
            target_type: Some(NotificationTargetType::Topic),
            target_id: Some(event.topic_id),
            metadata: json!({
                "topic_id": event.topic_id,
                "topic_slug": event.topic_slug,
                "topic_title": event.topic_title,
                "href": format!("/topics/{}", event.topic_slug),
            }),
        })
        .await
    }

    async fn on_comment_liked(&self, event: CommentLikedEvent) -> Result<(), NotificationError> {
        if event.actor_id == event.recipient_id {
            return Ok(());
        }
        self.create_inbox(NewNotification {
            user_id: event.recipient_id,
            actor_id: Some(event.actor_id),
            notification_type: NotificationType::CommentLiked,
            title: "评论被点赞",
            content: "有人赞了你的评论",
            target_type: Some(NotificationTargetType::Comment),
            target_id: Some(event.comment_id),
            metadata: json!({
                "comment_id": event.comment_id,
                "topic_id": event.topic_id,
                "topic_slug": event.topic_slug,
                "href": format!("/topics/{}#comment-{}", event.topic_slug, event.comment_id),
            }),
        })
        .await
    }

    async fn on_comment_created(
        &self,
        event: CommentCreatedEvent,
    ) -> Result<(), NotificationError> {
        if event.actor_id == event.recipient_id {
            return Ok(());
        }
        self.create_inbox(NewNotification {
            user_id: event.recipient_id,
            actor_id: Some(event.actor_id),
            notification_type: NotificationType::CommentCreated,
            title: "帖子有新评论",
            content: &format!("有人评论了你的帖子《{}》", event.topic_title),
            target_type: Some(NotificationTargetType::Comment),
            target_id: Some(event.comment_id),
            metadata: json!({
                "comment_id": event.comment_id,
                "topic_id": event.topic_id,
                "topic_slug": event.topic_slug,
                "topic_title": event.topic_title,
                "href": format!("/topics/{}#comment-{}", event.topic_slug, event.comment_id),
            }),
        })
        .await
    }

    async fn on_comment_replied(
        &self,
        event: CommentRepliedEvent,
    ) -> Result<(), NotificationError> {
        if event.actor_id == event.recipient_id {
            return Ok(());
        }
        self.create_inbox(NewNotification {
            user_id: event.recipient_id,
            actor_id: Some(event.actor_id),
            notification_type: NotificationType::CommentReplied,
            title: "评论有新回复",
            content: "有人回复了你的评论",
            target_type: Some(NotificationTargetType::Comment),
            target_id: Some(event.comment_id),
            metadata: json!({
                "comment_id": event.comment_id,
                "parent_comment_id": event.parent_comment_id,
                "topic_id": event.topic_id,
                "topic_slug": event.topic_slug,
                "href": format!("/topics/{}#comment-{}", event.topic_slug, event.comment_id),
            }),
        })
        .await
    }

    async fn on_topic_favorited(
        &self,
        event: TopicFavoritedEvent,
    ) -> Result<(), NotificationError> {
        if event.actor_id == event.recipient_id {
            return Ok(());
        }
        self.create_inbox(NewNotification {
            user_id: event.recipient_id,
            actor_id: Some(event.actor_id),
            notification_type: NotificationType::TopicFavorited,
            title: "帖子被收藏",
            content: &format!("有人收藏了你的帖子《{}》", event.topic_title),
            target_type: Some(NotificationTargetType::Topic),
            target_id: Some(event.topic_id),
            metadata: json!({
                "topic_id": event.topic_id,
                "topic_slug": event.topic_slug,
                "topic_title": event.topic_title,
                "href": format!("/topics/{}", event.topic_slug),
            }),
        })
        .await
    }

    async fn on_user_followed(&self, event: UserFollowedEvent) -> Result<(), NotificationError> {
        if event.actor_id == event.recipient_id {
            return Ok(());
        }
        self.create_inbox(NewNotification {
            user_id: event.recipient_id,
            actor_id: Some(event.actor_id),
            notification_type: NotificationType::UserFollowed,
            title: "新增粉丝",
            content: "有人关注了你",
            target_type: Some(NotificationTargetType::User),
            target_id: Some(event.actor_id),
            metadata: json!({
                "user_id": event.actor_id,
                "href": format!("/users/{}/followers", event.recipient_id),
            }),
        })
        .await
    }

    async fn create_inbox(&self, input: NewNotification<'_>) -> Result<(), NotificationError> {
        let user_id = input.user_id;
        self.notifications.create(input).await.map_err(internal)?;
        self.invalidate_unread_cache(user_id).await;
        Ok(())
    }

    fn unread_key(user_id: Uuid) -> String {
        format!("notifications:unread:{user_id}")
    }

    async fn cached_unread(&self, user_id: Uuid) -> Option<i64> {
        let mut redis = self.redis.clone();
        match redis.get::<_, Option<i64>>(Self::unread_key(user_id)).await {
            Ok(value) => value,
            Err(error) => {
                tracing::warn!(%error, %user_id, "notification unread cache read failed");
                None
            }
        }
    }

    async fn set_unread_cache(&self, user_id: Uuid, count: i64) {
        let mut redis = self.redis.clone();
        if let Err(error) = redis
            .set_ex::<_, _, ()>(Self::unread_key(user_id), count, UNREAD_CACHE_TTL_SECS)
            .await
        {
            tracing::warn!(%error, %user_id, "notification unread cache write failed");
        }
    }

    async fn invalidate_unread_cache(&self, user_id: Uuid) {
        let mut redis = self.redis.clone();
        if let Err(error) = redis.del::<_, ()>(Self::unread_key(user_id)).await {
            tracing::warn!(%error, %user_id, "notification unread cache invalidate failed");
        }
    }
}

fn require(
    principal: &AuthenticatedPrincipal,
    permission: &'static str,
) -> Result<(), NotificationError> {
    if principal.has_permission(permission) {
        Ok(())
    } else {
        Err(NotificationError::Forbidden)
    }
}

fn internal(error: impl Into<anyhow::Error>) -> NotificationError {
    NotificationError::Internal(error.into())
}

#[cfg(test)]
mod tests {
    use super::require;
    use crate::models::{AuthenticatedPrincipal, PERMISSION_NOTIFICATION_READ_SELF, ROLE_USER};
    use uuid::Uuid;

    #[test]
    fn permission_gate_works() {
        let denied = AuthenticatedPrincipal::new(
            Uuid::new_v4(),
            ROLE_USER.into(),
            0,
            Uuid::new_v4(),
            Vec::<String>::new(),
        );
        assert!(require(&denied, PERMISSION_NOTIFICATION_READ_SELF).is_err());
        let allowed = AuthenticatedPrincipal::new(
            Uuid::new_v4(),
            ROLE_USER.into(),
            0,
            Uuid::new_v4(),
            [PERMISSION_NOTIFICATION_READ_SELF.into()],
        );
        assert!(require(&allowed, PERMISSION_NOTIFICATION_READ_SELF).is_ok());
    }
}
