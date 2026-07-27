use std::sync::Arc;

use anyhow::Context;
use argon2::{
    password_hash::{rand_core::OsRng, SaltString},
    Argon2, Params, PasswordHash, PasswordHasher, PasswordVerifier,
};
use tokio::sync::Semaphore;

#[derive(Clone)]
pub struct PasswordService {
    semaphore: Arc<Semaphore>,
}

impl PasswordService {
    pub fn new(max_concurrent_hashes: usize) -> anyhow::Result<Self> {
        if max_concurrent_hashes == 0 {
            anyhow::bail!("password hash concurrency must be positive");
        }

        Ok(Self {
            semaphore: Arc::new(Semaphore::new(max_concurrent_hashes)),
        })
    }

    pub async fn hash(&self, password: String) -> anyhow::Result<String> {
        let permit = self
            .semaphore
            .clone()
            .acquire_owned()
            .await
            .context("password hash semaphore closed")?;

        tokio::task::spawn_blocking(move || {
            let _permit = permit;
            let salt = SaltString::generate(&mut OsRng);
            argon2id()?
                .hash_password(password.as_bytes(), &salt)
                .map(|hash| hash.to_string())
                .map_err(|_| anyhow::anyhow!("failed to hash password"))
        })
        .await
        .context("password hash task failed")?
    }

    pub async fn verify(&self, password: String, password_hash: String) -> anyhow::Result<bool> {
        let permit = self
            .semaphore
            .clone()
            .acquire_owned()
            .await
            .context("password hash semaphore closed")?;

        tokio::task::spawn_blocking(move || {
            let _permit = permit;
            let parsed = PasswordHash::new(&password_hash)
                .map_err(|_| anyhow::anyhow!("invalid password hash"))?;
            Ok(argon2id()?
                .verify_password(password.as_bytes(), &parsed)
                .is_ok())
        })
        .await
        .context("password verification task failed")?
    }

    pub async fn consume_dummy_work(&self, password: String) -> anyhow::Result<()> {
        self.hash(password).await.map(|_| ())
    }
}

fn argon2id<'a>() -> anyhow::Result<Argon2<'a>> {
    let params = Params::new(19_456, 2, 1, None)
        .map_err(|_| anyhow::anyhow!("invalid Argon2 parameters"))?;
    Ok(Argon2::new(
        argon2::Algorithm::Argon2id,
        argon2::Version::V0x13,
        params,
    ))
}

#[cfg(test)]
mod tests {
    use super::PasswordService;

    #[tokio::test]
    async fn hashes_and_verifies_passwords() {
        let service = PasswordService::new(1).expect("valid service");
        let hash = service
            .hash("correct horse battery staple".into())
            .await
            .expect("hash succeeds");

        assert!(hash.starts_with("$argon2id$"));
        assert!(service
            .verify("correct horse battery staple".into(), hash.clone())
            .await
            .expect("verification succeeds"));
        assert!(!service
            .verify("wrong password".into(), hash)
            .await
            .expect("verification succeeds"));
    }
}
