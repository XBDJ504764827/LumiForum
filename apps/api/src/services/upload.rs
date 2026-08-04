use std::sync::Arc;

use bytes::Bytes;
use chrono::{Datelike, Utc};
use thiserror::Error;
use uuid::Uuid;

use crate::models::{Paginated, PaginationMeta, UploadCategory, UploadListQuery, UploadResponse};
use crate::repositories::{
    repository_upload_to_response, NewUpload, RepositoryUpload, UploadRepository,
};
use crate::services::ModerationService;
use crate::storage::{PutOptions, StorageProvider};

use super::upload_image::process_image;

const CACHE_CONTROL: &str = "public, max-age=31536000, immutable";

/// Accepted non-image types, keyed by the MIME type sniffed from magic bytes.
/// Files whose detected type is not in this table — executables (PE/ELF/Mach-O/
/// Wasm/class), scripts (sh/bat/ps1), HTML, SVG, XML, fonts, certificates — are
/// rejected. Client-provided names and MIME types are only cross-checked against
/// this detection and never trusted on their own.
const ATTACHMENT_TYPES: &[(&str, &str)] = &[
    // Documents
    ("application/pdf", "pdf"),
    ("application/msword", "doc"),
    (
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "docx",
    ),
    ("application/vnd.ms-excel", "xls"),
    (
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "xlsx",
    ),
    ("application/vnd.ms-powerpoint", "ppt"),
    (
        "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        "pptx",
    ),
    ("application/vnd.oasis.opendocument.text", "odt"),
    ("application/vnd.oasis.opendocument.spreadsheet", "ods"),
    ("application/vnd.oasis.opendocument.presentation", "odp"),
    ("application/rtf", "rtf"),
    ("text/plain", "txt"),
    // Archives
    ("application/zip", "zip"),
    ("application/x-7z-compressed", "7z"),
    ("application/vnd.rar", "rar"),
    ("application/x-tar", "tar"),
    ("application/gzip", "gz"),
    ("application/x-bzip2", "bz2"),
    ("application/x-xz", "xz"),
    ("application/zstd", "zst"),
    // Audio
    ("audio/mpeg", "mp3"),
    ("audio/m4a", "m4a"),
    ("audio/ogg", "ogg"),
    ("audio/opus", "opus"),
    ("audio/x-flac", "flac"),
    ("audio/x-wav", "wav"),
    ("audio/aac", "aac"),
    ("audio/midi", "midi"),
    // Video
    ("video/mp4", "mp4"),
    ("video/webm", "webm"),
    ("video/x-matroska", "mkv"),
    ("video/quicktime", "mov"),
    ("video/x-msvideo", "avi"),
    ("video/x-ms-wmv", "wmv"),
    ("video/mpeg", "mpg"),
    ("video/x-m4v", "m4v"),
];

fn attachment_extension(mime_type: &str) -> Option<&'static str> {
    ATTACHMENT_TYPES
        .iter()
        .find(|(mime, _)| *mime == mime_type)
        .map(|(_, extension)| *extension)
}

/// Legacy MIME aliases that browsers and file managers (notably Windows
/// Explorer) claim for files whose verified content has a canonical type.
/// Keyed by the claimed type, mapped to the canonical detected type.
const CLAIMED_MIME_ALIASES: &[(&str, &str)] = &[
    ("application/x-zip-compressed", "application/zip"),
    ("application/zip-compressed", "application/zip"),
    ("application/x-rar-compressed", "application/vnd.rar"),
    ("application/x-gzip", "application/gzip"),
    ("application/x-compressed-tar", "application/gzip"),
    ("application/x-bzip", "application/x-bzip2"),
    ("application/x-zstd", "application/zstd"),
    ("application/mp4", "video/mp4"),
    ("application/x-m4a", "audio/m4a"),
    ("audio/x-mpeg", "audio/mpeg"),
    ("audio/mp3", "audio/mpeg"),
    ("audio/wav", "audio/x-wav"),
    ("image/jpg", "image/jpeg"),
];

/// Whether the browser-declared MIME type is compatible with the type sniffed
/// from the file content. The declared value is only a hint: content sniffing
/// is authoritative for what gets stored and served. Comparison is
/// case-insensitive, ignores parameters, and tolerates the legacy aliases
/// above; a blatant mismatch (e.g. a ZIP claiming to be an image) is still
/// rejected.
fn claimed_mime_matches(claimed: &str, detected: &str) -> bool {
    let claimed = claimed
        .split(';')
        .next()
        .unwrap_or(claimed)
        .trim()
        .to_ascii_lowercase();
    if claimed.is_empty() || claimed == "application/octet-stream" {
        return true;
    }
    if claimed == detected {
        return true;
    }
    if CLAIMED_MIME_ALIASES
        .iter()
        .any(|(alias, canonical)| *alias == claimed && *canonical == detected)
    {
        return true;
    }
    // Markdown, CSV, logs, … are all verified plain text; accept any text/*
    // declaration for them.
    detected == "text/plain" && claimed.starts_with("text/")
}

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
    moderation: Option<ModerationService>,
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
    /// "inline" for processed images, "attachment" for everything else, so
    /// uploaded content can never be rendered inside the forum origin.
    content_disposition: &'static str,
    width: Option<i32>,
    height: Option<i32>,
}

