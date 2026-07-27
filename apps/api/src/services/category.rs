use thiserror::Error;
use uuid::Uuid;

use crate::models::{
    AuthenticatedPrincipal, CategoryResponse, CreateCategoryRequest, PatchField,
    UpdateCategoryRequest, PERMISSION_CATEGORY_MANAGE,
};
use crate::repositories::{
    repository_category_to_response, CategoryRepository, CategoryUpdate, NewCategory,
};

#[derive(Clone)]
pub struct CategoryService {
    repository: CategoryRepository,
}

#[derive(Debug, Error)]
pub enum CategoryError {
    #[error("invalid category input: {0}")]
    Validation(&'static str),
    #[error("category slug is already in use")]
    SlugConflict,
    #[error("category not found")]
    NotFound,
    #[error("category contains topics")]
    NotEmpty,
    #[error("permission denied")]
    Forbidden,
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

impl CategoryService {
    pub fn new(repository: CategoryRepository) -> Self {
        Self { repository }
    }

    pub async fn list_public(&self) -> Result<Vec<CategoryResponse>, CategoryError> {
        let categories = self.repository.list(false).await.map_err(internal)?;
        Ok(categories
            .into_iter()
            .map(repository_category_to_response)
            .collect())
    }

    pub async fn list_admin(&self) -> Result<Vec<CategoryResponse>, CategoryError> {
        let categories = self.repository.list(true).await.map_err(internal)?;
        Ok(categories
            .into_iter()
            .map(repository_category_to_response)
            .collect())
    }

    pub async fn get_public(&self, slug: &str) -> Result<CategoryResponse, CategoryError> {
        let slug = normalize_slug(slug, 64)?;
        self.repository
            .find_by_slug(&slug, false)
            .await
            .map_err(internal)?
            .map(repository_category_to_response)
            .ok_or(CategoryError::NotFound)
    }

    pub async fn create(
        &self,
        principal: &AuthenticatedPrincipal,
        request: CreateCategoryRequest,
    ) -> Result<CategoryResponse, CategoryError> {
        require_manage(principal)?;
        let name = normalize_name(request.name)?;
        let slug = match request.slug {
            Some(slug) => normalize_slug(&slug, 64)?,
            None => {
                let generated = generated_slug(&name, "category", 64);
                if generated == "category" {
                    format!("category-{}", &Uuid::new_v4().simple().to_string()[..8])
                } else {
                    generated
                }
            }
        };
        let description =
            normalize_optional(request.description, 2_000, "description is too long")?;
        let icon = normalize_optional(request.icon, 64, "icon is too long")?;
        let sort_order = request.sort_order.unwrap_or_default();
        validate_sort_order(sort_order)?;

        self.repository
            .create(NewCategory {
                slug: &slug,
                name: &name,
                description: description.as_deref(),
                icon: icon.as_deref(),
                sort_order,
                is_visible: request.is_visible.unwrap_or(true),
            })
            .await
            .map(repository_category_to_response)
            .map_err(map_write_error)
    }

    pub async fn update(
        &self,
        principal: &AuthenticatedPrincipal,
        category_id: Uuid,
        request: UpdateCategoryRequest,
    ) -> Result<CategoryResponse, CategoryError> {
        require_manage(principal)?;
        let slug = request
            .slug
            .map(|slug| normalize_slug(&slug, 64))
            .transpose()?;
        let name = request.name.map(normalize_name).transpose()?;
        let (description_changed, description) =
            normalize_patch(request.description, 2_000, "description is too long")?;
        let (icon_changed, icon) = normalize_patch(request.icon, 64, "icon is too long")?;
        if let Some(sort_order) = request.sort_order {
            validate_sort_order(sort_order)?;
        }
        if slug.is_none()
            && name.is_none()
            && !description_changed
            && !icon_changed
            && request.sort_order.is_none()
            && request.is_visible.is_none()
        {
            return Err(CategoryError::Validation(
                "category update contains no fields",
            ));
        }

        self.repository
            .update(
                category_id,
                CategoryUpdate {
                    slug: slug.as_deref(),
                    name: name.as_deref(),
                    description_changed,
                    description: description.as_deref(),
                    icon_changed,
                    icon: icon.as_deref(),
                    sort_order: request.sort_order,
                    is_visible: request.is_visible,
                },
            )
            .await
            .map_err(map_write_error)?
            .map(repository_category_to_response)
            .ok_or(CategoryError::NotFound)
    }

    pub async fn delete(
        &self,
        principal: &AuthenticatedPrincipal,
        category_id: Uuid,
    ) -> Result<(), CategoryError> {
        require_manage(principal)?;
        let deleted = self
            .repository
            .delete(category_id)
            .await
            .map_err(map_delete_error)?;
        if deleted {
            Ok(())
        } else {
            Err(CategoryError::NotFound)
        }
    }
}

pub(crate) fn generated_slug(value: &str, fallback: &str, max_len: usize) -> String {
    let mut output = String::with_capacity(value.len().min(max_len));
    let mut separator = false;
    for character in value.chars() {
        if character.is_ascii_alphanumeric() {
            if separator && !output.is_empty() && output.len() < max_len {
                output.push('-');
            }
            separator = false;
            if output.len() < max_len {
                output.push(character.to_ascii_lowercase());
            }
        } else {
            separator = true;
        }
    }
    while output.ends_with('-') {
        output.pop();
    }
    if output.len() < 2 {
        fallback.to_owned()
    } else {
        output
    }
}

pub(crate) fn normalize_slug(value: &str, max_len: usize) -> Result<String, CategoryError> {
    let value = value.trim().to_ascii_lowercase();
    if !(2..=max_len).contains(&value.len())
        || value.starts_with('-')
        || value.ends_with('-')
        || value.chars().any(|character| {
            !character.is_ascii_lowercase() && !character.is_ascii_digit() && character != '-'
        })
        || value.contains("--")
    {
        return Err(CategoryError::Validation("invalid slug"));
    }
    Ok(value)
}

fn normalize_name(value: String) -> Result<String, CategoryError> {
    let value = value.trim().to_owned();
    if value.is_empty() || value.chars().count() > 100 {
        return Err(CategoryError::Validation(
            "name must contain between 1 and 100 characters",
        ));
    }
    Ok(value)
}

fn normalize_optional(
    value: Option<String>,
    max_len: usize,
    message: &'static str,
) -> Result<Option<String>, CategoryError> {
    let value = value
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty());
    if value
        .as_ref()
        .is_some_and(|value| value.chars().count() > max_len || value.chars().any(char::is_control))
    {
        return Err(CategoryError::Validation(message));
    }
    Ok(value)
}

fn normalize_patch(
    field: PatchField<String>,
    max_len: usize,
    message: &'static str,
) -> Result<(bool, Option<String>), CategoryError> {
    match field {
        PatchField::Missing => Ok((false, None)),
        PatchField::Set(value) => {
            normalize_optional(value, max_len, message).map(|value| (true, value))
        }
    }
}

fn validate_sort_order(sort_order: i32) -> Result<(), CategoryError> {
    if (-1_000_000..=1_000_000).contains(&sort_order) {
        Ok(())
    } else {
        Err(CategoryError::Validation("sort order is out of range"))
    }
}

fn require_manage(principal: &AuthenticatedPrincipal) -> Result<(), CategoryError> {
    if principal.has_permission(PERMISSION_CATEGORY_MANAGE) {
        Ok(())
    } else {
        Err(CategoryError::Forbidden)
    }
}

fn map_write_error(error: sqlx::Error) -> CategoryError {
    if is_database_code(&error, "23505") {
        CategoryError::SlugConflict
    } else {
        internal(error)
    }
}

fn map_delete_error(error: sqlx::Error) -> CategoryError {
    if is_database_code(&error, "23503") {
        CategoryError::NotEmpty
    } else {
        internal(error)
    }
}

fn is_database_code(error: &sqlx::Error, expected: &str) -> bool {
    error
        .as_database_error()
        .is_some_and(|error| error.code().as_deref() == Some(expected))
}

fn internal(error: impl Into<anyhow::Error>) -> CategoryError {
    CategoryError::Internal(error.into())
}

#[cfg(test)]
mod tests {
    use super::{generated_slug, normalize_slug};

    #[test]
    fn generates_ascii_slugs_with_a_unicode_fallback() {
        assert_eq!(
            generated_slug("Hello, Forum!", "category", 64),
            "hello-forum"
        );
        assert_eq!(generated_slug("综合讨论", "category", 64), "category");
    }

    #[test]
    fn validates_explicit_slugs() {
        assert_eq!(
            normalize_slug(" General-Chat ", 64).unwrap(),
            "general-chat"
        );
        assert!(normalize_slug("bad--slug", 64).is_err());
    }
}
