use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{FavoriteItem, RoleSummary, UserPublicSummary};
use crate::repositories::{repository_topic_to_summary, RepositoryTopic};

#[derive(Clone)]
pub struct ReactionRepository {
    pool: PgPool,
}

#[derive(Clone, Copy, Debug, sqlx::FromRow)]
pub struct FollowCounters {
    pub followers_count: i64,
    pub following_count: i64,
}

#[derive(sqlx::FromRow)]
struct RepositoryPublicUser {
    id: Uuid,
    username: String,
    nickname: Option<String>,
    avatar: Option<String>,
    role_code: String,
    role_name: String,
    followers_count: i64,
    following_count: i64,
    is_following: bool,
    created_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
struct FavoriteRow {
    favorited_at: DateTime<Utc>,
    id: Uuid,
    category_id: Uuid,
    author_id: Uuid,
    title: String,
    slug: String,
    content: Option<String>,
    summary: Option<String>,
    status: String,
    view_count: i64,
    reply_count: i64,
    like_count: i64,
    is_pinned: bool,
    is_featured: bool,
    last_reply_at: Option<DateTime<Utc>>,
    deleted_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    category_slug: String,
    category_name: String,
    category_icon: Option<String>,
    category_is_visible: bool,
    author_username: String,
    author_nickname: Option<String>,
    author_avatar: Option<String>,
    author_role_code: String,
    author_role_name: String,
    has_poll: bool,
}

impl ReactionRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn topic_is_published(&self, topic_id: Uuid) -> Result<bool, sqlx::Error> {
        sqlx::query_scalar::<_, bool>(
            r#"
            SELECT EXISTS (
                SELECT 1
                FROM topics t
                JOIN categories c ON c.id = t.category_id
                WHERE t.id = $1
                  AND t.status = 'published'
                  AND c.is_visible = true
            )
            "#,
        )
        .bind(topic_id)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn comment_is_published(&self, comment_id: Uuid) -> Result<bool, sqlx::Error> {
        sqlx::query_scalar::<_, bool>(
            r#"
            SELECT EXISTS (
                SELECT 1
                FROM comments c
                JOIN topics t ON t.id = c.topic_id
                JOIN categories cat ON cat.id = t.category_id
                WHERE c.id = $1
                  AND c.status = 'published'
                  AND t.status = 'published'
                  AND cat.is_visible = true
            )
            "#,
        )
        .bind(comment_id)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn user_is_active(&self, user_id: Uuid) -> Result<bool, sqlx::Error> {
        sqlx::query_scalar::<_, bool>(
            r#"
            SELECT EXISTS (
                SELECT 1 FROM users
                WHERE id = $1 AND status = 'active'
            )
            "#,
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn has_topic_like(&self, user_id: Uuid, topic_id: Uuid) -> Result<bool, sqlx::Error> {
        sqlx::query_scalar::<_, bool>(
            r#"
            SELECT EXISTS (
                SELECT 1 FROM topic_likes
                WHERE user_id = $1 AND topic_id = $2
            )
            "#,
        )
        .bind(user_id)
        .bind(topic_id)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn has_comment_like(
        &self,
        user_id: Uuid,
        comment_id: Uuid,
    ) -> Result<bool, sqlx::Error> {
        sqlx::query_scalar::<_, bool>(
            r#"
            SELECT EXISTS (
                SELECT 1 FROM comment_likes
                WHERE user_id = $1 AND comment_id = $2
            )
            "#,
        )
        .bind(user_id)
        .bind(comment_id)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn has_favorite(&self, user_id: Uuid, topic_id: Uuid) -> Result<bool, sqlx::Error> {
        sqlx::query_scalar::<_, bool>(
            r#"
            SELECT EXISTS (
                SELECT 1 FROM favorites
                WHERE user_id = $1 AND topic_id = $2
            )
            "#,
        )
        .bind(user_id)
        .bind(topic_id)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn has_follow(
        &self,
        follower_id: Uuid,
        following_id: Uuid,
    ) -> Result<bool, sqlx::Error> {
        sqlx::query_scalar::<_, bool>(
            r#"
            SELECT EXISTS (
                SELECT 1 FROM user_follows
                WHERE follower_id = $1 AND following_id = $2
            )
            "#,
        )
        .bind(follower_id)
        .bind(following_id)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn topic_like_count(&self, topic_id: Uuid) -> Result<i64, sqlx::Error> {
        sqlx::query_scalar::<_, i64>("SELECT like_count FROM topics WHERE id = $1")
            .bind(topic_id)
            .fetch_one(&self.pool)
            .await
    }

    pub async fn comment_like_count(&self, comment_id: Uuid) -> Result<i64, sqlx::Error> {
        sqlx::query_scalar::<_, i64>("SELECT like_count FROM comments WHERE id = $1")
            .bind(comment_id)
            .fetch_one(&self.pool)
            .await
    }

    pub async fn like_topic(
        &self,
        user_id: Uuid,
        topic_id: Uuid,
    ) -> Result<(i64, bool), sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        let inserted = sqlx::query(
            r#"
            INSERT INTO topic_likes (topic_id, user_id)
            VALUES ($1, $2)
            ON CONFLICT (topic_id, user_id) DO NOTHING
            "#,
        )
        .bind(topic_id)
        .bind(user_id)
        .execute(&mut *tx)
        .await?
        .rows_affected();

        if inserted == 1 {
            sqlx::query(
                r#"
                UPDATE topics
                SET like_count = like_count + 1
                WHERE id = $1
                "#,
            )
            .bind(topic_id)
            .execute(&mut *tx)
            .await?;
        }

        let like_count =
            sqlx::query_scalar::<_, i64>("SELECT like_count FROM topics WHERE id = $1")
                .bind(topic_id)
                .fetch_one(&mut *tx)
                .await?;
        tx.commit().await?;
        Ok((like_count, inserted == 1))
    }

    pub async fn unlike_topic(&self, user_id: Uuid, topic_id: Uuid) -> Result<i64, sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        let deleted = sqlx::query(
            r#"
            DELETE FROM topic_likes
            WHERE topic_id = $1 AND user_id = $2
            "#,
        )
        .bind(topic_id)
        .bind(user_id)
        .execute(&mut *tx)
        .await?
        .rows_affected();

        if deleted == 1 {
            sqlx::query(
                r#"
                UPDATE topics
                SET like_count = GREATEST(like_count - 1, 0)
                WHERE id = $1
                "#,
            )
            .bind(topic_id)
            .execute(&mut *tx)
            .await?;
        }

        let like_count =
            sqlx::query_scalar::<_, i64>("SELECT like_count FROM topics WHERE id = $1")
                .bind(topic_id)
                .fetch_one(&mut *tx)
                .await?;
        tx.commit().await?;
        Ok(like_count)
    }

    pub async fn like_comment(
        &self,
        user_id: Uuid,
        comment_id: Uuid,
    ) -> Result<(i64, bool), sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        let inserted = sqlx::query(
            r#"
            INSERT INTO comment_likes (comment_id, user_id)
            VALUES ($1, $2)
            ON CONFLICT (comment_id, user_id) DO NOTHING
            "#,
        )
        .bind(comment_id)
        .bind(user_id)
        .execute(&mut *tx)
        .await?
        .rows_affected();

        if inserted == 1 {
            sqlx::query(
                r#"
                UPDATE comments
                SET like_count = like_count + 1
                WHERE id = $1
                "#,
            )
            .bind(comment_id)
            .execute(&mut *tx)
            .await?;
        }

        let like_count =
            sqlx::query_scalar::<_, i64>("SELECT like_count FROM comments WHERE id = $1")
                .bind(comment_id)
                .fetch_one(&mut *tx)
                .await?;
        tx.commit().await?;
        Ok((like_count, inserted == 1))
    }

    pub async fn unlike_comment(
        &self,
        user_id: Uuid,
        comment_id: Uuid,
    ) -> Result<i64, sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        let deleted = sqlx::query(
            r#"
            DELETE FROM comment_likes
            WHERE comment_id = $1 AND user_id = $2
            "#,
        )
        .bind(comment_id)
        .bind(user_id)
        .execute(&mut *tx)
        .await?
        .rows_affected();

        if deleted == 1 {
            sqlx::query(
                r#"
                UPDATE comments
                SET like_count = GREATEST(like_count - 1, 0)
                WHERE id = $1
                "#,
            )
            .bind(comment_id)
            .execute(&mut *tx)
            .await?;
        }

        let like_count =
            sqlx::query_scalar::<_, i64>("SELECT like_count FROM comments WHERE id = $1")
                .bind(comment_id)
                .fetch_one(&mut *tx)
                .await?;
        tx.commit().await?;
        Ok(like_count)
    }

    pub async fn favorite_topic(&self, user_id: Uuid, topic_id: Uuid) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            r#"
            INSERT INTO favorites (topic_id, user_id)
            VALUES ($1, $2)
            ON CONFLICT (topic_id, user_id) DO NOTHING
            "#,
        )
        .bind(topic_id)
        .bind(user_id)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() == 1)
    }

