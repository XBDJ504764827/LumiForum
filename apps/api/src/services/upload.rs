use std::sync::Arc;

use bytes::Bytes;
use chrono::{Datelike, Utc};
use thiserror::Error;
use uuid::Uuid;

use crate::models::{Paginated, PaginationMeta, UploadCategory, UploadListQuery, UploadResponse};
use crate::repositories::{
    repository_upload_to_response, NewUpload, RepositoryUpload, UploadRepository,
};
use crate::storage::{PutOptions, StorageProvider};

use super::upload_image::process_image;

const CACHE_CONTROL: &str = "public, max-age=31536000, immutable";

pub struct UploadInput {
    pub original_filename: String,
    pub claimed_mime_type: Option<String>,
    pub category: UploadCategory,
    pub data: Bytes,
}

#[derive(Clone)]
pub struct UploadService {
    repository: UploadRepository,
    storage: Arc<dyn StorageProvider>,
}

#[derive(Debug, Error)]
pub enum UploadError {
    #[error("invalid upload: {0}")]
    Validation(&'static str),
    #[error("file is too large")]
    TooLarge,
    #[error("unsupported file type")]
    UnsupportedMediaType,
    #[error("upload not found")]
    NotFound,
    #[error("permission denied")]
    Forbidden,
    #[error("storage is unavailable")]
    StorageUnavailable,
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

struct PreparedUpload {
    data: Bytes,
    thumbnail: Option<Bytes>,
    mime_type: &'static str,
    extension: &'static str,
    width: Option<i32>,
    height: Option<i32>,
}

impl UploadService {
    pub fn new(repository: UploadRepository, storage: Arc<dyn StorageProvider>) -> Self {
        Self {
            repository,
            storage,
        }
    }

    pub async fn create(
        &self,
        user_id: Uuid,
        input: UploadInput,
    ) -> Result<UploadResponse, UploadError> {
        if input.data.is_empty() {
            return Err(UploadError::Validation("file is empty"));
        }
        if input.data.len() > input.category.max_bytes() {
            return Err(UploadError::TooLarge);
        }

        let original_filename = sanitize_filename(&input.original_filename);
        let detected_mime = detect_mime(&input.data).ok_or(UploadError::UnsupportedMediaType)?;
        if input
            .claimed_mime_type
            .as_deref()
            .filter(|value| *value != "application/octet-stream")
            .is_some_and(|value| value != detected_mime)
        {
            return Err(UploadError::UnsupportedMediaType);
        }
        let prepared = prepare(input.category, input.data, detected_mime)?;
        if prepared.data.len() > input.category.max_bytes() {
            return Err(UploadError::TooLarge);
        }

        let id = Uuid::new_v4();
        let now = Utc::now();
        let prefix = format!(
            "{}/{:04}/{:02}/{user_id}",
            input.category.as_str(),
            now.year(),
            now.month()
        );
        let filename = format!("{id}.{}", prepared.extension);
        let storage_key = format!("{prefix}/{filename}");
        let thumbnail_key = prepared
            .thumbnail
            .as_ref()
            .map(|_| format!("{prefix}/{id}-thumb.png"));

        self.repository
            .create_pending(NewUpload {
                id,
                user_id,
                filename: &filename,
                original_filename: &original_filename,
                storage_provider: self.storage.name(),
                storage_key: &storage_key,
                mime_type: prepared.mime_type,
                file_size: i64::try_from(prepared.data.len()).map_err(internal)?,
                category: input.category,
                thumbnail_storage_key: thumbnail_key.as_deref(),
                width: prepared.width,
                height: prepared.height,
            })
            .await
            .map_err(internal)?;

        if self
            .storage
            .put(
                &storage_key,
                prepared.data,
                PutOptions {
                    content_type: prepared.mime_type,
                    cache_control: CACHE_CONTROL,
                },
            )
            .await
            .is_err()
        {
            let _ = self.repository.mark_failed(id).await;
            return Err(UploadError::StorageUnavailable);
        }

        if let (Some(key), Some(data)) = (thumbnail_key.as_deref(), prepared.thumbnail) {
            if self
                .storage
                .put(
                    key,
                    data,
                    PutOptions {
                        content_type: "image/png",
                        cache_control: CACHE_CONTROL,
                    },
                )
                .await
                .is_err()
            {
                let _ = self.storage.delete(&storage_key).await;
                let _ = self.repository.mark_failed(id).await;
                return Err(UploadError::StorageUnavailable);
            }
        }

        let url = self.storage.public_url(&storage_key);
        let thumbnail_url = thumbnail_key
            .as_deref()
            .map(|key| self.storage.public_url(key));
        match self
            .repository
            .mark_ready(id, &url, thumbnail_url.as_deref())
            .await
        {
            Ok(Some(upload)) => to_response(upload),
            Ok(None) => {
                self.compensate_objects(&storage_key, thumbnail_key.as_deref())
                    .await;
                Err(internal(anyhow::anyhow!("pending upload disappeared")))
            }
            Err(error) => {
                self.compensate_objects(&storage_key, thumbnail_key.as_deref())
                    .await;
                Err(internal(error))
            }
        }
    }

    pub async fn get(&self, user_id: Uuid, upload_id: Uuid) -> Result<UploadResponse, UploadError> {
        let upload = self
            .repository
            .find_ready(upload_id)
            .await
            .map_err(internal)?
            .ok_or(UploadError::NotFound)?;
        if upload.user_id != user_id {
            return Err(UploadError::Forbidden);
        }
        to_response(upload)
    }

    pub async fn list_user(
        &self,
        viewer_id: Uuid,
        user_id: Uuid,
        query: UploadListQuery,
    ) -> Result<Paginated<UploadResponse>, UploadError> {
        if viewer_id != user_id {
            return Err(UploadError::Forbidden);
        }
        let page = query.page.unwrap_or(1).max(1);
        let page_size = query.page_size.unwrap_or(20).clamp(1, 100);
        let offset = i64::from((page - 1).saturating_mul(page_size));
        let (uploads, total) = self
            .repository
            .list_ready_by_user(user_id, query.category, i64::from(page_size), offset)
            .await
            .map_err(internal)?;
        let items = uploads
            .into_iter()
            .map(to_response)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Paginated {
            items,
            pagination: PaginationMeta::new(page, page_size, u64::try_from(total).unwrap_or(0)),
        })
    }

    pub async fn delete(&self, user_id: Uuid, upload_id: Uuid) -> Result<(), UploadError> {
        let upload = self
            .repository
            .begin_delete(upload_id, user_id)
            .await
            .map_err(internal)?
            .ok_or(UploadError::NotFound)?;
        if self.delete_objects(&upload).await.is_err() {
            self.repository
                .restore_ready(upload_id)
                .await
                .map_err(internal)?;
            return Err(UploadError::StorageUnavailable);
        }
        self.repository
            .mark_deleted(upload_id)
            .await
            .map_err(internal)
    }

    pub async fn create_avatar(
        &self,
        user_id: Uuid,
        mut input: UploadInput,
    ) -> Result<UploadResponse, UploadError> {
        input.category = UploadCategory::Avatar;
        let upload = self.create(user_id, input).await?;
        let previous = self
            .repository
            .set_avatar(user_id, upload.id)
            .await
            .map_err(internal)?
            .ok_or(UploadError::NotFound)?;
        if let Some(previous) = previous.filter(|id| *id != upload.id) {
            if let Err(error) = self.delete(user_id, previous).await {
                tracing::warn!(%previous, %error, "failed to delete replaced avatar");
            }
        }
        Ok(upload)
    }

    pub async fn delete_avatar(&self, user_id: Uuid) -> Result<(), UploadError> {
        if let Some(upload_id) = self
            .repository
            .clear_avatar(user_id)
            .await
            .map_err(internal)?
        {
            self.delete(user_id, upload_id).await?;
        }
        Ok(())
    }

    async fn delete_objects(&self, upload: &RepositoryUpload) -> anyhow::Result<()> {
        self.storage.delete(&upload.storage_key).await?;
        if let Some(key) = upload.thumbnail_storage_key.as_deref() {
            self.storage.delete(key).await?;
        }
        Ok(())
    }

    async fn compensate_objects(&self, key: &str, thumbnail_key: Option<&str>) {
        let _ = self.storage.delete(key).await;
        if let Some(key) = thumbnail_key {
            let _ = self.storage.delete(key).await;
        }
    }
}

fn prepare(
    category: UploadCategory,
    data: Bytes,
    mime_type: &'static str,
) -> Result<PreparedUpload, UploadError> {
    if category.is_image() {
        let image = process_image(category, data, mime_type)
            .map_err(|_| UploadError::UnsupportedMediaType)?;
        return Ok(PreparedUpload {
            data: image.data,
            thumbnail: Some(image.thumbnail),
            mime_type: image.mime_type,
            extension: image.extension,
            width: Some(image.width),
            height: Some(image.height),
        });
    }

    let extension = match mime_type {
        "application/pdf" => "pdf",
        "text/plain" => "txt",
        "application/zip" => "zip",
        _ => return Err(UploadError::UnsupportedMediaType),
    };
    Ok(PreparedUpload {
        data,
        thumbnail: None,
        mime_type,
        extension,
        width: None,
        height: None,
    })
}

fn detect_mime(data: &[u8]) -> Option<&'static str> {
    if let Some(kind) = infer::get(data) {
        return match kind.mime_type() {
            "image/jpeg" => Some("image/jpeg"),
            "image/png" => Some("image/png"),
            "image/webp" => Some("image/webp"),
            "image/gif" => Some("image/gif"),
            "application/pdf" => Some("application/pdf"),
            "application/zip" => Some("application/zip"),
            _ => None,
        };
    }
    std::str::from_utf8(data)
        .ok()
        .filter(|text| !text.contains('\0'))
        .map(|_| "text/plain")
}

fn sanitize_filename(filename: &str) -> String {
    let basename = filename
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or_default()
        .trim();
    let sanitized = basename
        .chars()
        .filter(|character| !character.is_control())
        .take(255)
        .collect::<String>();
    if sanitized.is_empty() {
        "upload".to_owned()
    } else {
        sanitized
    }
}

fn to_response(upload: RepositoryUpload) -> Result<UploadResponse, UploadError> {
    repository_upload_to_response(upload).map_err(|message| internal(anyhow::anyhow!(message)))
}

fn internal(error: impl Into<anyhow::Error>) -> UploadError {
    UploadError::Internal(error.into())
}

#[cfg(test)]
mod tests {
    use super::{detect_mime, sanitize_filename};

    #[test]
    fn removes_path_from_original_filename() {
        assert_eq!(sanitize_filename("../../etc/passwd"), "passwd");
        assert_eq!(sanitize_filename(r"C:\\fakepath\\photo.png"), "photo.png");
    }

    #[test]
    fn rejects_html_and_accepts_plain_text() {
        assert_eq!(detect_mime(b"hello world"), Some("text/plain"));
        assert_eq!(detect_mime(b"<!doctype html><script>x</script>"), None);
    }
}
