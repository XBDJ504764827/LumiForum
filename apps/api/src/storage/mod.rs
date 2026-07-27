mod local;
mod s3;

use async_trait::async_trait;
use bytes::Bytes;

pub use local::LocalStorage;
pub use s3::{S3Storage, S3StorageConfig};

#[derive(Clone, Copy, Debug)]
pub struct PutOptions<'a> {
    pub content_type: &'a str,
    pub cache_control: &'a str,
}

#[async_trait]
pub trait StorageProvider: Send + Sync {
    fn name(&self) -> &'static str;

    async fn put(&self, key: &str, data: Bytes, options: PutOptions<'_>) -> anyhow::Result<()>;

    async fn delete(&self, key: &str) -> anyhow::Result<()>;

    fn public_url(&self, key: &str) -> String;
}
