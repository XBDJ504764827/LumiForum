use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{RoleSummary, TopicSummary};

#[derive(Clone, Copy, Debug, Serialize)]
pub struct TopicLikeState {
    pub liked: bool,
    pub like_count: i64,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct CommentLikeState {
    pub liked: bool,
    pub like_count: i64,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct FavoriteState {
    pub favorited: bool,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct FollowState {
    pub following: bool,
    pub followers_count: i64,
    pub following_count: i64,
}

#[derive(Clone, Debug, Serialize)]
pub struct UserPublicSummary {
    pub id: Uuid,
    pub username: String,
    pub nickname: Option<String>,
    pub avatar: Option<String>,
    pub role: RoleSummary,
    pub followers_count: i64,
    pub following_count: i64,
    pub is_following: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize)]
pub struct FavoriteItem {
    pub favorited_at: DateTime<Utc>,
    pub topic: TopicSummary,
}

#[derive(Default, Deserialize)]
pub struct ReactionListQuery {
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}
