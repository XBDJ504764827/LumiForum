use redis::{aio::ConnectionManager, AsyncCommands};
use thiserror::Error;
use uuid::Uuid;

use crate::models::{
    AuthenticatedPrincipal, CommentListQuery, CommentNode, CreateCommentRequest, Paginated,
    PaginationMeta, UpdateCommentRequest, PERMISSION_COMMENT_CREATE, PERMISSION_COMMENT_DELETE_ANY,
    PERMISSION_COMMENT_DELETE_SELF, PERMISSION_COMMENT_REPLY, PERMISSION_COMMENT_RESTORE,
    PERMISSION_COMMENT_UPDATE_ANY, PERMISSION_COMMENT_UPDATE_SELF,
};
use crate::repositories::{
    repository_comment_to_node, CommentRepository, NewComment, TopicRepository,
};

const DEFAULT_PAGE_SIZE: u32 = 20;
const MAX_PAGE_SIZE: u32 = 50;
const MAX_PAGE: u32 = 1_000_000;
const RATE_LIMIT_WINDOW_SECS: u64 = 60;
const RATE_LIMIT_MAX: u64 = 10;

#[derive(Clone)]
pub struct CommentService {
    comments: CommentRepository,
    topics: TopicRepository,
    redis: ConnectionManager,
}

