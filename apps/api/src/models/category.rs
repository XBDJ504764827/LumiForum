use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::PatchField;

#[derive(sqlx::FromRow)]
pub struct CategoryRecord {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub sort_order: i32,
    pub is_visible: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CategorySummary {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    pub icon: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CategoryResponse {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub sort_order: i32,
    pub is_visible: bool,
    pub topic_count: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Deserialize)]
pub struct CreateCategoryRequest {
    pub slug: Option<String>,
    pub name: String,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub sort_order: Option<i32>,
    pub is_visible: Option<bool>,
}

#[derive(Default, Deserialize)]
pub struct UpdateCategoryRequest {
    pub slug: Option<String>,
    pub name: Option<String>,
    #[serde(default)]
    pub description: PatchField<String>,
    #[serde(default)]
    pub icon: PatchField<String>,
    pub sort_order: Option<i32>,
    pub is_visible: Option<bool>,
}
