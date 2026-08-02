//! Phase 14: poll repository — SQL for polls, options, votes, results, admin.

use chrono::{DateTime, Utc};
use sqlx::{PgPool, Postgres, QueryBuilder, Row, Transaction};
use uuid::Uuid;

use crate::models::{
    AdminPollItem, CategorySummary, HotPollItem, PollOptionItem, PollOptionRecord, PollRecord,
    PollResultOption, PollResults, PollVoterItem,
};

#[derive(Clone)]
pub struct PollRepository {
    pool: PgPool,
}

#[derive(Clone, Debug)]
pub struct NewPollOption<'a> {
    pub content: &'a str,
    pub sort_order: i32,
}

#[derive(Clone, Debug)]
pub struct NewPoll<'a> {
    pub topic_id: Uuid,
    pub author_id: Uuid,
    pub title: &'a str,
    pub description: Option<&'a str>,
    pub multiple_choice: bool,
    pub anonymous: bool,
    pub allow_cancel: bool,
    pub max_choices: i32,
    pub expires_at: Option<DateTime<Utc>>,
    pub options: Vec<NewPollOption<'a>>,
}

#[derive(sqlx::FromRow)]
pub struct PollVoterRow {
    pub option_id: Uuid,
    pub user_id: Uuid,
    pub username: String,
    pub nickname: Option<String>,
    pub avatar: Option<String>,
}

#[derive(sqlx::FromRow)]
pub struct HotPollRow {
    pub poll_id: Uuid,
    pub topic_id: Uuid,
    pub topic_slug: String,
    pub topic_title: String,
    pub poll_title: String,
    pub participant_count: i64,
    pub option_count: i64,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub category_id: Uuid,
    pub category_slug: String,
    pub category_name: String,
    pub category_icon: Option<String>,
}

#[derive(sqlx::FromRow)]
pub struct AdminPollRow {
    pub id: Uuid,
    pub topic_id: Uuid,
    pub topic_title: String,
    pub topic_slug: String,
    pub title: String,
    pub status: String,
    pub multiple_choice: bool,
    pub anonymous: bool,
    pub max_choices: i32,
    pub option_count: i64,
    pub participant_count: i64,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub author_id: Uuid,
    pub author_username: String,
}

