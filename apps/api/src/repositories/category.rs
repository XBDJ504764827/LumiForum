use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::CategoryResponse;

#[derive(Clone)]
pub struct CategoryRepository {
    pool: PgPool,
}

#[derive(sqlx::FromRow)]
pub struct RepositoryCategory {
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

pub struct NewCategory<'a> {
    pub slug: &'a str,
    pub name: &'a str,
    pub description: Option<&'a str>,
    pub icon: Option<&'a str>,
    pub sort_order: i32,
    pub is_visible: bool,
}

pub struct CategoryUpdate<'a> {
    pub slug: Option<&'a str>,
    pub name: Option<&'a str>,
    pub description_changed: bool,
    pub description: Option<&'a str>,
    pub icon_changed: bool,
    pub icon: Option<&'a str>,
    pub sort_order: Option<i32>,
    pub is_visible: Option<bool>,
}

impl CategoryRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn list(&self, include_hidden: bool) -> Result<Vec<RepositoryCategory>, sqlx::Error> {
        sqlx::query_as::<_, RepositoryCategory>(CATEGORY_SELECT)
            .bind(include_hidden)
            .fetch_all(&self.pool)
            .await
    }

    pub async fn find_by_slug(
        &self,
        slug: &str,
        include_hidden: bool,
    ) -> Result<Option<RepositoryCategory>, sqlx::Error> {
        sqlx::query_as::<_, RepositoryCategory>(CATEGORY_BY_SLUG)
            .bind(slug)
            .bind(include_hidden)
            .fetch_optional(&self.pool)
            .await
    }

    pub async fn find_by_id(
        &self,
        category_id: Uuid,
    ) -> Result<Option<RepositoryCategory>, sqlx::Error> {
        sqlx::query_as::<_, RepositoryCategory>(CATEGORY_BY_ID)
            .bind(category_id)
            .fetch_optional(&self.pool)
            .await
    }

    pub async fn create(
        &self,
        category: NewCategory<'_>,
    ) -> Result<RepositoryCategory, sqlx::Error> {
        let id = sqlx::query_scalar::<_, Uuid>(
            r#"
            INSERT INTO categories (slug, name, description, icon, sort_order, is_visible)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id
            "#,
        )
        .bind(category.slug)
        .bind(category.name)
        .bind(category.description)
        .bind(category.icon)
        .bind(category.sort_order)
        .bind(category.is_visible)
        .fetch_one(&self.pool)
        .await?;

        self.find_by_id(id).await?.ok_or(sqlx::Error::RowNotFound)
    }

    pub async fn update(
        &self,
        category_id: Uuid,
        update: CategoryUpdate<'_>,
    ) -> Result<Option<RepositoryCategory>, sqlx::Error> {
        let id = sqlx::query_scalar::<_, Uuid>(
            r#"
            UPDATE categories
            SET slug = COALESCE($2, slug),
                name = COALESCE($3, name),
                description = CASE WHEN $4 THEN $5 ELSE description END,
                icon = CASE WHEN $6 THEN $7 ELSE icon END,
                sort_order = COALESCE($8, sort_order),
                is_visible = COALESCE($9, is_visible)
            WHERE id = $1
            RETURNING id
            "#,
        )
        .bind(category_id)
        .bind(update.slug)
        .bind(update.name)
        .bind(update.description_changed)
        .bind(update.description)
        .bind(update.icon_changed)
        .bind(update.icon)
        .bind(update.sort_order)
        .bind(update.is_visible)
        .fetch_optional(&self.pool)
        .await?;

        match id {
            Some(id) => self.find_by_id(id).await,
            None => Ok(None),
        }
    }

    pub async fn delete(&self, category_id: Uuid) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("DELETE FROM categories WHERE id = $1")
            .bind(category_id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() == 1)
    }
}

pub fn repository_category_to_response(category: RepositoryCategory) -> CategoryResponse {
    CategoryResponse {
        id: category.id,
        slug: category.slug,
        name: category.name,
        description: category.description,
        icon: category.icon,
        sort_order: category.sort_order,
        is_visible: category.is_visible,
        topic_count: category.topic_count,
        created_at: category.created_at,
        updated_at: category.updated_at,
    }
}

const CATEGORY_SELECT: &str = r#"
    SELECT
        categories.id,
        categories.slug,
        categories.name,
        categories.description,
        categories.icon,
        categories.sort_order,
        categories.is_visible,
        count(topics.id) FILTER (WHERE topics.status = 'published') AS topic_count,
        categories.created_at,
        categories.updated_at
    FROM categories
    LEFT JOIN topics ON topics.category_id = categories.id
    WHERE $1 OR categories.is_visible = true
    GROUP BY categories.id
    ORDER BY categories.sort_order, categories.name, categories.id
"#;

const CATEGORY_BY_SLUG: &str = r#"
    SELECT
        categories.id,
        categories.slug,
        categories.name,
        categories.description,
        categories.icon,
        categories.sort_order,
        categories.is_visible,
        count(topics.id) FILTER (WHERE topics.status = 'published') AS topic_count,
        categories.created_at,
        categories.updated_at
    FROM categories
    LEFT JOIN topics ON topics.category_id = categories.id
    WHERE categories.slug = $1 AND ($2 OR categories.is_visible = true)
    GROUP BY categories.id
"#;

const CATEGORY_BY_ID: &str = r#"
    SELECT
        categories.id,
        categories.slug,
        categories.name,
        categories.description,
        categories.icon,
        categories.sort_order,
        categories.is_visible,
        count(topics.id) FILTER (WHERE topics.status = 'published') AS topic_count,
        categories.created_at,
        categories.updated_at
    FROM categories
    LEFT JOIN topics ON topics.category_id = categories.id
    WHERE categories.id = $1
    GROUP BY categories.id
"#;
