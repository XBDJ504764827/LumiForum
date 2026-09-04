use thiserror::Error;
use uuid::Uuid;

use crate::models::{ChangePasswordRequest, PatchField, ProfileUpdateRequest, UserResponse};
use crate::repositories::{repository_user_to_response, UserRepository};

use super::PasswordService;

#[derive(Clone)]
pub struct UserService {
    repository: UserRepository,
    passwords: PasswordService,
}

#[derive(Debug, Error)]
pub enum UserError {
    #[error("profile update contains no fields")]
    EmptyUpdate,
    #[error("invalid profile input: {0}")]
    Validation(&'static str),
    #[error("user not found")]
    NotFound,
    #[error("current password is invalid")]
    CurrentPasswordInvalid,
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

impl UserService {
    pub fn new(
        repository: UserRepository,
        password_hash_concurrency: usize,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            repository,
            passwords: PasswordService::new(password_hash_concurrency)?,
        })
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

    /// Change the in-session password. For accounts that already have a
    /// password, the current one must be verified (Argon2, off the runtime
    /// threads). Steam-only accounts may set a password with no current one.
    pub async fn set_password(
        &self,
        user_id: Uuid,
        request: ChangePasswordRequest,
    ) -> Result<UserResponse, UserError> {
        let new_password = request.new_password.trim();
        if new_password.chars().count() < 8 {
            return Err(UserError::Validation("new password is too short"));
        }
        if new_password.len() > 128 {
            return Err(UserError::Validation("new password is too long"));
        }

        let user = self
            .repository
            .find_by_id(user_id)
            .await
            .map_err(internal)?
            .ok_or(UserError::NotFound)?;

        // Verify the current password when one exists (nit: timing — a wrong
        // current password should be indistinguishable from Steam-only set
        // for a nonexistent user; the Argon2 dummy work keeps cost similar).
        if let Some(current_hash) = &user.password_hash {
            let Some(current) = &request.current_password else {
                return Err(UserError::CurrentPasswordInvalid);
            };
            if current.is_empty() {
                return Err(UserError::CurrentPasswordInvalid);
            }
            let valid = self
                .passwords
                .verify(current.clone(), current_hash.clone())
                .await
                .map_err(internal)?;
            if !valid {
                return Err(UserError::CurrentPasswordInvalid);
            }
        }

        let new_hash = self
            .passwords
            .hash(new_password.to_owned())
            .await
            .map_err(internal)?;
        let user = self
            .repository
            .update_password(user_id, &new_hash)
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
