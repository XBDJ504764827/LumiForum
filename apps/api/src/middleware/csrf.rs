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
    // A missing Origin means a non-browser client (curl, server-side script);
    // browsers always send Origin on cross-site requests. Requiring an exact
    // match only when the header is present keeps JSON API access usable.
    if request
        .headers()
        .get(ORIGIN)
        .is_some_and(|_| !origin_matches(&request, &layer.allowed_origin))
    {
        return Err(AppError::CsrfValidationFailed);
    }
    Ok(next.run(request).await)
}

/// Require an exact Origin only for state-changing methods when the Origin
/// header is present. This protects bearer-token admin APIs while keeping
/// GET requests and non-browser clients usable.
pub async fn enforce_mutation_origin(
    State(layer): State<CsrfLayer>,
    request: Request,
    next: Next,
) -> AppResult<Response> {
    let is_mutation = !matches!(
        request.method(),
        &Method::GET | &Method::HEAD | &Method::OPTIONS
    );
    if is_mutation
        && request
            .headers()
            .get(ORIGIN)
            .is_some_and(|_| !origin_matches(&request, &layer.allowed_origin))
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
    fn missing_origin_is_treated_as_non_browser_client() {
        // Non-browser clients (curl, server-side scripts) do not send Origin;
        // browsers always send it on cross-site requests.
        let request = Request::new(Body::empty());
        assert!(!origin_matches(&request, "http://192.168.0.138:3000"));
    }
}
