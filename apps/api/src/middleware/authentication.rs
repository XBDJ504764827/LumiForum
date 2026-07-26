use axum::{
    extract::{Request, State},
    http::header::AUTHORIZATION,
    middleware::Next,
    response::Response,
};

use crate::error::{AppError, AppResult};
use crate::state::AppState;

#[derive(Clone)]
pub struct AuthorizationLayer {
    state: AppState,
    permission: &'static str,
}

impl AuthorizationLayer {
    pub fn new(state: AppState, permission: &'static str) -> Self {
        Self { state, permission }
    }
}

pub async fn require_permission(
    State(layer): State<AuthorizationLayer>,
    mut request: Request,
    next: Next,
) -> AppResult<Response> {
    let token = bearer_token(&request).ok_or(AppError::Unauthorized)?;
    let claims = layer
        .state
        .auth()
        .token_service()
        .decode_access_token(token)
        .map_err(|_| AppError::Unauthorized)?;
    let principal = layer.state.authorization().authenticate(claims).await?;
    if !principal.has_permission(layer.permission) {
        return Err(AppError::Forbidden);
    }

    request.extensions_mut().insert(principal);
    Ok(next.run(request).await)
}

fn bearer_token(request: &Request) -> Option<&str> {
    let value = request.headers().get(AUTHORIZATION)?.to_str().ok()?;
    let (scheme, token) = value.split_once(' ')?;
    if scheme.eq_ignore_ascii_case("Bearer") && !token.is_empty() && !token.contains(' ') {
        Some(token)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use axum::{body::Body, http::Request};

    use super::bearer_token;

    #[test]
    fn accepts_only_well_formed_bearer_tokens() {
        let request = Request::builder()
            .header("authorization", "Bearer token-value")
            .body(Body::empty())
            .unwrap();
        assert_eq!(bearer_token(&request), Some("token-value"));

        let malformed = Request::builder()
            .header("authorization", "Basic value")
            .body(Body::empty())
            .unwrap();
        assert_eq!(bearer_token(&malformed), None);
    }
}
