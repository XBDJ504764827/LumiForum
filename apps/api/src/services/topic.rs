use thiserror::Error;
use uuid::Uuid;

use crate::models::{
    AuthenticatedPrincipal, CreateTopicRequest, ModerateTopicRequest, Paginated, PaginationMeta,
    PatchField, TopicDetail, TopicListQuery, TopicStatus, TopicSummary, UpdateTopicRequest,
    PERMISSION_CATEGORY_MANAGE, PERMISSION_TOPIC_CREATE, PERMISSION_TOPIC_DELETE_ANY,
    PERMISSION_TOPIC_DELETE_SELF, PERMISSION_TOPIC_FEATURE, PERMISSION_TOPIC_PIN,
    PERMISSION_TOPIC_UPDATE_ANY, PERMISSION_TOPIC_UPDATE_SELF,
};
use crate::repositories::{
    repository_topic_to_detail, repository_topic_to_summary, CategoryRepository, NewTopic,
    RepositoryTopic, TopicListOptions, TopicModeration, TopicRepository, TopicUpdate,
};

use super::category::{generated_slug, normalize_slug};

const DEFAULT_PAGE_SIZE: u32 = 20;
const MAX_PAGE_SIZE: u32 = 100;
const MAX_PAGE: u32 = 1_000_000;

#[derive(Clone)]
pub struct TopicService {
    topics: TopicRepository,
    categories: CategoryRepository,
}

