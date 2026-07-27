use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{UploadCategory, UploadResponse};

#[derive(Clone, Debug, sqlx::FromRow)]
pub struct RepositoryUpload {
    pub id: Uuid,
    pub user_id: Uuid,
    pub filename: String,
    pub original_filename: String,
    pub storage_provider: String,
    pub storage_key: String,
    pub mime_type: String,
    pub file_size: i64,
    pub category: String,
    pub url: Option<String>,
    pub thumbnail_storage_key: Option<String>,
    pub thumbnail_url: Option<String>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct NewUpload<'a> {
    pub id: Uuid,
    pub user_id: Uuid,
    pub filename: &'a str,
    pub original_filename: &'a str,
    pub storage_provider: &'a str,
    pub storage_key: &'a str,
    pub mime_type: &'a str,
    pub file_size: i64,
    pub category: UploadCategory,
    pub thumbnail_storage_key: Option<&'a str>,
    pub width: Option<i32>,
    pub height: Option<i32>,
}

#[derive(Clone)]
pub struct UploadRepository {
    pool: PgPool,
}

impl UploadRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create_pending(&self, upload: NewUpload<'_>) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO uploads (
                id, user_id, filename, original_filename, storage_provider, storage_key,
                mime_type, file_size, category, thumbnail_storage_key, width, height
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            "#,
        )
        .bind(upload.id)
        .bind(upload.user_id)
        .bind(upload.filename)
        .bind(upload.original_filename)
        .bind(upload.storage_provider)
        .bind(upload.storage_key)
        .bind(upload.mime_type)
        .bind(upload.file_size)
        .bind(upload.category.as_str())
        .bind(upload.thumbnail_storage_key)
        .bind(upload.width)
        .bind(upload.height)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn mark_ready(
        &self,
        upload_id: Uuid,
        url: &str,
        thumbnail_url: Option<&str>,
    ) -> Result<Option<RepositoryUpload>, sqlx::Error> {
        sqlx::query_as::<_, RepositoryUpload>(
            r#"
            UPDATE uploads
            SET status = 'ready', url = $2, thumbnail_url = $3
            WHERE id = $1 AND status = 'pending'
            RETURNING id, user_id, filename, original_filename, storage_provider, storage_key,
                      mime_type, file_size, category, url, thumbnail_storage_key, thumbnail_url,
                      width, height, status, created_at, updated_at
            "#,
        )
        .bind(upload_id)
        .bind(url)
        .bind(thumbnail_url)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn mark_failed(&self, upload_id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE uploads SET status = 'failed' WHERE id = $1 AND status = 'pending'")
            .bind(upload_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn find_ready(
        &self,
        upload_id: Uuid,
    ) -> Result<Option<RepositoryUpload>, sqlx::Error> {
        sqlx::query_as::<_, RepositoryUpload>(&format!(
            "{UPLOAD_COLUMNS} WHERE id = $1 AND status = 'ready'"
        ))
        .bind(upload_id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn list_ready_by_user(
        &self,
        user_id: Uuid,
        category: Option<UploadCategory>,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<RepositoryUpload>, i64), sqlx::Error> {
        let category = category.map(UploadCategory::as_str);
        let items = sqlx::query_as::<_, RepositoryUpload>(&format!(
            "{UPLOAD_COLUMNS} WHERE user_id = $1 AND status = 'ready' \
             AND ($2::text IS NULL OR category = $2) ORDER BY created_at DESC LIMIT $3 OFFSET $4"
        ))
        .bind(user_id)
        .bind(category)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;
        let total = sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM uploads WHERE user_id = $1 AND status = 'ready' \
             AND ($2::text IS NULL OR category = $2)",
        )
        .bind(user_id)
        .bind(category)
        .fetch_one(&self.pool)
        .await?;
        Ok((items, total))
    }

    pub async fn begin_delete(
        &self,
        upload_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<RepositoryUpload>, sqlx::Error> {
        sqlx::query_as::<_, RepositoryUpload>(
            r#"
            UPDATE uploads SET status = 'deleting'
            WHERE id = $1 AND user_id = $2 AND status = 'ready'
            RETURNING id, user_id, filename, original_filename, storage_provider, storage_key,
                      mime_type, file_size, category, url, thumbnail_storage_key, thumbnail_url,
                      width, height, status, created_at, updated_at
            "#,
        )
        .bind(upload_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn restore_ready(&self, upload_id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE uploads SET status = 'ready' WHERE id = $1 AND status = 'deleting'")
            .bind(upload_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn mark_deleted(&self, upload_id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            WITH changed AS (
                UPDATE uploads
                SET status = 'deleted', deleted_at = now(), url = NULL, thumbnail_url = NULL
                WHERE id = $1 AND status = 'deleting'
                RETURNING id
            )
            UPDATE users SET avatar_url = NULL, avatar_upload_id = NULL
            WHERE avatar_upload_id = $1 AND EXISTS (SELECT 1 FROM changed)
            "#,
        )
        .bind(upload_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn set_avatar(
        &self,
        user_id: Uuid,
        upload_id: Uuid,
    ) -> Result<Option<Option<Uuid>>, sqlx::Error> {
        let result = sqlx::query_as::<_, (Option<Uuid>,)>(
            r#"
            WITH candidate AS (
                SELECT id, url FROM uploads
                WHERE id = $2 AND user_id = $1 AND category = 'avatar' AND status = 'ready'
            ), previous AS (
                SELECT avatar_upload_id FROM users WHERE id = $1
            )
            UPDATE users
            SET avatar_url = (SELECT url FROM candidate),
                avatar_upload_id = (SELECT id FROM candidate)
            WHERE id = $1 AND EXISTS (SELECT 1 FROM candidate)
            RETURNING (SELECT avatar_upload_id FROM previous)
            "#,
        )
        .bind(user_id)
        .bind(upload_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(result.map(|row| row.0))
    }

    pub async fn clear_avatar(&self, user_id: Uuid) -> Result<Option<Uuid>, sqlx::Error> {
        sqlx::query_scalar::<_, Option<Uuid>>(
            r#"
            WITH previous AS (
                SELECT avatar_upload_id FROM users WHERE id = $1
            )
            UPDATE users SET avatar_url = NULL, avatar_upload_id = NULL WHERE id = $1
            RETURNING (SELECT avatar_upload_id FROM previous)
            "#,
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map(Option::flatten)
    }
}

pub fn repository_upload_to_response(
    upload: RepositoryUpload,
) -> Result<UploadResponse, &'static str> {
    let category = upload.category.parse::<UploadCategory>()?;
    Ok(UploadResponse {
        id: upload.id,
        user_id: upload.user_id,
        filename: upload.filename,
        original_filename: upload.original_filename,
        storage_provider: upload.storage_provider,
        mime_type: upload.mime_type,
        file_size: upload.file_size,
        category,
        url: upload.url.ok_or("ready upload has no URL")?,
        thumbnail_url: upload.thumbnail_url,
        width: upload.width,
        height: upload.height,
        created_at: upload.created_at,
        updated_at: upload.updated_at,
    })
}

const UPLOAD_COLUMNS: &str = r#"
    SELECT id, user_id, filename, original_filename, storage_provider, storage_key,
           mime_type, file_size, category, url, thumbnail_storage_key, thumbnail_url,
           width, height, status, created_at, updated_at
    FROM uploads
"#;
