use redis::{aio::ConnectionManager, AsyncCommands};
use thiserror::Error;
use uuid::Uuid;

use crate::models::{
    AuthenticatedPrincipal, CommentLikeState, FavoriteItem, FavoriteState, FollowState, Paginated,
    PaginationMeta, ReactionListQuery, TopicLikeState, UserPublicSummary, PERMISSION_COMMENT_LIKE,
    PERMISSION_TOPIC_FAVORITE, PERMISSION_TOPIC_LIKE, PERMISSION_USER_FOLLOW,
};
use crate::repositories::ReactionRepository;

const DEFAULT_PAGE_SIZE: u32 = 20;
const MAX_PAGE_SIZE: u32 = 50;
const MAX_PAGE: u32 = 1_000_000;
const RATE_LIMIT_WINDOW_SECS: u64 = 60;
const RATE_LIMIT_MAX: u64 = 60;
const STATS_CACHE_TTL_SECS: u64 = 30;

#[derive(Clone)]
pub struct ReactionService {
    reactions: ReactionRepository,
    redis: ConnectionManager,
}

#[derive(Debug, Error)]
pub enum ReactionError {
    #[error("invalid reaction input: {0}")]
    Validation(&'static str),
    #[error("resource not found")]
    NotFound,
    #[error("permission denied")]
    Forbidden,
    #[error("rate limit exceeded")]
    RateLimited,
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

impl ReactionService {
    pub fn new(reactions: ReactionRepository, redis: ConnectionManager) -> Self {
        Self { reactions, redis }
    }

    pub async fn like_topic(
        &self,
        principal: &AuthenticatedPrincipal,
        topic_id: Uuid,
    ) -> Result<TopicLikeState, ReactionError> {
        require(principal, PERMISSION_TOPIC_LIKE)?;
        self.enforce_rate_limit(principal.user_id).await?;
        self.ensure_topic(topic_id).await?;
        let like_count = self
            .reactions
            .like_topic(principal.user_id, topic_id)
            .await
            .map_err(internal)?;
        self.cache_topic_likes(topic_id, like_count).await;
        Ok(TopicLikeState {
            liked: true,
            like_count,
        })
    }

    pub async fn unlike_topic(
        &self,
        principal: &AuthenticatedPrincipal,
        topic_id: Uuid,
    ) -> Result<TopicLikeState, ReactionError> {
        require(principal, PERMISSION_TOPIC_LIKE)?;
        self.enforce_rate_limit(principal.user_id).await?;
        self.ensure_topic(topic_id).await?;
        let like_count = self
            .reactions
            .unlike_topic(principal.user_id, topic_id)
            .await
            .map_err(internal)?;
        self.cache_topic_likes(topic_id, like_count).await;
        Ok(TopicLikeState {
            liked: false,
            like_count,
        })
    }

    pub async fn like_comment(
        &self,
        principal: &AuthenticatedPrincipal,
        comment_id: Uuid,
    ) -> Result<CommentLikeState, ReactionError> {
        require(principal, PERMISSION_COMMENT_LIKE)?;
        self.enforce_rate_limit(principal.user_id).await?;
        self.ensure_comment(comment_id).await?;
        let like_count = self
            .reactions
            .like_comment(principal.user_id, comment_id)
            .await
            .map_err(internal)?;
        self.cache_comment_likes(comment_id, like_count).await;
        Ok(CommentLikeState {
            liked: true,
            like_count,
        })
    }

    pub async fn unlike_comment(
        &self,
        principal: &AuthenticatedPrincipal,
        comment_id: Uuid,
    ) -> Result<CommentLikeState, ReactionError> {
        require(principal, PERMISSION_COMMENT_LIKE)?;
        self.enforce_rate_limit(principal.user_id).await?;
        self.ensure_comment(comment_id).await?;
        let like_count = self
            .reactions
            .unlike_comment(principal.user_id, comment_id)
            .await
            .map_err(internal)?;
        self.cache_comment_likes(comment_id, like_count).await;
        Ok(CommentLikeState {
            liked: false,
            like_count,
        })
    }

    pub async fn favorite_topic(
        &self,
        principal: &AuthenticatedPrincipal,
        topic_id: Uuid,
    ) -> Result<FavoriteState, ReactionError> {
        require(principal, PERMISSION_TOPIC_FAVORITE)?;
        self.enforce_rate_limit(principal.user_id).await?;
        self.ensure_topic(topic_id).await?;
        self.reactions
            .favorite_topic(principal.user_id, topic_id)
            .await
            .map_err(internal)?;
        Ok(FavoriteState { favorited: true })
    }

    pub async fn unfavorite_topic(
        &self,
        principal: &AuthenticatedPrincipal,
        topic_id: Uuid,
    ) -> Result<FavoriteState, ReactionError> {
        require(principal, PERMISSION_TOPIC_FAVORITE)?;
        self.enforce_rate_limit(principal.user_id).await?;
        self.ensure_topic(topic_id).await?;
        self.reactions
            .unfavorite_topic(principal.user_id, topic_id)
            .await
            .map_err(internal)?;
        Ok(FavoriteState { favorited: false })
    }

    pub async fn list_my_favorites(
        &self,
        principal: &AuthenticatedPrincipal,
        query: ReactionListQuery,
    ) -> Result<Paginated<FavoriteItem>, ReactionError> {
        require(principal, PERMISSION_TOPIC_FAVORITE)?;
        let (page, page_size) = paginate(query)?;
        let offset = i64::from(page - 1) * i64::from(page_size);
        let (items, total) = self
            .reactions
            .list_favorites(principal.user_id, i64::from(page_size), offset)
            .await
            .map_err(internal)?;
        let total =
            u64::try_from(total).map_err(|_| internal(anyhow::anyhow!("negative favorite count")))?;
        Ok(Paginated {
            items,
            pagination: PaginationMeta::new(page, page_size, total),
        })
    }

    pub async fn follow_user(
        &self,
        principal: &AuthenticatedPrincipal,
        user_id: Uuid,
    ) -> Result<FollowState, ReactionError> {
        require(principal, PERMISSION_USER_FOLLOW)?;
        self.enforce_rate_limit(principal.user_id).await?;
        if principal.user_id == user_id {
            return Err(ReactionError::Validation("cannot follow yourself"));
        }
        self.ensure_user(user_id).await?;
        let counters = self
            .reactions
            .follow_user(principal.user_id, user_id)
            .await
            .map_err(internal)?;
        self.cache_follow_stats(user_id, counters.followers_count, counters.following_count)
            .await;
        Ok(FollowState {
            following: true,
            followers_count: counters.followers_count,
            following_count: counters.following_count,
        })
    }

    pub async fn unfollow_user(
        &self,
        principal: &AuthenticatedPrincipal,
        user_id: Uuid,
    ) -> Result<FollowState, ReactionError> {
        require(principal, PERMISSION_USER_FOLLOW)?;
        self.enforce_rate_limit(principal.user_id).await?;
        if principal.user_id == user_id {
            return Err(ReactionError::Validation("cannot unfollow yourself"));
        }
        self.ensure_user(user_id).await?;
        let counters = self
            .reactions
            .unfollow_user(principal.user_id, user_id)
            .await
            .map_err(internal)?;
        self.cache_follow_stats(user_id, counters.followers_count, counters.following_count)
            .await;
        Ok(FollowState {
            following: false,
            followers_count: counters.followers_count,
            following_count: counters.following_count,
        })
    }

    pub async fn list_followers(
        &self,
        user_id: Uuid,
        viewer_id: Option<Uuid>,
        query: ReactionListQuery,
    ) -> Result<Paginated<UserPublicSummary>, ReactionError> {
        self.ensure_user(user_id).await?;
        let (page, page_size) = paginate(query)?;
        let offset = i64::from(page - 1) * i64::from(page_size);
        let (items, total) = self
            .reactions
            .list_followers(user_id, viewer_id, i64::from(page_size), offset)
            .await
            .map_err(internal)?;
        let total =
            u64::try_from(total).map_err(|_| internal(anyhow::anyhow!("negative follower count")))?;
        Ok(Paginated {
            items,
            pagination: PaginationMeta::new(page, page_size, total),
        })
    }

    pub async fn list_following(
        &self,
        user_id: Uuid,
        viewer_id: Option<Uuid>,
        query: ReactionListQuery,
    ) -> Result<Paginated<UserPublicSummary>, ReactionError> {
        self.ensure_user(user_id).await?;
        let (page, page_size) = paginate(query)?;
        let offset = i64::from(page - 1) * i64::from(page_size);
        let (items, total) = self
            .reactions
            .list_following(user_id, viewer_id, i64::from(page_size), offset)
            .await
            .map_err(internal)?;
        let total = u64::try_from(total)
            .map_err(|_| internal(anyhow::anyhow!("negative following count")))?;
        Ok(Paginated {
            items,
            pagination: PaginationMeta::new(page, page_size, total),
        })
    }

    pub async fn viewer_topic_flags(
        &self,
        topic_id: Uuid,
        author_id: Uuid,
        viewer_id: Option<Uuid>,
    ) -> Result<(bool, bool, bool), ReactionError> {
        let Some(viewer_id) = viewer_id else {
            return Ok((false, false, false));
        };
        let liked = self
            .reactions
            .has_topic_like(viewer_id, topic_id)
            .await
            .map_err(internal)?;
        let favorited = self
            .reactions
            .has_favorite(viewer_id, topic_id)
            .await
            .map_err(internal)?;
        let following = if viewer_id == author_id {
            false
        } else {
            self.reactions
                .has_follow(viewer_id, author_id)
                .await
                .map_err(internal)?
        };
        Ok((liked, favorited, following))
    }

    pub async fn mark_comment_likes(
        &self,
        nodes: &mut [crate::models::CommentNode],
        viewer_id: Option<Uuid>,
    ) -> Result<(), ReactionError> {
        let Some(viewer_id) = viewer_id else {
            return Ok(());
        };
        let mut ids = Vec::new();
        collect_comment_ids(nodes, &mut ids);
        if ids.is_empty() {
            return Ok(());
        }
        let liked = self
            .reactions
            .liked_comment_ids(viewer_id, &ids)
            .await
            .map_err(internal)?;
        let liked_set: std::collections::HashSet<Uuid> = liked.into_iter().collect();
        apply_comment_likes(nodes, &liked_set);
        Ok(())
    }

    async fn ensure_topic(&self, topic_id: Uuid) -> Result<(), ReactionError> {
        if self
            .reactions
            .topic_is_published(topic_id)
            .await
            .map_err(internal)?
        {
            Ok(())
        } else {
            Err(ReactionError::NotFound)
        }
    }

    async fn ensure_comment(&self, comment_id: Uuid) -> Result<(), ReactionError> {
        if self
            .reactions
            .comment_is_published(comment_id)
            .await
            .map_err(internal)?
        {
            Ok(())
        } else {
            Err(ReactionError::NotFound)
        }
    }

    async fn ensure_user(&self, user_id: Uuid) -> Result<(), ReactionError> {
        if self
            .reactions
            .user_is_active(user_id)
            .await
            .map_err(internal)?
        {
            Ok(())
        } else {
            Err(ReactionError::NotFound)
        }
    }

    async fn enforce_rate_limit(&self, user_id: Uuid) -> Result<(), ReactionError> {
        let key = format!("rate:reaction:{user_id}");
        let mut redis = self.redis.clone();
        match redis.incr::<_, u64, u64>(&key, 1_u64).await {
            Ok(count) => {
                if count == 1 {
                    let _: Result<(), _> = redis.expire(&key, RATE_LIMIT_WINDOW_SECS as i64).await;
                }
                if count > RATE_LIMIT_MAX {
                    Err(ReactionError::RateLimited)
                } else {
                    Ok(())
                }
            }
            Err(error) => {
                tracing::warn!(%error, %user_id, "reaction rate limit unavailable; allowing request");
                Ok(())
            }
        }
    }

    async fn cache_topic_likes(&self, topic_id: Uuid, like_count: i64) {
        let key = format!("stats:topic:{topic_id}:likes");
        let mut redis = self.redis.clone();
        if let Err(error) = redis
            .set_ex::<_, _, ()>(key, like_count, STATS_CACHE_TTL_SECS)
            .await
        {
            tracing::warn!(%error, %topic_id, "failed to cache topic like stats");
        }
    }

    async fn cache_comment_likes(&self, comment_id: Uuid, like_count: i64) {
        let key = format!("stats:comment:{comment_id}:likes");
        let mut redis = self.redis.clone();
        if let Err(error) = redis
            .set_ex::<_, _, ()>(key, like_count, STATS_CACHE_TTL_SECS)
            .await
        {
            tracing::warn!(%error, %comment_id, "failed to cache comment like stats");
        }
    }

    async fn cache_follow_stats(
        &self,
        user_id: Uuid,
        followers_count: i64,
        following_count: i64,
    ) {
        let mut redis = self.redis.clone();
        let followers_key = format!("stats:user:{user_id}:followers");
        let following_key = format!("stats:user:{user_id}:following");
        if let Err(error) = redis
            .set_ex::<_, _, ()>(followers_key, followers_count, STATS_CACHE_TTL_SECS)
            .await
        {
            tracing::warn!(%error, %user_id, "failed to cache followers stats");
        }
        if let Err(error) = redis
            .set_ex::<_, _, ()>(following_key, following_count, STATS_CACHE_TTL_SECS)
            .await
        {
            tracing::warn!(%error, %user_id, "failed to cache following stats");
        }
    }
}

fn paginate(query: ReactionListQuery) -> Result<(u32, u32), ReactionError> {
    let page = query.page.unwrap_or(1);
    let page_size = query.page_size.unwrap_or(DEFAULT_PAGE_SIZE);
    if page == 0 || page > MAX_PAGE {
        return Err(ReactionError::Validation("page is out of range"));
    }
    if page_size == 0 || page_size > MAX_PAGE_SIZE {
        return Err(ReactionError::Validation(
            "page size must be between 1 and 50",
        ));
    }
    Ok((page, page_size))
}

fn require(
    principal: &AuthenticatedPrincipal,
    permission: &'static str,
) -> Result<(), ReactionError> {
    if principal.has_permission(permission) {
        Ok(())
    } else {
        Err(ReactionError::Forbidden)
    }
}

fn collect_comment_ids(nodes: &[crate::models::CommentNode], out: &mut Vec<Uuid>) {
    for node in nodes {
        out.push(node.id);
        collect_comment_ids(&node.replies, out);
    }
}

fn apply_comment_likes(
    nodes: &mut [crate::models::CommentNode],
    liked: &std::collections::HashSet<Uuid>,
) {
    for node in nodes {
        node.liked_by_me = liked.contains(&node.id);
        apply_comment_likes(&mut node.replies, liked);
    }
}

fn internal(error: impl Into<anyhow::Error>) -> ReactionError {
    ReactionError::Internal(error.into())
}

#[cfg(test)]
mod tests {
    use super::{paginate, require};
    use crate::models::{
        AuthenticatedPrincipal, ReactionListQuery, PERMISSION_TOPIC_LIKE, ROLE_USER,
    };
    use uuid::Uuid;

    #[test]
    fn paginate_defaults_and_bounds() {
        let (page, size) = paginate(ReactionListQuery::default()).unwrap();
        assert_eq!((page, size), (1, 20));
        assert!(paginate(ReactionListQuery {
            page: Some(0),
            page_size: None
        })
        .is_err());
        assert!(paginate(ReactionListQuery {
            page: Some(1),
            page_size: Some(100)
        })
        .is_err());
    }

    #[test]
    fn permission_gate_rejects_missing_permission() {
        let principal = AuthenticatedPrincipal::new(
            Uuid::new_v4(),
            ROLE_USER.into(),
            0,
            Uuid::new_v4(),
            Vec::<String>::new(),
        );
        assert!(require(&principal, PERMISSION_TOPIC_LIKE).is_err());
        let allowed = AuthenticatedPrincipal::new(
            Uuid::new_v4(),
            ROLE_USER.into(),
            0,
            Uuid::new_v4(),
            [PERMISSION_TOPIC_LIKE.into()],
        );
        assert!(require(&allowed, PERMISSION_TOPIC_LIKE).is_ok());
    }
}