impl PollRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    // ------------------------------------------------------------------
    // Create
    // ------------------------------------------------------------------

    /// Insert poll + options atomically.
    pub async fn create(
        &self,
        input: NewPoll<'_>,
    ) -> Result<(PollRecord, Vec<PollOptionRecord>), sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        let poll = sqlx::query_as::<_, PollRecord>(
            r#"
            INSERT INTO polls (
                topic_id, author_id, title, description, multiple_choice, anonymous,
                allow_cancel, max_choices, expires_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING id, topic_id, author_id, title, description, poll_type, status,
                      multiple_choice, anonymous, allow_cancel, max_choices, expires_at,
                      created_at, updated_at
            "#,
        )
        .bind(input.topic_id)
        .bind(input.author_id)
        .bind(input.title)
        .bind(input.description)
        .bind(input.multiple_choice)
        .bind(input.anonymous)
        .bind(input.allow_cancel)
        .bind(input.max_choices)
        .bind(input.expires_at)
        .fetch_one(&mut *tx)
        .await?;

        let mut options = Vec::with_capacity(input.options.len());
        for option in input.options {
            let record = sqlx::query_as::<_, PollOptionRecord>(
                r#"
                INSERT INTO poll_options (poll_id, content, sort_order)
                VALUES ($1, $2, $3)
                RETURNING id, poll_id, content, sort_order, vote_count, created_at
                "#,
            )
            .bind(poll.id)
            .bind(option.content)
            .bind(option.sort_order)
            .fetch_one(&mut *tx)
            .await?;
            options.push(record);
        }

        tx.commit().await?;
        Ok((poll, options))
    }

    // ------------------------------------------------------------------
    // Read
    // ------------------------------------------------------------------

    pub async fn find_by_id(&self, poll_id: Uuid) -> Result<Option<PollRecord>, sqlx::Error> {
        sqlx::query_as::<_, PollRecord>(
            r#"
            SELECT id, topic_id, author_id, title, description, poll_type, status,
                   multiple_choice, anonymous, allow_cancel, max_choices, expires_at,
                   created_at, updated_at
            FROM polls
            WHERE id = $1
            "#,
        )
        .bind(poll_id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn find_by_topic_id(
        &self,
        topic_id: Uuid,
    ) -> Result<Option<PollRecord>, sqlx::Error> {
        sqlx::query_as::<_, PollRecord>(
            r#"
            SELECT id, topic_id, author_id, title, description, poll_type, status,
                   multiple_choice, anonymous, allow_cancel, max_choices, expires_at,
                   created_at, updated_at
            FROM polls
            WHERE topic_id = $1
            "#,
        )
        .bind(topic_id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn list_options(&self, poll_id: Uuid) -> Result<Vec<PollOptionRecord>, sqlx::Error> {
        sqlx::query_as::<_, PollOptionRecord>(
            r#"
            SELECT id, poll_id, content, sort_order, vote_count, created_at
            FROM poll_options
            WHERE poll_id = $1
            ORDER BY sort_order, id
            "#,
        )
        .bind(poll_id)
        .fetch_all(&self.pool)
        .await
    }

    /// Option ids a viewer voted for on this poll.
    pub async fn my_votes(&self, poll_id: Uuid, user_id: Uuid) -> Result<Vec<Uuid>, sqlx::Error> {
        sqlx::query_scalar::<_, Uuid>(
            r#"
            SELECT option_id
            FROM poll_votes
            WHERE poll_id = $1 AND user_id = $2
            ORDER BY created_at, id
            "#,
        )
        .bind(poll_id)
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn topic_meta(
        &self,
        topic_id: Uuid,
    ) -> Result<Option<(String, String)>, sqlx::Error> {
        sqlx::query_as::<_, (String, String)>(
            r#"
            SELECT slug, title FROM topics WHERE id = $1
            "#,
        )
        .bind(topic_id)
        .fetch_optional(&self.pool)
        .await
    }

    // ------------------------------------------------------------------
    // Vote (transactional, row-lock serialized)
    // ------------------------------------------------------------------

    /// Vote for a single option.
    /// Locks the poll row so concurrent votes on the same poll serialize; the
    /// caller decides semantics (single vs multi) after reading poll state.
    pub async fn vote(
        &self,
        poll_id: Uuid,
        option_id: Uuid,
        user_id: Uuid,
    ) -> Result<VoteOutcome, VoteError> {
        let mut tx = self.pool.begin().await?;
        let poll = sqlx::query_as::<_, PollRecord>(
            r#"
            SELECT id, topic_id, author_id, title, description, poll_type, status,
                   multiple_choice, anonymous, allow_cancel, max_choices, expires_at,
                   created_at, updated_at
            FROM polls
            WHERE id = $1
            FOR UPDATE
            "#,
        )
        .bind(poll_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(VoteError::PollNotFound)?;

        let option_belongs = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS (SELECT 1 FROM poll_options WHERE id = $1 AND poll_id = $2)",
        )
        .bind(option_id)
        .bind(poll_id)
        .fetch_one(&mut *tx)
        .await?;
        if !option_belongs {
            return Err(VoteError::OptionNotFound);
        }

        let already_voted = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS (SELECT 1 FROM poll_votes WHERE poll_id = $1 AND user_id = $2 AND option_id = $3)",
        )
        .bind(poll_id)
        .bind(user_id)
        .bind(option_id)
        .fetch_one(&mut *tx)
        .await?;
        if already_voted {
            return Err(VoteError::AlreadyVoted);
        }

        // The unique constraint (poll_id, user_id, option_id) is the backstop;
        // a violation surfaces as sqlx::Error::Database with code 23505.
        sqlx::query(
            r#"
            INSERT INTO poll_votes (poll_id, option_id, user_id)
            VALUES ($1, $2, $3)
            "#,
        )
        .bind(poll_id)
        .bind(option_id)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

        sqlx::query("UPDATE poll_options SET vote_count = vote_count + 1 WHERE id = $1")
            .bind(option_id)
            .execute(&mut *tx)
            .await?;

        let (total_votes, participants) = poll_totals(&mut tx, poll_id).await?;
        tx.commit().await?;
        Ok(VoteOutcome {
            poll,
            total_votes,
            participants,
        })
    }

    /// Remove one vote (or all votes when option_id is None).
    /// Returns the affected option ids.
    pub async fn cancel_votes(
        &self,
        poll_id: Uuid,
        user_id: Uuid,
        option_id: Option<Uuid>,
    ) -> Result<Vec<Uuid>, sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        let removed: Vec<Uuid> = match option_id {
            Some(option_id) => sqlx::query(
                r#"
                    DELETE FROM poll_votes
                    WHERE poll_id = $1 AND user_id = $2 AND option_id = $3
                    RETURNING option_id
                    "#,
            )
            .bind(poll_id)
            .bind(user_id)
            .bind(option_id)
            .fetch_all(&mut *tx)
            .await?
            .into_iter()
            .map(|row| row.get::<Uuid, _>("option_id"))
            .collect(),
            None => sqlx::query(
                r#"
                    DELETE FROM poll_votes
                    WHERE poll_id = $1 AND user_id = $2
                    RETURNING option_id
                    "#,
            )
            .bind(poll_id)
            .bind(user_id)
            .fetch_all(&mut *tx)
            .await?
            .into_iter()
            .map(|row| row.get::<Uuid, _>("option_id"))
            .collect(),
        };

        if !removed.is_empty() {
            let mut builder: QueryBuilder<Postgres> = QueryBuilder::new(
                "UPDATE poll_options SET vote_count = vote_count - 1 WHERE id IN (",
            );
            let mut separated = builder.separated(", ");
            for id in &removed {
                separated.push_bind(*id);
            }
            builder.push(")");
            builder.build().execute(&mut *tx).await?;
        }

        tx.commit().await?;
        Ok(removed)
    }

    pub async fn count_votes_by_user(
        &self,
        poll_id: Uuid,
        user_id: Uuid,
    ) -> Result<i64, sqlx::Error> {
        sqlx::query_scalar::<_, i64>(
            r#"
            SELECT count(*) FROM poll_votes WHERE poll_id = $1 AND user_id = $2
            "#,
        )
        .bind(poll_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn count_participants(&self, poll_id: Uuid) -> Result<i64, sqlx::Error> {
        sqlx::query_scalar::<_, i64>(
            "SELECT count(DISTINCT user_id) FROM poll_votes WHERE poll_id = $1",
        )
        .bind(poll_id)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn totals(&self, poll_id: Uuid) -> Result<(i64, i64), sqlx::Error> {
        let total = sqlx::query_scalar::<_, i64>(
            "SELECT COALESCE(sum(vote_count), 0) FROM poll_options WHERE poll_id = $1",
        )
        .bind(poll_id)
        .fetch_one(&self.pool)
        .await?;
        let participants = self.count_participants(poll_id).await?;
        Ok((total, participants))
    }

    // ------------------------------------------------------------------
    // Results
    // ------------------------------------------------------------------

    pub async fn results(
        &self,
        poll_id: Uuid,
        include_voters: bool,
    ) -> Result<Option<PollResults>, sqlx::Error> {
        let poll = sqlx::query_as::<_, PollRecord>(
            r#"
            SELECT p.id, p.topic_id, p.author_id, p.title, p.description, p.poll_type,
                   p.status, p.multiple_choice, p.anonymous, p.allow_cancel, p.max_choices,
                   p.expires_at, p.created_at, p.updated_at
            FROM polls p
            WHERE p.id = $1
            "#,
        )
        .bind(poll_id)
        .fetch_optional(&self.pool)
        .await?;

        let Some(poll) = poll else { return Ok(None) };

        let (topic_slug, topic_title) =
            sqlx::query_as::<_, (String, String)>("SELECT slug, title FROM topics WHERE id = $1")
                .bind(poll.topic_id)
                .fetch_one(&self.pool)
                .await?;

        let rows = sqlx::query_as::<_, PollOptionRecord>(
            r#"
            SELECT id, poll_id, content, sort_order, vote_count, created_at
            FROM poll_options
            WHERE poll_id = $1
            ORDER BY sort_order, id
            "#,
        )
        .bind(poll_id)
        .fetch_all(&self.pool)
        .await?;

        let total_votes: i64 = rows.iter().map(|row| i64::from(row.vote_count)).sum();
        let participants = sqlx::query_scalar::<_, i64>(
            "SELECT count(DISTINCT user_id) FROM poll_votes WHERE poll_id = $1",
        )
        .bind(poll_id)
        .fetch_one(&self.pool)
        .await?;

        let options = rows
            .iter()
            .map(|row| PollResultOption {
                option_id: row.id,
                content: row.content.clone(),
                vote_count: i64::from(row.vote_count),
                percentage: percentage(i64::from(row.vote_count), total_votes),
            })
            .collect();

        let voters = if include_voters {
            let rows = sqlx::query_as::<_, PollVoterRow>(
                r#"
                SELECT v.option_id, u.id AS user_id, u.username, u.nickname, u.avatar_url AS avatar
                FROM poll_votes v
                JOIN users u ON u.id = v.user_id
                WHERE v.poll_id = $1
                ORDER BY v.created_at, v.id
                "#,
            )
            .bind(poll_id)
            .fetch_all(&self.pool)
            .await?;
            Some(
                rows.into_iter()
                    .map(|row| PollVoterItem {
                        user_id: row.user_id,
                        username: row.username,
                        nickname: row.nickname,
                        avatar: row.avatar,
                        option_id: row.option_id,
                    })
                    .collect(),
            )
        } else {
            None
        };

        Ok(Some(PollResults {
            poll_id: poll.id,
            topic_id: poll.topic_id,
            topic_slug,
            topic_title,
            title: poll.title,
            status: if poll.status == "closed" {
                crate::models::PollStatus::Closed
            } else {
                crate::models::PollStatus::Active
            },
            multiple_choice: poll.multiple_choice,
            anonymous: poll.anonymous,
            expires_at: poll.expires_at,
            total_votes,
            participant_count: participants,
            options,
            voters,
        }))
    }

    // ------------------------------------------------------------------
    // Mutations
    // ------------------------------------------------------------------

    pub async fn update_fields(
        &self,
        poll_id: Uuid,
        title: Option<&str>,
        description: Option<Option<&str>>,
        expires_at: Option<Option<DateTime<Utc>>>,
        allow_cancel: Option<bool>,
    ) -> Result<PollRecord, sqlx::Error> {
        sqlx::query_as::<_, PollRecord>(
            r#"
            UPDATE polls
            SET title = COALESCE($2, title),
                description = CASE WHEN $3::boolean THEN $4::text ELSE description END,
                expires_at = CASE WHEN $5::boolean THEN $6::timestamptz ELSE expires_at END,
                allow_cancel = COALESCE($7, allow_cancel)
            WHERE id = $1
            RETURNING id, topic_id, author_id, title, description, poll_type, status,
                      multiple_choice, anonymous, allow_cancel, max_choices, expires_at,
                      created_at, updated_at
            "#,
        )
        .bind(poll_id)
        .bind(title)
        .bind(description.is_some())
        .bind(description.flatten())
        .bind(expires_at.is_some())
        .bind(expires_at.flatten())
        .bind(allow_cancel)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(sqlx::Error::RowNotFound)
    }

    pub async fn close(&self, poll_id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE polls SET status = 'closed' WHERE id = $1 AND status = 'active'")
            .bind(poll_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Hard delete (options/votes cascade) — used by admin for violating polls.
    pub async fn delete(&self, poll_id: Uuid) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("DELETE FROM polls WHERE id = $1")
            .bind(poll_id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    /// Update poll fields + manage options atomically: append new options and
    /// remove zero-vote options. Options that already have votes are protected.
    pub async fn update_with_options(
        &self,
        poll_id: Uuid,
        title: Option<&str>,
        description: Option<Option<&str>>,
        expires_at: Option<Option<DateTime<Utc>>>,
        allow_cancel: Option<bool>,
        options_to_add: &[String],
        option_ids_to_remove: &[Uuid],
    ) -> Result<PollRecord, PollUpdateError> {
        let mut tx = self.pool.begin().await?;

        let poll = sqlx::query_as::<_, PollRecord>(
            r#"
            UPDATE polls
            SET title = COALESCE($2, title),
                description = CASE WHEN $3::boolean THEN $4::text ELSE description END,
                expires_at = CASE WHEN $5::boolean THEN $6::timestamptz ELSE expires_at END,
                allow_cancel = COALESCE($7, allow_cancel)
            WHERE id = $1
            RETURNING id, topic_id, author_id, title, description, poll_type, status,
                      multiple_choice, anonymous, allow_cancel, max_choices, expires_at,
                      created_at, updated_at
            "#,
        )
        .bind(poll_id)
        .bind(title)
        .bind(description.is_some())
        .bind(description.flatten())
        .bind(expires_at.is_some())
        .bind(expires_at.flatten())
        .bind(allow_cancel)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(PollUpdateError::NotFound)?;

        // Refuse to remove options that already received votes (protects results).
        if !option_ids_to_remove.is_empty() {
            let voted = sqlx::query_scalar::<_, Uuid>(
                r#"
                SELECT id FROM poll_options
                WHERE poll_id = $1 AND id = ANY($2) AND vote_count > 0
                "#,
            )
            .bind(poll_id)
            .bind(option_ids_to_remove)
            .fetch_all(&mut *tx)
            .await?;
            if !voted.is_empty() {
                return Err(PollUpdateError::OptionHasVotes);
            }
            // Unmatched ids (not belonging to this poll) are rejected as well.
            let matched = sqlx::query_scalar::<_, Uuid>(
                r#"
                SELECT id FROM poll_options
                WHERE poll_id = $1 AND id = ANY($2)
                "#,
            )
            .bind(poll_id)
            .bind(option_ids_to_remove)
            .fetch_all(&mut *tx)
            .await?;
            if matched.len() != option_ids_to_remove.len() {
                return Err(PollUpdateError::UnknownOption);
            }
            // Votes cascade-delete via the FK on poll_votes.option_id.
            sqlx::query(
                r#"
                DELETE FROM poll_options
                WHERE poll_id = $1 AND id = ANY($2) AND vote_count = 0
                "#,
            )
            .bind(poll_id)
            .bind(option_ids_to_remove)
            .execute(&mut *tx)
            .await?;
        }

        for content in options_to_add {
            let next_order = sqlx::query_scalar::<_, i32>(
                r#"
                SELECT COALESCE(MAX(sort_order), -1) + 1
                FROM poll_options WHERE poll_id = $1
                "#,
            )
            .bind(poll_id)
            .fetch_one(&mut *tx)
            .await?;
            sqlx::query(
                r#"
                INSERT INTO poll_options (poll_id, content, sort_order)
                VALUES ($1, $2, $3)
                "#,
            )
            .bind(poll_id)
            .bind(content)
            .bind(next_order)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(poll)
    }

    /// Mark expired polls as closed; returns affected poll ids.
    pub async fn close_expired(&self, now: DateTime<Utc>) -> Result<Vec<Uuid>, sqlx::Error> {
        let rows = sqlx::query_scalar::<_, Uuid>(
            r#"
            UPDATE polls
            SET status = 'closed', updated_at = now()
            WHERE status = 'active' AND expires_at IS NOT NULL AND expires_at <= $1
            RETURNING id
            "#,
        )
        .bind(now)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    // ------------------------------------------------------------------
    // Admin
    // ------------------------------------------------------------------

    pub async fn list_admin(
        &self,
        q: Option<&str>,
        status: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<AdminPollItem>, i64), sqlx::Error> {
        let rows = sqlx::query_as::<_, AdminPollRow>(
            r#"
            SELECT p.id, p.topic_id, t.title AS topic_title, t.slug AS topic_slug,
                   p.title, p.status, p.multiple_choice, p.anonymous, p.max_choices,
                   (SELECT count(*) FROM poll_options o WHERE o.poll_id = p.id) AS option_count,
                   (SELECT count(DISTINCT v.user_id) FROM poll_votes v WHERE v.poll_id = p.id) AS participant_count,
                   p.expires_at, p.created_at, p.updated_at,
                   p.author_id, u.username AS author_username
            FROM polls p
            JOIN topics t ON t.id = p.topic_id
            JOIN users u ON u.id = p.author_id
            WHERE ($1::text IS NULL
                   OR p.title ILIKE '%' || $1 || '%'
                   OR t.title ILIKE '%' || $1 || '%')
              AND ($2::text IS NULL OR p.status = $2)
            ORDER BY p.created_at DESC
            LIMIT $3 OFFSET $4
            "#,
        )
        .bind(q)
        .bind(status)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let total = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT count(*)
            FROM polls p
            JOIN topics t ON t.id = p.topic_id
            WHERE ($1::text IS NULL
                   OR p.title ILIKE '%' || $1 || '%'
                   OR t.title ILIKE '%' || $1 || '%')
              AND ($2::text IS NULL OR p.status = $2)
            "#,
        )
        .bind(q)
        .bind(status)
        .fetch_one(&self.pool)
        .await?;

        let items = rows
            .into_iter()
            .map(|row| AdminPollItem {
                id: row.id,
                topic_id: row.topic_id,
                topic_title: row.topic_title,
                topic_slug: row.topic_slug,
                title: row.title,
                status: row.status,
                multiple_choice: row.multiple_choice,
                anonymous: row.anonymous,
                max_choices: row.max_choices,
                option_count: row.option_count,
                participant_count: row.participant_count,
                expires_at: row.expires_at,
                created_at: row.created_at,
                updated_at: row.updated_at,
                author_id: row.author_id,
                author_username: row.author_username,
            })
            .collect();
        Ok((items, total))
    }

    // ------------------------------------------------------------------
    // Hot polls (cached)
    // ------------------------------------------------------------------

    pub async fn hot(&self, limit: i64) -> Result<Vec<HotPollItem>, sqlx::Error> {
        let rows = sqlx::query_as::<_, HotPollRow>(
            r#"
            SELECT p.id AS poll_id, p.topic_id, t.slug AS topic_slug, t.title AS topic_title,
                   p.title AS poll_title,
                   (SELECT count(DISTINCT v.user_id) FROM poll_votes v WHERE v.poll_id = p.id) AS participant_count,
                   (SELECT count(*) FROM poll_options o WHERE o.poll_id = p.id) AS option_count,
                   p.status, p.created_at,
                   c.id AS category_id, c.slug AS category_slug, c.name AS category_name,
                   c.icon AS category_icon
            FROM polls p
            JOIN topics t ON t.id = p.topic_id
            JOIN categories c ON c.id = t.category_id
            WHERE t.status = 'published' AND t.deleted_at IS NULL
              AND c.is_visible = true
            ORDER BY participant_count DESC, p.created_at DESC
            LIMIT $1
            "#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| HotPollItem {
                poll_id: row.poll_id,
                topic_id: row.topic_id,
                topic_slug: row.topic_slug,
                topic_title: row.topic_title,
                poll_title: row.poll_title,
                participant_count: row.participant_count,
                option_count: row.option_count,
                is_closed: row.status == "closed",
                category: CategorySummary {
                    id: row.category_id,
                    slug: row.category_slug,
                    name: row.category_name,
                    icon: row.category_icon,
                    restricted_posting: false,
                },
                created_at: row.created_at,
            })
            .collect())
    }
}

pub struct VoteOutcome {
    pub poll: PollRecord,
    pub total_votes: i64,
    pub participants: i64,
}

#[derive(Debug, thiserror::Error)]
pub enum VoteError {
    #[error("poll not found")]
    PollNotFound,
    #[error("option not found")]
    OptionNotFound,
    #[error("already voted for this option")]
    AlreadyVoted,
    #[error(transparent)]
    Database(#[from] sqlx::Error),
}

async fn poll_totals(
    tx: &mut Transaction<'_, Postgres>,
    poll_id: Uuid,
) -> Result<(i64, i64), sqlx::Error> {
    let total = sqlx::query_scalar::<_, i64>(
        "SELECT COALESCE(sum(vote_count), 0) FROM poll_options WHERE poll_id = $1",
    )
    .bind(poll_id)
    .fetch_one(&mut **tx)
    .await?;
    let participants = sqlx::query_scalar::<_, i64>(
        "SELECT count(DISTINCT user_id) FROM poll_votes WHERE poll_id = $1",
    )
    .bind(poll_id)
    .fetch_one(&mut **tx)
    .await?;
    Ok((total, participants))
}

fn percentage(vote_count: i64, total_votes: i64) -> f64 {
    if total_votes <= 0 {
        return 0.0;
    }
    let value = (vote_count as f64 / total_votes as f64) * 100.0;
    (value * 10.0).round() / 10.0
}

pub fn option_item(record: &PollOptionRecord) -> PollOptionItem {
    PollOptionItem {
        id: record.id,
        content: record.content.clone(),
        sort_order: record.sort_order,
        vote_count: i64::from(record.vote_count),
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PollUpdateError {
    #[error("poll not found")]
    NotFound,
    #[error("option has votes and cannot be removed")]
    OptionHasVotes,
    #[error("option does not belong to this poll")]
    UnknownOption,
    #[error(transparent)]
    Database(#[from] sqlx::Error),
}
