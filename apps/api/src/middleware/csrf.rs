use axum::{
    extract::{Request, State},
    http::{header::ORIGIN, Method},
    middleware::Next,
    response::Response,
};

use crate::error::{AppError, AppResult};

#[derive(Clone)]
pub struct CsrfLayer {
    allowed_origin: String,
}

impl CsrfLayer {
    pub fn new(allowed_origin: String) -> Self {
        Self { allowed_origin }
    }
}

pub async fn enforce_origin(
    State(layer): State<CsrfLayer>,
    request: Request,
    next: Next,
) -> AppResult<Response> {
    if !origin_matches(&request, &layer.allowed_origin) {
        return Err(AppError::CsrfValidationFailed);
    }
    Ok(next.run(request).await)
}

/// Require an exact Origin only for state-changing methods. This protects
/// bearer-token admin APIs while keeping GET requests usable by server-side clients.
pub async fn enforce_mutation_origin(
    State(layer): State<CsrfLayer>,
    request: Request,
    next: Next,
) -> AppResult<Response> {
    if !matches!(request.method(), &Method::GET | &Method::HEAD | &Method::OPTIONS)
        && !origin_matches(&request, &layer.allowed_origin)
    {
        return Err(AppError::CsrfValidationFailed);
    }
    Ok(next.run(request).await)
}

fn origin_matches(request: &Request, allowed_origin: &str) -> bool {
    request
        .headers()
        .get(ORIGIN)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|origin| origin == allowed_origin)
}

#[cfg(test)]
mod tests {
    use axum::{body::Body, http::Request};

    use super::origin_matches;

    #[test]
    fn requires_an_exact_origin_match() {
        let request = Request::builder()
            .header("origin", "http://192.168.0.138:3000")
            .body(Body::empty())
            .unwrap();

        assert!(origin_matches(&request, "http://192.168.0.138:3000"));
        assert!(!origin_matches(&request, "https://forum.example.com"));
    }

    #[test]
    fn rejects_a_missing_origin() {
        let request = Request::new(Body::empty());
        assert!(!origin_matches(&request, "http://192.168.0.138:3000"));
    }
}
