use axum::{
    extract::{rejection::QueryRejection, ConnectInfo, State},
    routing::get,
    Json, Router,
};
use std::net::SocketAddr;

use crate::error::AppResult;
use crate::models::{HotKeywordsResponse, SearchQuery, SearchResponse, SearchSuggestionsResponse};
use crate::state::AppState;

use super::response::{parse_query, ApiResponse};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/search", get(search))
        .route("/search/suggestions", get(suggestions))
        .route("/search/hot", get(hot_keywords))
}

async fn search(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    query: Result<axum::extract::Query<SearchQuery>, QueryRejection>,
) -> AppResult<Json<ApiResponse<SearchResponse>>> {
    let query = parse_query(query)?;
    let client_key = peer.ip().to_string();
    let result = state.search().search(query, &client_key).await?;
    Ok(Json(ApiResponse::new(result)))
}

async fn suggestions(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    query: Result<axum::extract::Query<SuggestionQuery>, QueryRejection>,
) -> AppResult<Json<ApiResponse<SearchSuggestionsResponse>>> {
    let query = parse_query(query)?;
    let client_key = peer.ip().to_string();
    let result = state.search().suggestions(query.q, &client_key).await?;
    Ok(Json(ApiResponse::new(result)))
}

async fn hot_keywords(
    State(state): State<AppState>,
) -> AppResult<Json<ApiResponse<HotKeywordsResponse>>> {
    let result = state.search().hot_keywords().await?;
    Ok(Json(ApiResponse::new(result)))
}

#[derive(Debug, serde::Deserialize)]
struct SuggestionQuery {
    q: Option<String>,
}