#[derive(Debug, Error)]
pub enum CommentError {
    #[error("invalid comment input: {0}")]
    Validation(&'static str),
    #[error("comment not found")]
    NotFound,
    #[error("topic not found")]
    TopicNotFound,
    #[error("permission denied")]
    Forbidden,
    #[error("rate limit exceeded")]
    RateLimited,
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

impl CommentService {
    pub fn new(
        comments: CommentRepository,
        topics: TopicRepository,
        redis: ConnectionManager,
    ) -> Self {
        Self {
            comments,
            topics,
            redis,
        }
    }

    pub async fn list_for_topic(
        &self,
        topic_id: Uuid,
        query: CommentListQuery,
    ) -> Result<Paginated<CommentNode>, CommentError> {
        self.ensure_topic_published(topic_id).await?;
        let page = query.page.unwrap_or(1);
        let page_size = query.page_size.unwrap_or(DEFAULT_PAGE_SIZE);
        if page == 0 || page > MAX_PAGE {
            return Err(CommentError::Validation("page is out of range"));
        }
        if page_size == 0 || page_size > MAX_PAGE_SIZE {
            return Err(CommentError::Validation(
                "page size must be between 1 and 50",
            ));
        }
        let offset = i64::from(page - 1) * i64::from(page_size);
        let (items, total) = self
            .comments
            .list_tree_page(topic_id, i64::from(page_size), offset)
            .await
            .map_err(internal)?;
        let total =
            u64::try_from(total).map_err(|_| internal(anyhow::anyhow!("negative count")))?;
        Ok(Paginated {
            items,
            pagination: PaginationMeta::new(page, page_size, total),
        })
    }

    pub async fn create_root(
        &self,
        principal: &AuthenticatedPrincipal,
        topic_id: Uuid,
        request: CreateCommentRequest,
    ) -> Result<CommentNode, CommentError> {
        require(principal, PERMISSION_COMMENT_CREATE)?;
        self.ensure_topic_published(topic_id).await?;
        self.enforce_rate_limit(principal.user_id).await?;
        let content = normalize_content(request.content)?;
        let comment = self
            .comments
            .create(NewComment {
                topic_id,
                author_id: principal.user_id,
                parent_id: None,
                content: &content,
            })
            .await
            .map_err(map_write_error)?;
        Ok(repository_comment_to_node(comment, Vec::new()))
    }

    pub async fn reply(
        &self,
        principal: &AuthenticatedPrincipal,
        parent_id: Uuid,
        request: CreateCommentRequest,
    ) -> Result<CommentNode, CommentError> {
        require(principal, PERMISSION_COMMENT_REPLY)?;
        self.enforce_rate_limit(principal.user_id).await?;
        let parent = self
            .comments
            .find_by_id(parent_id)
            .await
            .map_err(internal)?
            .ok_or(CommentError::NotFound)?;
        if parent.status != "published" || parent.parent_id.is_some() {
            return Err(CommentError::Validation(
                "can only reply to a published root comment",
            ));
        }
        self.ensure_topic_published(parent.topic_id).await?;
        let content = normalize_content(request.content)?;
        let comment = self
            .comments
            .create(NewComment {
                topic_id: parent.topic_id,
                author_id: principal.user_id,
                parent_id: Some(parent.id),
                content: &content,
            })
            .await
            .map_err(map_write_error)?;
        Ok(repository_comment_to_node(comment, Vec::new()))
    }

    pub async fn update(
        &self,
        principal: &AuthenticatedPrincipal,
        comment_id: Uuid,
        request: UpdateCommentRequest,
    ) -> Result<CommentNode, CommentError> {
        let existing = self
            .comments
            .find_by_id(comment_id)
            .await
            .map_err(internal)?
            .ok_or(CommentError::NotFound)?;
        if existing.status != "published" {
            return Err(CommentError::NotFound);
        }
        require_owner_or_any(
            principal,
            existing.author_id,
            PERMISSION_COMMENT_UPDATE_SELF,
            PERMISSION_COMMENT_UPDATE_ANY,
        )?;
        let content = normalize_content(request.content)?;
        let comment = self
            .comments
            .update_content(comment_id, &content)
            .await
            .map_err(internal)?
            .ok_or(CommentError::NotFound)?;
        Ok(repository_comment_to_node(comment, Vec::new()))
    }

    pub async fn delete(
        &self,
        principal: &AuthenticatedPrincipal,
        comment_id: Uuid,
    ) -> Result<(), CommentError> {
        let existing = self
            .comments
            .find_by_id(comment_id)
            .await
            .map_err(internal)?
            .ok_or(CommentError::NotFound)?;
        if existing.status != "published" {
            return Err(CommentError::NotFound);
        }
        require_owner_or_any(
            principal,
            existing.author_id,
            PERMISSION_COMMENT_DELETE_SELF,
            PERMISSION_COMMENT_DELETE_ANY,
        )?;
        if self
            .comments
            .soft_delete(comment_id)
            .await
            .map_err(internal)?
        {
            Ok(())
        } else {
            Err(CommentError::NotFound)
        }
    }

    pub async fn restore(
        &self,
        principal: &AuthenticatedPrincipal,
        comment_id: Uuid,
    ) -> Result<CommentNode, CommentError> {
        require(principal, PERMISSION_COMMENT_RESTORE)?;
        let comment = self
            .comments
            .restore(comment_id)
            .await
            .map_err(|error| match error {
                sqlx::Error::RowNotFound => CommentError::Validation(
                    "comment cannot be restored because its parent or topic is unavailable",
                ),
                other => internal(other),
            })?
            .ok_or(CommentError::NotFound)?;
        Ok(repository_comment_to_node(comment, Vec::new()))
    }

    async fn ensure_topic_published(&self, topic_id: Uuid) -> Result<(), CommentError> {
        let topic = self
            .topics
            .find_by_id(topic_id)
            .await
            .map_err(internal)?
            .ok_or(CommentError::TopicNotFound)?;
        if topic.status != "published" || !topic.category_is_visible {
            return Err(CommentError::TopicNotFound);
        }
        Ok(())
    }

    async fn enforce_rate_limit(&self, user_id: Uuid) -> Result<(), CommentError> {
        let key = format!("rate:comment:{user_id}");
        let mut redis = self.redis.clone();
        match redis.incr::<_, u64, u64>(&key, 1_u64).await {
            Ok(count) => {
                if count == 1 {
                    let _: Result<(), _> = redis.expire(&key, RATE_LIMIT_WINDOW_SECS as i64).await;
                }
                if count > RATE_LIMIT_MAX {
                    Err(CommentError::RateLimited)
                } else {
                    Ok(())
                }
            }
            Err(error) => {
                tracing::warn!(%error, %user_id, "comment rate limit unavailable; allowing request");
                Ok(())
            }
        }
    }
}

fn normalize_content(value: String) -> Result<String, CommentError> {
    let value = value.trim().to_owned();
    if value.is_empty() || value.chars().count() > 20_000 {
        return Err(CommentError::Validation(
            "content must contain between 1 and 20000 characters",
        ));
    }
    Ok(value)
}

fn require(
    principal: &AuthenticatedPrincipal,
    permission: &'static str,
) -> Result<(), CommentError> {
    if principal.has_permission(permission) {
        Ok(())
    } else {
        Err(CommentError::Forbidden)
    }
}

fn require_owner_or_any(
    principal: &AuthenticatedPrincipal,
    author_id: Uuid,
    self_permission: &'static str,
    any_permission: &'static str,
) -> Result<(), CommentError> {
    let allowed = if principal.user_id == author_id {
        principal.has_permission(self_permission) || principal.has_permission(any_permission)
    } else {
        principal.has_permission(any_permission)
    };
    if allowed {
        Ok(())
    } else {
        Err(CommentError::Forbidden)
    }
}

fn map_write_error(error: sqlx::Error) -> CommentError {
    if let Some(db) = error.as_database_error() {
        if db.message().contains("nesting deeper") {
            return CommentError::Validation("comment nesting deeper than 2 levels is not allowed");
        }
        if db.message().contains("same topic") {
            return CommentError::Validation("comment parent must belong to the same topic");
        }
        if db.message().contains("deleted comment") {
            return CommentError::Validation("cannot reply to a deleted comment");
        }
    }
    internal(error)
}

fn internal(error: impl Into<anyhow::Error>) -> CommentError {
    CommentError::Internal(error.into())
}

#[cfg(test)]
mod tests {
    use super::normalize_content;

    #[test]
    fn rejects_empty_content() {
        assert!(normalize_content("   ".into()).is_err());
    }
}
