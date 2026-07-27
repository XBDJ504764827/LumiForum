use std::path::{Component, Path, PathBuf};

use async_trait::async_trait;
use bytes::Bytes;
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

use super::{PutOptions, StorageProvider};

pub struct LocalStorage {
    root: PathBuf,
    public_url: String,
}

impl LocalStorage {
    pub async fn new(root: impl Into<PathBuf>, public_url: String) -> anyhow::Result<Self> {
        let root = root.into();
        tokio::fs::create_dir_all(&root).await?;
        let root = tokio::fs::canonicalize(root).await?;
        Ok(Self {
            root,
            public_url: public_url.trim_end_matches('/').to_owned(),
        })
    }

    fn object_path(&self, key: &str) -> anyhow::Result<PathBuf> {
        let key_path = Path::new(key);
        if key.is_empty()
            || key.contains('\\')
            || key_path.is_absolute()
            || !key_path
                .components()
                .all(|component| matches!(component, Component::Normal(_)))
        {
            anyhow::bail!("invalid storage key");
        }
        Ok(self.root.join(key_path))
    }
}

#[async_trait]
impl StorageProvider for LocalStorage {
    fn name(&self) -> &'static str {
        "local"
    }

    async fn put(&self, key: &str, data: Bytes, _options: PutOptions<'_>) -> anyhow::Result<()> {
        let destination = self.object_path(key)?;
        let parent = destination
            .parent()
            .ok_or_else(|| anyhow::anyhow!("storage key has no parent"))?;
        tokio::fs::create_dir_all(parent).await?;

        let temporary = parent.join(format!(".upload-{}.tmp", Uuid::new_v4()));
        let result = async {
            let mut file = tokio::fs::File::create(&temporary).await?;
            file.write_all(&data).await?;
            file.sync_all().await?;
            tokio::fs::rename(&temporary, &destination).await?;
            anyhow::Ok(())
        }
        .await;
        if result.is_err() {
            let _ = tokio::fs::remove_file(&temporary).await;
        }
        result
    }

    async fn delete(&self, key: &str) -> anyhow::Result<()> {
        let path = self.object_path(key)?;
        match tokio::fs::remove_file(path).await {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error.into()),
        }
    }

    fn public_url(&self, key: &str) -> String {
        format!("{}/{key}", self.public_url)
    }
}

#[cfg(test)]
mod tests {
    use super::LocalStorage;

    #[tokio::test]
    async fn rejects_path_traversal() {
        let root = std::env::temp_dir().join(format!("lumiforum-storage-{}", uuid::Uuid::new_v4()));
        let storage = LocalStorage::new(&root, "http://localhost/storage".into())
            .await
            .unwrap();

        assert!(storage.object_path("../secret").is_err());
        assert!(storage.object_path("/absolute").is_err());
        assert!(storage.object_path("safe/object.jpg").is_ok());

        tokio::fs::remove_dir_all(root).await.unwrap();
    }
}