    pub async fn unfavorite_topic(&self, user_id: Uuid, topic_id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            DELETE FROM favorites
            WHERE topic_id = $1 AND user_id = $2
            "#,
        )
        .bind(topic_id)
        .bind(user_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn list_favorites(
        &self,
        user_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<FavoriteItem>, i64), sqlx::Error> {
        let total = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT count(*)
            FROM favorites f
            JOIN topics t ON t.id = f.topic_id
            JOIN categories c ON c.id = t.category_id
            WHERE f.user_id = $1
              AND t.status = 'published'
              AND c.is_visible = true
            "#,
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        let rows = sqlx::query_as::<_, FavoriteRow>(
            r#"
            SELECT
                f.created_at AS favorited_at,
                t.id,
                t.category_id,
                t.author_id,
                t.title,
                t.slug,
                NULL::text AS content,
                t.summary,
                t.status,
                t.view_count,
                t.reply_count,
                t.like_count,
                t.is_pinned,
                t.is_featured,
                t.last_reply_at,
                t.deleted_at,
                t.created_at,
                t.updated_at,
                c.slug AS category_slug,
                c.name AS category_name,
                c.icon AS category_icon,
                c.is_visible AS category_is_visible,
                u.username AS author_username,
                u.nickname AS author_nickname,
                u.avatar_url AS author_avatar,
                r.code AS author_role_code,
                r.name AS author_role_name,
                EXISTS (SELECT 1 FROM polls poll WHERE poll.topic_id = t.id) AS has_poll
            FROM favorites f
            JOIN topics t ON t.id = f.topic_id
            JOIN categories c ON c.id = t.category_id
            JOIN users u ON u.id = t.author_id
            JOIN roles r ON r.id = u.role_id
            WHERE f.user_id = $1
              AND t.status = 'published'
              AND c.is_visible = true
            ORDER BY f.created_at DESC, f.id DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(user_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let items = rows
            .into_iter()
            .map(|row| FavoriteItem {
                favorited_at: row.favorited_at,
                topic: repository_topic_to_summary(RepositoryTopic {
                    id: row.id,
                    category_id: row.category_id,
                    author_id: row.author_id,
                    title: row.title,
                    slug: row.slug,
                    content: row.content,
                    summary: row.summary,
                    status: row.status,
                    view_count: row.view_count,
                    reply_count: row.reply_count,
                    like_count: row.like_count,
                    is_pinned: row.is_pinned,
                    is_featured: row.is_featured,
                    last_reply_at: row.last_reply_at,
                    deleted_at: row.deleted_at,
                    created_at: row.created_at,
                    updated_at: row.updated_at,
                    category_slug: row.category_slug,
                    category_name: row.category_name,
                    category_icon: row.category_icon,
                    category_is_visible: row.category_is_visible,
                    author_username: row.author_username,
                    author_nickname: row.author_nickname,
                    author_avatar: row.author_avatar,
                    author_role_code: row.author_role_code,
                    author_role_name: row.author_role_name,
                    has_poll: row.has_poll,
                }),
            })
            .collect();

        Ok((items, total))
    }

    pub async fn follow_user(
        &self,
        follower_id: Uuid,
        following_id: Uuid,
    ) -> Result<(FollowCounters, bool), sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        let inserted = sqlx::query(
            r#"
            INSERT INTO user_follows (follower_id, following_id)
            VALUES ($1, $2)
            ON CONFLICT (follower_id, following_id) DO NOTHING
            "#,
        )
        .bind(follower_id)
        .bind(following_id)
        .execute(&mut *tx)
        .await?
        .rows_affected();

        if inserted == 1 {
            sqlx::query(
                r#"
                UPDATE users
                SET following_count = following_count + 1
                WHERE id = $1
                "#,
            )
            .bind(follower_id)
            .execute(&mut *tx)
            .await?;
            sqlx::query(
                r#"
                UPDATE users
                SET followers_count = followers_count + 1
                WHERE id = $1
                "#,
            )
            .bind(following_id)
            .execute(&mut *tx)
            .await?;
        }

        let counters = load_follow_counters(&mut tx, following_id).await?;
        tx.commit().await?;
        Ok((counters, inserted == 1))
    }

    pub async fn unfollow_user(
        &self,
        follower_id: Uuid,
        following_id: Uuid,
    ) -> Result<FollowCounters, sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        let deleted = sqlx::query(
            r#"
            DELETE FROM user_follows
            WHERE follower_id = $1 AND following_id = $2
            "#,
        )
        .bind(follower_id)
        .bind(following_id)
        .execute(&mut *tx)
        .await?
        .rows_affected();

        if deleted == 1 {
            sqlx::query(
                r#"
                UPDATE users
                SET following_count = GREATEST(following_count - 1, 0)
                WHERE id = $1
                "#,
            )
            .bind(follower_id)
            .execute(&mut *tx)
            .await?;
            sqlx::query(
                r#"
                UPDATE users
                SET followers_count = GREATEST(followers_count - 1, 0)
                WHERE id = $1
                "#,
            )
            .bind(following_id)
            .execute(&mut *tx)
            .await?;
        }

        let counters = load_follow_counters(&mut tx, following_id).await?;
        tx.commit().await?;
        Ok(counters)
    }

    pub async fn list_followers(
        &self,
        user_id: Uuid,
        viewer_id: Option<Uuid>,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<UserPublicSummary>, i64), sqlx::Error> {
        let total = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT count(*)
            FROM user_follows
            WHERE following_id = $1
            "#,
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        let rows = sqlx::query_as::<_, RepositoryPublicUser>(
            r#"
            SELECT
                u.id,
                u.username,
                u.nickname,
                u.avatar_url AS avatar,
                r.code AS role_code,
                r.name AS role_name,
                u.followers_count,
                u.following_count,
                CASE
                    WHEN $2::uuid IS NULL THEN false
                    ELSE EXISTS (
                        SELECT 1 FROM user_follows vf
                        WHERE vf.follower_id = $2 AND vf.following_id = u.id
                    )
                END AS is_following,
                u.created_at
            FROM user_follows f
            JOIN users u ON u.id = f.follower_id
            JOIN roles r ON r.id = u.role_id
            WHERE f.following_id = $1
            ORDER BY f.created_at DESC, f.id DESC
            LIMIT $3 OFFSET $4
            "#,
        )
        .bind(user_id)
        .bind(viewer_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok((rows.into_iter().map(to_public_user).collect(), total))
    }

    pub async fn list_following(
        &self,
        user_id: Uuid,
        viewer_id: Option<Uuid>,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<UserPublicSummary>, i64), sqlx::Error> {
        let total = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT count(*)
            FROM user_follows
            WHERE follower_id = $1
            "#,
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        let rows = sqlx::query_as::<_, RepositoryPublicUser>(
            r#"
            SELECT
                u.id,
                u.username,
                u.nickname,
                u.avatar_url AS avatar,
                r.code AS role_code,
                r.name AS role_name,
                u.followers_count,
                u.following_count,
                CASE
                    WHEN $2::uuid IS NULL THEN false
                    ELSE EXISTS (
                        SELECT 1 FROM user_follows vf
                        WHERE vf.follower_id = $2 AND vf.following_id = u.id
                    )
                END AS is_following,
                u.created_at
            FROM user_follows f
            JOIN users u ON u.id = f.following_id
            JOIN roles r ON r.id = u.role_id
            WHERE f.follower_id = $1
            ORDER BY f.created_at DESC, f.id DESC
            LIMIT $3 OFFSET $4
            "#,
        )
        .bind(user_id)
        .bind(viewer_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok((rows.into_iter().map(to_public_user).collect(), total))
    }

    pub async fn get_public_user(
        &self,
        user_id: Uuid,
        viewer_id: Option<Uuid>,
    ) -> Result<Option<UserPublicSummary>, sqlx::Error> {
        let row = sqlx::query_as::<_, RepositoryPublicUser>(
            r#"
            SELECT
                u.id,
                u.username,
                u.nickname,
                u.avatar_url AS avatar,
                r.code AS role_code,
                r.name AS role_name,
                u.followers_count,
                u.following_count,
                CASE
                    WHEN $2::uuid IS NULL THEN false
                    ELSE EXISTS (
                        SELECT 1 FROM user_follows vf
                        WHERE vf.follower_id = $2 AND vf.following_id = u.id
                    )
                END AS is_following,
                u.created_at
            FROM users u
            JOIN roles r ON r.id = u.role_id
            WHERE u.id = $1 AND u.status = 'active'
            "#,
        )
        .bind(user_id)
        .bind(viewer_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(to_public_user))
    }

    pub async fn topic_author_id(&self, topic_id: Uuid) -> Result<Option<Uuid>, sqlx::Error> {
        sqlx::query_scalar::<_, Uuid>(
            r#"
            SELECT author_id
            FROM topics
            WHERE id = $1 AND status = 'published'
            "#,
        )
        .bind(topic_id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn liked_comment_ids(
        &self,
        user_id: Uuid,
        comment_ids: &[Uuid],
    ) -> Result<Vec<Uuid>, sqlx::Error> {
        if comment_ids.is_empty() {
            return Ok(Vec::new());
        }
        sqlx::query_scalar::<_, Uuid>(
            r#"
            SELECT comment_id
            FROM comment_likes
            WHERE user_id = $1
              AND comment_id = ANY($2)
            "#,
        )
        .bind(user_id)
        .bind(comment_ids)
        .fetch_all(&self.pool)
        .await
    }
}

async fn load_follow_counters(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    user_id: Uuid,
) -> Result<FollowCounters, sqlx::Error> {
    sqlx::query_as::<_, FollowCounters>(
        r#"
        SELECT followers_count, following_count
        FROM users
        WHERE id = $1
        "#,
    )
    .bind(user_id)
    .fetch_one(&mut **tx)
    .await
}

fn to_public_user(user: RepositoryPublicUser) -> UserPublicSummary {
    UserPublicSummary {
        id: user.id,
        username: user.username,
        nickname: user.nickname,
        avatar: user.avatar,
        role: RoleSummary {
            code: user.role_code,
            name: user.role_name,
        },
        followers_count: user.followers_count,
        following_count: user.following_count,
        is_following: user.is_following,
        created_at: user.created_at,
    }
}
