use std::{collections::HashMap, io::Cursor, sync::Arc};

use async_trait::async_trait;
use bytes::Bytes;
use image::{DynamicImage, ImageFormat};
use lumiforum_api::{
    models::UploadCategory,
    repositories::UploadRepository,
    services::{UploadError, UploadInput, UploadService},
    storage::{PutOptions, StorageProvider},
};
use sqlx::postgres::PgPoolOptions;
use tokio::sync::Mutex;
use uuid::Uuid;

#[derive(Default)]
struct MemoryStorage {
    objects: Mutex<HashMap<String, Bytes>>,
    fail_put: Mutex<bool>,
}

#[async_trait]
impl StorageProvider for MemoryStorage {
    fn name(&self) -> &'static str {
        "local"
    }

    async fn put(&self, key: &str, data: Bytes, _options: PutOptions<'_>) -> anyhow::Result<()> {
        if *self.fail_put.lock().await {
            anyhow::bail!("injected put failure");
        }
        self.objects.lock().await.insert(key.to_owned(), data);
        Ok(())
    }

    async fn delete(&self, key: &str) -> anyhow::Result<()> {
        self.objects.lock().await.remove(key);
        Ok(())
    }

    fn public_url(&self, key: &str) -> String {
        format!("https://uploads.test/{key}")
    }
}

#[tokio::test]
async fn upload_policy_storage_failures_and_ownership() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    let Ok(database_url) = std::env::var("DATABASE_URL") else {
        return Ok(());
    };
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&database_url)
        .await?;
    sqlx::migrate!("./migrations").run(&pool).await?;

    let owner_id = insert_user(&pool).await?;
    let other_id = insert_user(&pool).await?;
    let storage = Arc::new(MemoryStorage::default());
    let service = UploadService::new(UploadRepository::new(pool.clone()), storage.clone());

    let upload = service
        .create(
            owner_id,
            UploadInput {
                original_filename: "../../photo.png".into(),
                claimed_mime_type: Some("image/png".into()),
                category: UploadCategory::TopicImage,
                data: test_png()?,
            },
        )
        .await?;
    assert_eq!(upload.original_filename, "photo.png");
    assert_eq!(upload.mime_type, "image/png");
    assert_eq!((upload.width, upload.height), (Some(32), Some(20)));
    assert!(upload.thumbnail_url.is_some());
    assert_eq!(storage.objects.lock().await.len(), 2);

    let avatar = service
        .create_avatar(
            owner_id,
            UploadInput {
                original_filename: "avatar.png".into(),
                claimed_mime_type: Some("image/png".into()),
                category: UploadCategory::Avatar,
                data: test_png()?,
            },
        )
        .await?;
    let avatar_url =
        sqlx::query_scalar::<_, Option<String>>("SELECT avatar_url FROM users WHERE id = $1")
            .bind(owner_id)
            .fetch_one(&pool)
            .await?;
    assert_eq!(avatar_url.as_deref(), Some(avatar.url.as_str()));
    service.delete_avatar(owner_id).await?;
    let avatar_url =
        sqlx::query_scalar::<_, Option<String>>("SELECT avatar_url FROM users WHERE id = $1")
            .bind(owner_id)
            .fetch_one(&pool)
            .await?;
    assert!(avatar_url.is_none());
    assert_eq!(storage.objects.lock().await.len(), 2);

    let illegal = service
        .create(
            owner_id,
            UploadInput {
                original_filename: "attack.jpg".into(),
                claimed_mime_type: Some("image/jpeg".into()),
                category: UploadCategory::TopicImage,
                data: Bytes::from_static(b"<!doctype html><script>alert(1)</script>"),
            },
        )
        .await;
    assert!(matches!(illegal, Err(UploadError::UnsupportedMediaType)));

    let oversized = service
        .create(
            owner_id,
            UploadInput {
                original_filename: "large.png".into(),
                claimed_mime_type: Some("image/png".into()),
                category: UploadCategory::Avatar,
                data: Bytes::from(vec![0; 5 * 1024 * 1024 + 1]),
            },
        )
        .await;
    assert!(matches!(oversized, Err(UploadError::TooLarge)));

    *storage.fail_put.lock().await = true;
    let storage_failure = service
        .create(
            owner_id,
            UploadInput {
                original_filename: "notes.txt".into(),
                claimed_mime_type: Some("text/plain".into()),
                category: UploadCategory::Attachment,
                data: Bytes::from_static(b"safe attachment"),
            },
        )
        .await;
    assert!(matches!(
        storage_failure,
        Err(UploadError::StorageUnavailable)
    ));
    *storage.fail_put.lock().await = false;

    let unauthorized_delete = service.delete(other_id, upload.id).await;
    assert!(matches!(unauthorized_delete, Err(UploadError::NotFound)));
    service.delete(owner_id, upload.id).await?;
    assert!(storage.objects.lock().await.is_empty());

    sqlx::query("DELETE FROM users WHERE id = $1 OR id = $2")
        .bind(owner_id)
        .bind(other_id)
        .execute(&pool)
        .await?;
    Ok(())
}

async fn insert_user(pool: &sqlx::PgPool) -> anyhow::Result<Uuid> {
    let id = Uuid::new_v4();
    let short = id.simple().to_string();
    sqlx::query(
        r#"
        INSERT INTO users (id, username, email, password_hash, role_id)
        SELECT $1, $2, $3, '$argon2id$test', roles.id
        FROM roles WHERE roles.code = 'user'
        "#,
    )
    .bind(id)
    .bind(format!("test_{}", &short[..20]))
    .bind(format!("{short}@uploads.test"))
    .execute(pool)
    .await?;
    Ok(id)
}

fn test_png() -> anyhow::Result<Bytes> {
    let image = DynamicImage::new_rgb8(32, 20);
    let mut output = Cursor::new(Vec::new());
    image.write_to(&mut output, ImageFormat::Png)?;
    Ok(Bytes::from(output.into_inner()))
}
