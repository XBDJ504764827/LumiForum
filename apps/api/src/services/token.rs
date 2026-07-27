use std::collections::HashSet;

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use rand::{rngs::OsRng, RngCore};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::models::AccessTokenClaims;

#[derive(Clone)]
pub struct TokenService {
    secret: Vec<u8>,
    issuer: String,
    audience: String,
    access_token_ttl: Duration,
}

impl TokenService {
    pub fn new(
        secret: String,
        issuer: String,
        audience: String,
        access_token_ttl_seconds: i64,
    ) -> anyhow::Result<Self> {
        if secret.len() < 32 {
            anyhow::bail!("JWT secret must be at least 32 bytes");
        }
        if issuer.trim().is_empty() || audience.trim().is_empty() {
            anyhow::bail!("JWT issuer and audience must not be empty");
        }
        if !(60..=3_600).contains(&access_token_ttl_seconds) {
            anyhow::bail!("access token TTL must be between 60 and 3600 seconds");
        }

        Ok(Self {
            secret: secret.into_bytes(),
            issuer,
            audience,
            access_token_ttl: Duration::seconds(access_token_ttl_seconds),
        })
    }

    pub fn issue_access_token(
        &self,
        user_id: Uuid,
        role: &str,
        auth_version: i32,
    ) -> anyhow::Result<(String, i64)> {
        let now = Utc::now();
        let expires_at = now + self.access_token_ttl;
        let claims = AccessTokenClaims {
            sub: user_id,
            role: role.to_owned(),
            auth_version,
            jti: Uuid::new_v4(),
            iss: self.issuer.clone(),
            aud: self.audience.clone(),
            iat: now.timestamp(),
            nbf: now.timestamp(),
            exp: expires_at.timestamp(),
        };

        let token = encode(
            &Header::new(Algorithm::HS256),
            &claims,
            &EncodingKey::from_secret(&self.secret),
        )?;
        Ok((token, self.access_token_ttl.num_seconds()))
    }

    pub fn decode_access_token(&self, token: &str) -> anyhow::Result<AccessTokenClaims> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.validate_nbf = true;
        validation.leeway = 5;
        validation.set_issuer(&[self.issuer.as_str()]);
        validation.set_audience(&[self.audience.as_str()]);
        validation.required_spec_claims = HashSet::from([
            "exp".to_owned(),
            "iat".to_owned(),
            "nbf".to_owned(),
            "sub".to_owned(),
            "jti".to_owned(),
            "iss".to_owned(),
            "aud".to_owned(),
        ]);

        Ok(decode::<AccessTokenClaims>(
            token,
            &DecodingKey::from_secret(&self.secret),
            &validation,
        )?
        .claims)
    }

    pub fn generate_refresh_token() -> String {
        let mut bytes = [0_u8; 32];
        OsRng.fill_bytes(&mut bytes);
        URL_SAFE_NO_PAD.encode(bytes)
    }

    pub fn hash_refresh_token(token: &str) -> [u8; 32] {
        Sha256::digest(token.as_bytes()).into()
    }
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use super::TokenService;

    fn service() -> TokenService {
        TokenService::new(
            "test-secret-with-at-least-thirty-two-bytes".into(),
            "lumiforum-api".into(),
            "lumiforum-web".into(),
            900,
        )
        .expect("valid token service")
    }

    #[test]
    fn issues_and_validates_access_tokens() {
        let service = service();
        let user_id = Uuid::new_v4();
        let (token, expires_in) = service
            .issue_access_token(user_id, "user", 4)
            .expect("token issuance succeeds");
        let claims = service
            .decode_access_token(&token)
            .expect("token validates");

        assert_eq!(expires_in, 900);
        assert_eq!(claims.sub, user_id);
        assert_eq!(claims.role, "user");
        assert_eq!(claims.auth_version, 4);
    }

    #[test]
    fn creates_distinct_refresh_tokens_and_digests() {
        let first = TokenService::generate_refresh_token();
        let second = TokenService::generate_refresh_token();

        assert_ne!(first, second);
        assert_ne!(
            TokenService::hash_refresh_token(&first),
            TokenService::hash_refresh_token(&second)
        );
        assert_eq!(TokenService::hash_refresh_token(&first).len(), 32);
    }
}
