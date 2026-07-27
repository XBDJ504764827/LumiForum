use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{CategorySummary, RoleSummary};

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchType {
    #[default]
    Topic,
    Comment,
    User,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchSort {
    #[default]
    Relevance,
    Latest,
    Hot,
}

#[derive(Default, Deserialize)]
pub struct SearchQuery {
    /// Primary keyword. Alias `keyword` is accepted for clients that prefer it.
    pub q: Option<String>,
    pub keyword: Option<String>,
    #[serde(rename = "type")]
    pub search_type: Option<SearchType>,
    pub category_id: Option<Uuid>,
    pub author_id: Option<Uuid>,
    pub sort: Option<SearchSort>,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
    /// Alias for page_size (spec uses limit).
    pub limit: Option<u32>,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Serialize)]
pub struct SearchAuthor {
    pub id: Uuid,
    pub username: String,
    pub nickname: Option<String>,
    pub avatar: Option<String>,
    pub role: RoleSummary,
}

#[derive(Clone, Debug, Serialize)]
pub struct TopicSearchHit {
    pub id: Uuid,
    pub title: String,
    pub slug: String,
    pub summary: Option<String>,
    pub highlight: String,
    pub category: CategorySummary,
    pub author: SearchAuthor,
    pub stats: SearchTopicStats,
    pub created_at: DateTime<Utc>,
    pub rank: f32,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct SearchTopicStats {
    pub views: i64,
    pub replies: i64,
    pub likes: i64,
}

#[derive(Clone, Debug, Serialize)]
pub struct CommentSearchHit {
    pub id: Uuid,
    pub topic_id: Uuid,
    pub topic_slug: String,
    pub topic_title: String,
    pub content_preview: String,
    pub highlight: String,
    pub author: SearchAuthor,
    pub like_count: i64,
    pub created_at: DateTime<Utc>,
    pub rank: f32,
}

#[derive(Clone, Debug, Serialize)]
pub struct UserSearchHit {
    pub id: Uuid,
    pub username: String,
    pub nickname: Option<String>,
    pub avatar: Option<String>,
    pub role: RoleSummary,
    pub followers_count: i64,
    pub following_count: i64,
    pub highlight: String,
    pub created_at: DateTime<Utc>,
    pub rank: f32,
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SearchHit {
    Topic(TopicSearchHit),
    Comment(CommentSearchHit),
    User(UserSearchHit),
}

#[derive(Clone, Debug, Serialize)]
pub struct SearchResponse {
    pub query: String,
    #[serde(rename = "type")]
    pub search_type: SearchType,
    pub sort: SearchSort,
    pub items: Vec<SearchHit>,
    pub pagination: super::PaginationMeta,
    /// Reserved for swapping in Elasticsearch/Meilisearch later.
    pub engine: &'static str,
}

#[derive(Clone, Debug, Serialize)]
pub struct SearchSuggestionsResponse {
    pub query: String,
    pub suggestions: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct HotKeywordsResponse {
    pub keywords: Vec<HotKeyword>,
}

#[derive(Clone, Debug, Serialize)]
pub struct HotKeyword {
    pub keyword: String,
    pub score: i64,
}

#[cfg(test)]
mod tests {
    use super::{SearchSort, SearchType};

    #[test]
    fn defaults() {
        assert_eq!(SearchType::default(), SearchType::Topic);
        assert_eq!(SearchSort::default(), SearchSort::Relevance);
    }
}