impl UploadService {
    /// Backward-compatible constructor used by isolated upload tests.
    pub fn new(repository: UploadRepository, storage: Arc<dyn StorageProvider>) -> Self {
        Self {
            repository,
            storage,
            moderation: None,
        }
    }

    pub fn with_moderation(
        repository: UploadRepository,
        storage: Arc<dyn StorageProvider>,
        moderation: ModerationService,
    ) -> Self {
        Self {
            repository,
            storage,
            moderation: Some(moderation),
        }
    }

    pub async fn create(
        &self,
        user_id: Uuid,
        input: UploadInput,
    ) -> Result<UploadResponse, UploadError> {
        if let Some(moderation) = &self.moderation {
            moderation
                .enforce_upload_creation(user_id)
                .await
                .map_err(map_moderation)?;
            moderation
                .enforce_upload_allowed(user_id)
                .await
                .map_err(map_moderation)?;
            moderation
                .enforce_upload_rate_limit(user_id)
                .await
                .map_err(map_moderation)?;
        }
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
            .is_some_and(|claimed| !claimed_mime_matches(claimed, detected_mime))
        {
            return Err(UploadError::UnsupportedMediaType);
        }
        let prepared = prepare(input.category, input.data, detected_mime)?;
        if prepared.data.len() > input.category.max_bytes() {
            return Err(UploadError::TooLarge);
        }
        let content_disposition = prepared.content_disposition;

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
                    content_disposition,
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
                        content_disposition: "inline",
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
        self.finish_delete(upload_id, &upload).await
    }

    pub async fn admin_delete(&self, upload_id: Uuid) -> Result<(), UploadError> {
        let upload = self
            .repository
            .begin_delete_any(upload_id)
            .await
            .map_err(internal)?
            .ok_or(UploadError::NotFound)?;
        self.finish_delete(upload_id, &upload).await
    }

    async fn finish_delete(
        &self,
        upload_id: Uuid,
        upload: &crate::repositories::RepositoryUpload,
    ) -> Result<(), UploadError> {
        if self.delete_objects(upload).await.is_err() {
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
            content_disposition: "inline",
            width: Some(image.width),
            height: Some(image.height),
        });
    }

    let extension = attachment_extension(mime_type).ok_or(UploadError::UnsupportedMediaType)?;
    Ok(PreparedUpload {
        data,
        thumbnail: None,
        mime_type,
        extension,
        content_disposition: "attachment",
        width: None,
        height: None,
    })
}

fn detect_mime(data: &[u8]) -> Option<&'static str> {
    if let Some(kind) = infer::get(data) {
        let mime = kind.mime_type();
        return match mime {
            "image/jpeg" | "image/png" | "image/webp" | "image/gif" => Some(mime),
            _ if attachment_extension(mime).is_some() => Some(mime),
            // Everything else sniffable by `infer` — PE/ELF/Mach-O executables,
            // Java classes, Wasm, shell scripts, HTML, XML, SVG, fonts, MSI,
            // Debian packages, certificates — is rejected here.
            _ => None,
        };
    }
    // No magic signature: accept only UTF-8 plain text, and only when it does
    // not start like an executable script or an active-content document.
    let text = std::str::from_utf8(data).ok()?;
    if text.contains('\0') || starts_with_active_content(text) {
        return None;
    }
    Some("text/plain")
}

