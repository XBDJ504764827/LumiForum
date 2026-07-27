use thiserror::Error;
use uuid::Uuid;

use crate::models::{PatchField, ProfileUpdateRequest, UserResponse};
use crate::repositories::{repository_user_to_response, UserRepository};

#[derive(Clone)]
pub struct UserService {
    repository: UserRepository,
}

#[derive(Debug, Error)]
pub enum UserError {
    #[error("profile update contains no fields")]
    EmptyUpdate,
    #[error("invalid profile input: {0}")]
    Validation(&'static str),
    #[error("user not found")]
    NotFound,
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

impl UserService {
    pub fn new(repository: UserRepository) -> Self {
        Self { repository }
    }

    pub async fn get_profile(&self, user_id: Uuid) -> Result<UserResponse, UserError> {
        let user = self
            .repository
            .find_by_id(user_id)
            .await
            .map_err(internal)?
            .ok_or(UserError::NotFound)?;
        to_response(user)
    }

    pub async fn update_profile(
        &self,
        user_id: Uuid,
        request: ProfileUpdateRequest,
    ) -> Result<UserResponse, UserError> {
        let (nickname_changed, nickname) = normalize_nickname(request.nickname)?;
        if !nickname_changed {
            return Err(UserError::EmptyUpdate);
        }

        let user = self
            .repository
            .update_profile(user_id, nickname_changed, nickname.as_deref())
            .await
            .map_err(internal)?
            .ok_or(UserError::NotFound)?;
        to_response(user)
    }
}

fn normalize_nickname(field: PatchField<String>) -> Result<(bool, Option<String>), UserError> {
    match field {
        PatchField::Missing => Ok((false, None)),
        PatchField::Set(value) => {
            let value = value
                .map(|value| value.trim().to_owned())
                .filter(|value| !value.is_empty());
            if value
                .as_ref()
                .is_some_and(|value| value.chars().count() > 64)
            {
                return Err(UserError::Validation("nickname is too long"));
            }
            Ok((true, value))
        }
    }
}

fn to_response(user: crate::repositories::RepositoryUser) -> Result<UserResponse, UserError> {
    repository_user_to_response(user)
        .map_err(|_| internal(anyhow::anyhow!("unknown persisted user status")))
}

fn internal(error: impl Into<anyhow::Error>) -> UserError {
    UserError::Internal(error.into())
}

#[cfg(test)]
mod tests {
    use crate::models::{PatchField, ProfileUpdateRequest};

    use super::normalize_nickname;

    #[test]
    fn distinguishes_missing_and_cleared_fields() {
        assert_eq!(
            normalize_nickname(PatchField::Set(Some("  Lumi  ".into()))).unwrap(),
            (true, Some("Lumi".into()))
        );
    }

    #[test]
    fn deserializes_patch_field_states() {
        let missing: ProfileUpdateRequest = serde_json::from_str("{}").unwrap();
        let cleared: ProfileUpdateRequest = serde_json::from_str(r#"{"nickname":null}"#).unwrap();

        assert!(matches!(missing.nickname, PatchField::Missing));
        assert!(matches!(cleared.nickname, PatchField::Set(None)));
    }
}