#[derive(Debug, Error)]
pub enum TopicError {
    #[error("invalid topic input: {0}")]
    Validation(&'static str),
    #[error("topic not found")]
    NotFound,
    #[error("category not found or unavailable")]
    CategoryUnavailable,
    #[error("could not generate a unique topic slug")]
    SlugConflict,
    #[error("permission denied")]
    Forbidden,
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

impl TopicService {
    pub fn new(topics: TopicRepository, categories: CategoryRepository) -> Self {
        Self { topics, categories }
    }

    pub async fn list_public(
        &self,
        query: TopicListQuery,
    ) -> Result<Paginated<TopicSummary>, TopicError> {
        let page = query.page.unwrap_or(1);
        let page_size = query.page_size.unwrap_or(DEFAULT_PAGE_SIZE);
        if page == 0 || page > MAX_PAGE {
            return Err(TopicError::Validation("page is out of range"));
        }
        if page_size == 0 || page_size > MAX_PAGE_SIZE {
            return Err(TopicError::Validation(
                "page size must be between 1 and 100",
            ));
        }
        let category_slug = query
            .category
            .map(|value| normalize_slug(&value, 64).map_err(map_category_validation))
            .transpose()?;
        let sort = query.sort.unwrap_or_default();
        let offset = i64::from(page - 1) * i64::from(page_size);
        let (topics, total) = self
            .topics
            .list(TopicListOptions {
                category_slug: category_slug.as_deref(),
                sort,
                limit: i64::from(page_size),
                offset,
            })
            .await
            .map_err(internal)?;
        let total =
            u64::try_from(total).map_err(|_| internal(anyhow::anyhow!("negative topic count")))?;

        Ok(Paginated {
            items: topics
                .into_iter()
                .map(repository_topic_to_summary)
                .collect(),
            pagination: PaginationMeta::new(page, page_size, total),
        })
    }

    pub async fn get_public(&self, slug: &str) -> Result<TopicDetail, TopicError> {
        let slug = normalize_slug(slug, 220).map_err(map_category_validation)?;
        let topic = self
            .topics
            .find_published_by_slug_and_increment(&slug)
            .await
            .map_err(internal)?
            .ok_or(TopicError::NotFound)?;
        to_detail(topic)
    }

    pub async fn create(
        &self,
        principal: &AuthenticatedPrincipal,
        request: CreateTopicRequest,
    ) -> Result<TopicDetail, TopicError> {
        require(principal, PERMISSION_TOPIC_CREATE)?;
        self.ensure_category_usable(principal, request.category_id)
            .await?;
        let title = normalize_title(request.title)?;
        let content = normalize_content(request.content)?;
        let summary = match request.summary {
            Some(summary) => normalize_summary(Some(summary))?,
            None => Some(markdown_summary(&content, 240)),
        };
        let base_slug = generated_slug(&title, "topic", 200);

        for attempt in 0..4 {
            let slug = if attempt == 0 && base_slug != "topic" {
                base_slug.clone()
            } else {
                format!("{base_slug}-{}", short_suffix())
            };
            match self
                .topics
                .create(NewTopic {
                    category_id: request.category_id,
                    author_id: principal.user_id,
                    title: &title,
                    slug: &slug,
                    content: &content,
                    summary: summary.as_deref(),
                })
                .await
            {
                Ok(topic) => return to_detail(topic),
                Err(error) if is_database_code(&error, "23505") => continue,
                Err(error) if is_database_code(&error, "23503") => {
                    return Err(TopicError::CategoryUnavailable);
                }
                Err(error) => return Err(internal(error)),
            }
        }
        Err(TopicError::SlugConflict)
    }

    pub async fn update(
        &self,
        principal: &AuthenticatedPrincipal,
        topic_id: Uuid,
        request: UpdateTopicRequest,
    ) -> Result<TopicDetail, TopicError> {
        let existing = self.find_editable(topic_id).await?;
        require_owner_or_any(
            principal,
            existing.author_id,
            PERMISSION_TOPIC_UPDATE_SELF,
            PERMISSION_TOPIC_UPDATE_ANY,
        )?;
        if let Some(category_id) = request.category_id {
            self.ensure_category_usable(principal, category_id).await?;
        }
        let title = request.title.map(normalize_title).transpose()?;
        let content = request.content.map(normalize_content).transpose()?;
        let (summary_changed, summary) = normalize_summary_patch(request.summary)?;
        if request.category_id.is_none() && title.is_none() && content.is_none() && !summary_changed
        {
            return Err(TopicError::Validation("topic update contains no fields"));
        }

        let topic = self
            .topics
            .update(
                topic_id,
                TopicUpdate {
                    category_id: request.category_id,
                    title: title.as_deref(),
                    content: content.as_deref(),
                    summary_changed,
                    summary: summary.as_deref(),
                },
            )
            .await
            .map_err(internal)?
            .ok_or(TopicError::NotFound)?;
        to_detail(topic)
    }

    pub async fn moderate(
        &self,
        principal: &AuthenticatedPrincipal,
        topic_id: Uuid,
        request: ModerateTopicRequest,
    ) -> Result<TopicDetail, TopicError> {
        if request.is_pinned.is_none() && request.is_featured.is_none() {
            return Err(TopicError::Validation(
                "moderation update contains no fields",
            ));
        }
        if request.is_pinned.is_some() {
            require(principal, PERMISSION_TOPIC_PIN)?;
        }
        if request.is_featured.is_some() {
            require(principal, PERMISSION_TOPIC_FEATURE)?;
        }
        let topic = self
            .topics
            .moderate(
                topic_id,
                TopicModeration {
                    is_pinned: request.is_pinned,
                    is_featured: request.is_featured,
                },
            )
            .await
            .map_err(internal)?
            .ok_or(TopicError::NotFound)?;
        to_detail(topic)
    }

    pub async fn delete(
        &self,
        principal: &AuthenticatedPrincipal,
        topic_id: Uuid,
    ) -> Result<(), TopicError> {
        let existing = self.find_editable(topic_id).await?;
        require_owner_or_any(
            principal,
            existing.author_id,
            PERMISSION_TOPIC_DELETE_SELF,
            PERMISSION_TOPIC_DELETE_ANY,
        )?;
        if self.topics.soft_delete(topic_id).await.map_err(internal)? {
            Ok(())
        } else {
            Err(TopicError::NotFound)
        }
    }

    async fn find_editable(&self, topic_id: Uuid) -> Result<RepositoryTopic, TopicError> {
        let topic = self
            .topics
            .find_by_id(topic_id)
            .await
            .map_err(internal)?
            .ok_or(TopicError::NotFound)?;
        if topic.status == TopicStatus::Published.as_str() {
            Ok(topic)
        } else {
            Err(TopicError::NotFound)
        }
    }

    async fn ensure_category_usable(
        &self,
        principal: &AuthenticatedPrincipal,
        category_id: Uuid,
    ) -> Result<(), TopicError> {
        let category = self
            .categories
            .find_by_id(category_id)
            .await
            .map_err(internal)?
            .ok_or(TopicError::CategoryUnavailable)?;
        if category.is_visible || principal.has_permission(PERMISSION_CATEGORY_MANAGE) {
            Ok(())
        } else {
            Err(TopicError::CategoryUnavailable)
        }
    }
}

fn normalize_title(value: String) -> Result<String, TopicError> {
    let value = value.trim().to_owned();
    if !(3..=200).contains(&value.chars().count()) {
        return Err(TopicError::Validation(
            "title must contain between 3 and 200 characters",
        ));
    }
    Ok(value)
}

fn normalize_content(value: String) -> Result<String, TopicError> {
    let value = value.trim().to_owned();
    if value.is_empty() || value.chars().count() > 100_000 {
        return Err(TopicError::Validation(
            "content must contain between 1 and 100000 characters",
        ));
    }
    Ok(value)
}

fn normalize_summary(value: Option<String>) -> Result<Option<String>, TopicError> {
    let value = value
        .map(|value| value.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|value| !value.is_empty());
    if value
        .as_ref()
        .is_some_and(|value| value.chars().count() > 500)
    {
        return Err(TopicError::Validation(
            "summary cannot exceed 500 characters",
        ));
    }
    Ok(value)
}

fn normalize_summary_patch(
    field: PatchField<String>,
) -> Result<(bool, Option<String>), TopicError> {
    match field {
        PatchField::Missing => Ok((false, None)),
        PatchField::Set(value) => normalize_summary(value).map(|value| (true, value)),
    }
}

fn markdown_summary(content: &str, max_chars: usize) -> String {
    content
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(max_chars)
        .collect()
}

fn short_suffix() -> String {
    Uuid::new_v4().simple().to_string()[..8].to_owned()
}

fn require(principal: &AuthenticatedPrincipal, permission: &'static str) -> Result<(), TopicError> {
    if principal.has_permission(permission) {
        Ok(())
    } else {
        Err(TopicError::Forbidden)
    }
}

fn require_owner_or_any(
    principal: &AuthenticatedPrincipal,
    author_id: Uuid,
    self_permission: &'static str,
    any_permission: &'static str,
) -> Result<(), TopicError> {
    let allowed = if principal.user_id == author_id {
        principal.has_permission(self_permission) || principal.has_permission(any_permission)
    } else {
        principal.has_permission(any_permission)
    };
    if allowed {
        Ok(())
    } else {
        Err(TopicError::Forbidden)
    }
}

fn map_category_validation(error: super::CategoryError) -> TopicError {
    match error {
        super::CategoryError::Validation(message) => TopicError::Validation(message),
        _ => TopicError::Validation("invalid category slug"),
    }
}

fn is_database_code(error: &sqlx::Error, expected: &str) -> bool {
    error
        .as_database_error()
        .is_some_and(|error| error.code().as_deref() == Some(expected))
}

fn to_detail(topic: RepositoryTopic) -> Result<TopicDetail, TopicError> {
    repository_topic_to_detail(topic)
        .map_err(|_| internal(anyhow::anyhow!("topic detail content was not selected")))
}

fn internal(error: impl Into<anyhow::Error>) -> TopicError {
    TopicError::Internal(error.into())
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use crate::models::{
        AuthenticatedPrincipal, PERMISSION_TOPIC_UPDATE_ANY, PERMISSION_TOPIC_UPDATE_SELF,
        ROLE_USER,
    };

    use super::{markdown_summary, normalize_content, normalize_title, require_owner_or_any};

    #[test]
    fn normalizes_topic_fields() {
        assert_eq!(
            normalize_title("  A valid title  ".into()).unwrap(),
            "A valid title"
        );
        assert_eq!(
            normalize_content("  # Markdown  ".into()).unwrap(),
            "# Markdown"
        );
    }

    #[test]
    fn creates_bounded_markdown_summary() {
        let summary = markdown_summary("# Hello\n\nThis   is a topic.", 12);
        assert_eq!(summary, "# Hello This");
    }

    #[test]
    fn enforces_self_and_any_update_permissions() {
        let user_id = Uuid::new_v4();
        let other_id = Uuid::new_v4();
        let self_editor = principal(user_id, [PERMISSION_TOPIC_UPDATE_SELF]);
        assert!(require_owner_or_any(
            &self_editor,
            user_id,
            PERMISSION_TOPIC_UPDATE_SELF,
            PERMISSION_TOPIC_UPDATE_ANY,
        )
        .is_ok());
        assert!(require_owner_or_any(
            &self_editor,
            other_id,
            PERMISSION_TOPIC_UPDATE_SELF,
            PERMISSION_TOPIC_UPDATE_ANY,
        )
        .is_err());

        let moderator = principal(user_id, [PERMISSION_TOPIC_UPDATE_ANY]);
        assert!(require_owner_or_any(
            &moderator,
            other_id,
            PERMISSION_TOPIC_UPDATE_SELF,
            PERMISSION_TOPIC_UPDATE_ANY,
        )
        .is_ok());
    }

    fn principal(
        user_id: Uuid,
        permissions: impl IntoIterator<Item = &'static str>,
    ) -> AuthenticatedPrincipal {
        AuthenticatedPrincipal::new(
            user_id,
            ROLE_USER.into(),
            0,
            Uuid::new_v4(),
            permissions.into_iter().map(str::to_owned),
        )
    }
}
