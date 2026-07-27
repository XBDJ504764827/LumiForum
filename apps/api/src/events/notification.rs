use uuid::Uuid;

/// Domain events that may produce in-app notifications.
/// Keep payloads small and explicit so emitters stay decoupled from storage.
#[derive(Clone, Debug)]
pub enum NotificationEvent {
    TopicLiked(TopicLikedEvent),
    CommentLiked(CommentLikedEvent),
    CommentCreated(CommentCreatedEvent),
    CommentReplied(CommentRepliedEvent),
    TopicFavorited(TopicFavoritedEvent),
    UserFollowed(UserFollowedEvent),
}

#[derive(Clone, Debug)]
pub struct TopicLikedEvent {
    pub actor_id: Uuid,
    pub recipient_id: Uuid,
    pub topic_id: Uuid,
    pub topic_slug: String,
    pub topic_title: String,
}

#[derive(Clone, Debug)]
pub struct CommentLikedEvent {
    pub actor_id: Uuid,
    pub recipient_id: Uuid,
    pub comment_id: Uuid,
    pub topic_id: Uuid,
    pub topic_slug: String,
}

#[derive(Clone, Debug)]
pub struct CommentCreatedEvent {
    pub actor_id: Uuid,
    pub recipient_id: Uuid,
    pub comment_id: Uuid,
    pub topic_id: Uuid,
    pub topic_slug: String,
    pub topic_title: String,
}

#[derive(Clone, Debug)]
pub struct CommentRepliedEvent {
    pub actor_id: Uuid,
    pub recipient_id: Uuid,
    pub comment_id: Uuid,
    pub parent_comment_id: Uuid,
    pub topic_id: Uuid,
    pub topic_slug: String,
}

#[derive(Clone, Debug)]
pub struct TopicFavoritedEvent {
    pub actor_id: Uuid,
    pub recipient_id: Uuid,
    pub topic_id: Uuid,
    pub topic_slug: String,
    pub topic_title: String,
}

#[derive(Clone, Debug)]
pub struct UserFollowedEvent {
    pub actor_id: Uuid,
    pub recipient_id: Uuid,
}
