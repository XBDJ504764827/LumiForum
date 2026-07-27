use async_trait::async_trait;
use bytes::Bytes;
use object_store::{
    aws::{AmazonS3, AmazonS3Builder},
    path::Path,
    Attribute, Attributes, ObjectStore, PutOptions as ObjectPutOptions,
};

use super::{PutOptions, StorageProvider};

pub struct S3StorageConfig {
    pub endpoint: Option<String>,
    pub region: String,
    pub bucket: String,
    pub access_key: String,
    pub secret_key: String,
    pub force_path_style: bool,
    pub public_url: String,
}

pub struct S3Storage {
    store: AmazonS3,
    public_url: String,
}

impl S3Storage {
    pub fn new(config: S3StorageConfig) -> anyhow::Result<Self> {
        if config.bucket.trim().is_empty() || config.public_url.trim().is_empty() {
            anyhow::bail!("S3_BUCKET and S3_PUBLIC_URL are required for s3 storage");
        }
        if config.access_key.is_empty() || config.secret_key.is_empty() {
            anyhow::bail!("S3 credentials are required for s3 storage");
        }

        let mut builder = AmazonS3Builder::new()
            .with_bucket_name(config.bucket)
            .with_region(config.region)
            .with_access_key_id(config.access_key)
            .with_secret_access_key(config.secret_key)
            .with_virtual_hosted_style_request(!config.force_path_style);
        if let Some(endpoint) = config.endpoint.filter(|value| !value.is_empty()) {
            if endpoint.starts_with("http://") {
                builder = builder.with_allow_http(true);
            }
            builder = builder.with_endpoint(endpoint);
        }

        Ok(Self {
            store: builder.build()?,
            public_url: config.public_url.trim_end_matches('/').to_owned(),
        })
    }
}

#[async_trait]
impl StorageProvider for S3Storage {
    fn name(&self) -> &'static str {
        "s3"
    }

    async fn put(&self, key: &str, data: Bytes, options: PutOptions<'_>) -> anyhow::Result<()> {
        let mut attributes = Attributes::new();
        attributes.insert(
            Attribute::ContentType,
            options.content_type.to_owned().into(),
        );
        attributes.insert(
            Attribute::CacheControl,
            options.cache_control.to_owned().into(),
        );
        self.store
            .put_opts(
                &Path::parse(key)?,
                data.into(),
                ObjectPutOptions {
                    attributes,
                    ..Default::default()
                },
            )
            .await
            .map_err(|error| anyhow::anyhow!("S3 put failed: {error}"))?;
        Ok(())
    }

    async fn delete(&self, key: &str) -> anyhow::Result<()> {
        self.store
            .delete(&Path::parse(key)?)
            .await
            .map_err(|error| anyhow::anyhow!("S3 delete failed: {error}"))?;
        Ok(())
    }

    fn public_url(&self, key: &str) -> String {
        format!("{}/{key}", self.public_url)
    }
}
