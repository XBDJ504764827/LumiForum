use redis::{aio::ConnectionManager, AsyncCommands};
use thiserror::Error;

use crate::models::{
    HotKeyword, HotKeywordsResponse, PaginationMeta, SearchHit, SearchQuery, SearchResponse,
    SearchSuggestionsResponse, SearchType,
};
use crate::repositories::{
    CommentSearchFilter, SearchRepository, TopicSearchFilter, UserSearchFilter,
};

const DEFAULT_PAGE_SIZE: u32 = 20;
const MAX_PAGE_SIZE: u32 = 50;
const MAX_PAGE: u32 = 1_000_000;
const MIN_QUERY_LEN: usize = 1;
const MAX_QUERY_LEN: usize = 100;
const RATE_LIMIT_WINDOW_SECS: u64 = 60;
const RATE_LIMIT_MAX: u64 = 30;
const HOT_KEYWORDS_KEY: &str = "search:hot:keywords";
const HOT_KEYWORDS_LIMIT: isize = 20;

#[derive(Clone)]
pub struct SearchService {
    search: SearchRepository,
    redis: ConnectionManager,
}

#[derive(Debug, Error)]
pub enum SearchError {
    #[error("invalid search input: {0}")]
    Validation(&'static str),
    #[error("rate limit exceeded")]
    RateLimited,
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

impl SearchService {
    pub fn new(search: SearchRepository, redis: ConnectionManager) -> Self {
        Self { search, redis }
    }

    pub async fn search(
        &self,
        query: SearchQuery,
        client_key: &str,
    ) -> Result<SearchResponse, SearchError> {
        self.enforce_rate_limit(client_key).await?;
        let keyword = normalize_query(query.q.or(query.keyword))?;
        let search_type = query.search_type.unwrap_or_default();
        let sort = query.sort.unwrap_or_default();
        let page = query.page.unwrap_or(1);
        let page_size = query.page_size.or(query.limit).unwrap_or(DEFAULT_PAGE_SIZE);
        validate_pagination(page, page_size)?;
        if let (Some(from), Some(to)) = (query.from, query.to) {
            if from > to {
                return Err(SearchError::Validation("from must be before to"));
            }
        }

        let offset = i64::from(page - 1) * i64::from(page_size);

        let (items, total) = match search_type {
            SearchType::Topic => {
                let (rows, total) = self
                    .search
                    .search_topics(TopicSearchFilter {
                        keyword: &keyword,
                        category_id: query.category_id,
                        author_id: query.author_id,
                        from: query.from,
                        to: query.to,
                        has_poll: query.has_poll,
                        sort,
                        limit: i64::from(page_size),
                        offset,
                    })
                    .await
                    .map_err(internal)?;
                (
                    rows.into_iter().map(SearchHit::Topic).collect::<Vec<_>>(),
                    total,
                )
            }
            SearchType::Comment => {
                let (rows, total) = self
                    .search
                    .search_comments(CommentSearchFilter {
                        keyword: &keyword,
                        author_id: query.author_id,
                        from: query.from,
                        to: query.to,
                        sort,
                        limit: i64::from(page_size),
                        offset,
                    })
                    .await
                    .map_err(internal)?;
                (
                    rows.into_iter().map(SearchHit::Comment).collect::<Vec<_>>(),
                    total,
                )
            }
            SearchType::User => {
                let (rows, total) = self
                    .search
                    .search_users(UserSearchFilter {
                        keyword: &keyword,
                        sort,
                        limit: i64::from(page_size),
                        offset,
                    })
                    .await
                    .map_err(internal)?;
                (
                    rows.into_iter().map(SearchHit::User).collect::<Vec<_>>(),
                    total,
                )
            }
        };

        let total =
            u64::try_from(total).map_err(|_| internal(anyhow::anyhow!("negative search count")))?;
        let response = SearchResponse {
            query: keyword.clone(),
            search_type,
            sort,
            items,
            pagination: PaginationMeta::new(page, page_size, total),
            engine: "postgres_fts",
        };
        self.record_hot_keyword(&keyword).await;
        Ok(response)
    }

    pub async fn suggestions(
        &self,
        q: Option<String>,
        client_key: &str,
    ) -> Result<SearchSuggestionsResponse, SearchError> {
        self.enforce_rate_limit(client_key).await?;
        let keyword = normalize_query(q)?;
        let suggestions = self
            .search
            .suggest_topics(&keyword, 8)
            .await
            .map_err(internal)?;
        Ok(SearchSuggestionsResponse {
            query: keyword,
            suggestions,
        })
    }

    pub async fn hot_keywords(&self) -> Result<HotKeywordsResponse, SearchError> {
        let mut redis = self.redis.clone();
        let rows: Vec<(String, f64)> = match redis
            .zrevrange_withscores(HOT_KEYWORDS_KEY, 0, HOT_KEYWORDS_LIMIT - 1)
            .await
        {
            Ok(value) => value,
            Err(error) => {
                tracing::warn!(%error, "hot keywords cache unavailable");
                Vec::new()
            }
        };
        Ok(HotKeywordsResponse {
            keywords: rows
                .into_iter()
                .map(|(keyword, score)| HotKeyword {
                    keyword,
                    score: score as i64,
                })
                .collect(),
        })
    }

    async fn enforce_rate_limit(&self, client_key: &str) -> Result<(), SearchError> {
        let key = format!("rate:search:{client_key}");
        let mut redis = self.redis.clone();
        match redis.incr::<_, u64, u64>(&key, 1_u64).await {
            Ok(count) => {
                if count == 1 {
                    let _: Result<(), _> = redis.expire(&key, RATE_LIMIT_WINDOW_SECS as i64).await;
                }
                if count > RATE_LIMIT_MAX {
                    Err(SearchError::RateLimited)
                } else {
                    Ok(())
                }
            }
            Err(error) => {
                tracing::warn!(%error, "search rate limit unavailable; allowing request");
                Ok(())
            }
        }
    }

    async fn record_hot_keyword(&self, keyword: &str) {
        let mut redis = self.redis.clone();
        if let Err(error) = redis
            .zincr::<_, _, _, ()>(HOT_KEYWORDS_KEY, keyword, 1)
            .await
        {
            tracing::warn!(%error, "failed to record hot keyword");
            return;
        }
        let _: Result<(), _> = redis.zremrangebyrank(HOT_KEYWORDS_KEY, 0, -51).await;
        let _: Result<(), _> = redis.expire(HOT_KEYWORDS_KEY, 7 * 24 * 3600).await;
    }
}

fn normalize_query(value: Option<String>) -> Result<String, SearchError> {
    let value = value.unwrap_or_default();
    let value = value.trim();
    if value.is_empty() {
        return Err(SearchError::Validation("query must not be empty"));
    }
    let chars: Vec<char> = value.chars().collect();
    if chars.len() < MIN_QUERY_LEN || chars.len() > MAX_QUERY_LEN {
        return Err(SearchError::Validation(
            "query must contain between 1 and 100 characters",
        ));
    }
    // Strip control chars and neutralize LIKE metacharacters after binding (still escape).
    let cleaned: String = chars
        .into_iter()
        .filter(|ch| !ch.is_control())
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if cleaned.is_empty() {
        return Err(SearchError::Validation("query must not be empty"));
    }
    Ok(escape_like_meta(&cleaned))
}

fn escape_like_meta(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

fn validate_pagination(page: u32, page_size: u32) -> Result<(), SearchError> {
    if page == 0 || page > MAX_PAGE {
        return Err(SearchError::Validation("page is out of range"));
    }
    if page_size == 0 || page_size > MAX_PAGE_SIZE {
        return Err(SearchError::Validation(
            "page size must be between 1 and 50",
        ));
    }
    Ok(())
}

fn internal(error: impl Into<anyhow::Error>) -> SearchError {
    SearchError::Internal(error.into())
}

#[cfg(test)]
mod tests {
    use super::{escape_like_meta, normalize_query};

    #[test]
    fn rejects_empty_query() {
        assert!(normalize_query(Some("   ".into())).is_err());
        assert!(normalize_query(None).is_err());
    }

    #[test]
    fn escapes_like_metacharacters() {
        assert_eq!(escape_like_meta("100%_off"), r"100\%\_off");
    }

    #[test]
    fn normalizes_whitespace() {
        let value = normalize_query(Some("  rust   axum  ".into())).unwrap();
        assert_eq!(value, "rust axum");
    }
}
