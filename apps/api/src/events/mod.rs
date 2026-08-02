//! Domain event surface for cross-feature side effects (notifications, future search/index, etc.).
//! Business services emit events; handlers stay free of inbox SQL.

mod notification;

pub use notification::{
    CommentCreatedEvent, CommentLikedEvent, CommentRepliedEvent, NotificationEvent, PollEndedEvent,
    PollVotedEvent, TopicFavoritedEvent, TopicLikedEvent, UserFollowedEvent,
};
