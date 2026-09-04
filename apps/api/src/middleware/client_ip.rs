//! Resilient client-IP resolution behind a trusted reverse proxy.
//!
//! The API is intentionally deployed behind 1Panel/nginx/Caddy (see
//! `docs/deployment/architecture.md`). Without this middleware every request
//! looks like it comes from the proxy's loopback address, which collapses
//! per-IP rate limits (search, WebSocket, auth) into a single global bucket
//! and poisons audit/refresh-token `created_by_ip` values.
//!
//! When `TRUST_PROXY=true`, the first entry of `X-Forwarded-For` (the client
//! address added by the edge proxy) replaces the TCP peer in the request
//! extensions. Handlers that extract `ConnectInfo<SocketAddr>` therefore
//! transparently see the real client IP with no per-route changes.

use std::net::{IpAddr, SocketAddr};

use axum::{
    extract::{ConnectInfo, Request, State},
    middleware::Next,
    response::Response,
};

/// `X-Forwarded-For` has no constant in the `http` crate; nginx/Caddy emit
/// it lowercase, and header lookup is case-insensitive regardless.
const X_FORWARDED_FOR: &str = "x-forwarded-for";

/// Extract the first (leftmost, client-originated) IP from an
/// `X-Forwarded-For` header. Only called when `TRUST_PROXY` is enabled, so
/// the proxy is expected to have sanitised the header.
///
/// Returns `None` on malformed input, letting the caller fall back to the
/// TCP peer address.
fn forwarded_client_ip(headers: &axum::http::HeaderMap) -> Option<IpAddr> {
    let value = headers.get(X_FORWARDED_FOR)?.to_str().ok()?;
    let first = value.split(',').next()?.trim();
    first.parse::<IpAddr>().ok()
}

/// Rewrite the request's `ConnectInfo<SocketAddr>` so downstream handlers see
/// the real client IP instead of the reverse proxy's address.
///
/// - Uses the TCP peer when `TRUST_PROXY` is off.
/// - Falls back to the TCP peer when the header is missing or malformed.
pub async fn resolve_client_ip(
    State(trust_proxy): State<bool>,
    mut request: Request,
    next: Next,
) -> Response {
    let peer = request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|info| info.0.ip())
        .unwrap_or_else(|| IpAddr::from([127, 0, 0, 1]));

    let ip = if trust_proxy {
        forwarded_client_ip(request.headers()).unwrap_or(peer)
    } else {
        peer
    };

    // Replace the extension so `ConnectInfo<SocketAddr>` extractors observe the
    // resolved address.
    match request
        .extensions_mut()
        .get_mut::<ConnectInfo<SocketAddr>>()
    {
        Some(info) => info.0.set_ip(ip),
        None => {
            request
                .extensions_mut()
                .insert(ConnectInfo(SocketAddr::new(ip, 0)));
        }
    }

    next.run(request).await
}

#[cfg(test)]
mod tests {
    use axum::http::HeaderValue;

    use super::forwarded_client_ip;

    fn headers(pairs: &[(&'static str, &'static str)]) -> axum::http::HeaderMap {
        let mut map = axum::http::HeaderMap::new();
        for (name, value) in pairs {
            map.insert(*name, HeaderValue::from_static(value));
        }
        map
    }

    #[test]
    fn parses_first_forwarded_ip() {
        let map = headers(&[("x-forwarded-for", "203.0.113.7, 10.0.0.1")]);
        assert_eq!(
            forwarded_client_ip(&map),
            Some("203.0.113.7".parse().unwrap())
        );
    }

    #[test]
    fn handles_missing_or_malformed_header() {
        assert_eq!(forwarded_client_ip(&headers(&[])), None);
        assert_eq!(
            forwarded_client_ip(&headers(&[("x-forwarded-for", "not-an-ip")])),
            None
        );
        let bogus: axum::http::HeaderMap = headers(&[("x-forwarded-for", "")]);
        assert_eq!(forwarded_client_ip(&bogus), None);
    }
}