/// Rejects text that begins like a script (shebang) or a markup document that
/// browsers can execute or render as active content (SVG/XML/HTML). Only the
/// file head is inspected, so normal documents that merely mention such syntax
/// in the middle of the text are unaffected.
fn starts_with_active_content(text: &str) -> bool {
    let head = text
        .trim_start_matches(|character: char| character.is_whitespace() || character == '\u{feff}')
        .get(..32)
        .unwrap_or_default()
        .to_ascii_lowercase();
    head.starts_with("#!")
        || head.starts_with("<svg")
        || head.starts_with("<?xml")
        || head.starts_with("<!doctype")
        || head.starts_with("<html")
        || head.starts_with("<script")
        || head.starts_with("<iframe")
        || head.starts_with("<style")
        || head.starts_with("<title")
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

fn map_moderation(error: crate::services::ModerationError) -> UploadError {
    match error {
        crate::services::ModerationError::Validation(message) => UploadError::Validation(message),
        crate::services::ModerationError::Forbidden => UploadError::Forbidden,
        crate::services::ModerationError::RateLimited => {
            UploadError::Validation("upload rate limited")
        }
        _ => UploadError::Internal(anyhow::anyhow!("moderation rejected upload")),
    }
}

#[cfg(test)]
mod tests {
    use bytes::Bytes;

    use super::{detect_mime, prepare, sanitize_filename};
    use crate::models::UploadCategory;

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

    #[test]
    fn accepts_common_documents_archives_audio_and_video() {
        assert_eq!(
            detect_mime(b"%PDF-1.7\n%\xe2\xe3\xcf\xd3"),
            Some("application/pdf")
        );
        assert_eq!(
            detect_mime(b"PK\x03\x04\x14\x00\x00\x00"),
            Some("application/zip")
        );
        assert_eq!(
            detect_mime(b"ID3\x04\x00\x00\x00\x00\x00\x00"),
            Some("audio/mpeg")
        );
        assert_eq!(
            detect_mime(b"RIFF\x24\x00\x00\x00WAVEfmt "),
            Some("audio/x-wav")
        );
        assert_eq!(
            detect_mime(b"\x1a\x45\xdf\xa3\x93\x42\x82\x88matroska"),
            Some("video/x-matroska")
        );
    }

    #[test]
    fn rejects_executables_scripts_and_active_documents() {
        // Windows PE / Unix ELF executables
        assert_eq!(detect_mime(b"MZ\x90\x00\x03\x00\x00\x00"), None);
        assert_eq!(detect_mime(b"\x7fELF\x02\x01\x01\x00\x00\x00\x00"), None);
        // Shell scripts and shebang fallback
        assert_eq!(detect_mime(b"#!/bin/sh\necho pwned"), None);
        assert_eq!(detect_mime(b"#! /usr/bin/env python3\nprint(1)"), None);
        // SVG / XML that could be rendered as active content
        assert_eq!(
            detect_mime(b"<svg xmlns=\"http://www.w3.org/2000/svg\"><script>"),
            None
        );
        assert_eq!(detect_mime(b"  <?xml version=\"1.0\"?><root/>"), None);
        // Java class / WebAssembly bytecode
        assert_eq!(detect_mime(b"\xca\xfe\xba\xbe\x00\x00\x00\x34"), None);
        assert_eq!(detect_mime(b"\x00asm\x01\x00\x00\x00"), None);
    }

    #[test]
    fn accepts_browser_mime_aliases_and_rejects_blatant_mismatches() {
        use super::claimed_mime_matches;

        // Windows file pickers claim these legacy types for common archives.
        assert!(claimed_mime_matches(
            "application/x-zip-compressed",
            "application/zip"
        ));
        assert!(claimed_mime_matches(
            "application/x-gzip",
            "application/gzip"
        ));
        assert!(claimed_mime_matches(
            "application/x-rar-compressed",
            "application/vnd.rar"
        ));
        assert!(claimed_mime_matches("audio/x-mpeg", "audio/mpeg"));
        // Parameters and case must not matter.
        assert!(claimed_mime_matches(
            "text/plain; charset=utf-8",
            "text/plain"
        ));
        assert!(claimed_mime_matches("APPLICATION/ZIP", "application/zip"));
        // Unknown or generic declarations never cause a rejection.
        assert!(claimed_mime_matches(
            "application/octet-stream",
            "application/zip"
        ));
        assert!(claimed_mime_matches("", "application/zip"));
        // Markdown/CSV-style claims are fine for verified plain text.
        assert!(claimed_mime_matches("text/markdown", "text/plain"));
        // A ZIP claiming to be an image is a renamed file — still rejected.
        assert!(!claimed_mime_matches("image/png", "application/zip"));
        assert!(!claimed_mime_matches("text/html", "application/zip"));
    }

    #[test]
    fn maps_verified_mime_to_server_owned_extension() {
        let zip = Bytes::from_static(b"PK\x03\x04\x14\x00\x00\x00");
        let prepared = prepare(UploadCategory::Attachment, zip, "application/zip").unwrap();
        assert_eq!(prepared.extension, "zip");
        assert_eq!(prepared.mime_type, "application/zip");
        assert_eq!(prepared.content_disposition, "attachment");

        // A sniffed executable must never be stored, even if `prepare` is
        // called with its MIME type directly.
        let exe = Bytes::from_static(b"MZ\x90\x00");
        let rejected = prepare(
            UploadCategory::Attachment,
            exe,
            "application/vnd.microsoft.portable-executable",
        );
        assert!(matches!(
            rejected,
            Err(super::UploadError::UnsupportedMediaType)
        ));
    }
}
