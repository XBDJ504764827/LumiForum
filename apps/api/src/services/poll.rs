//! Phase 14: poll service — business rules, permissions, vote integrity,
//! caching, realtime broadcast, notifications, expiry maintenance.

use chrono::{DateTime, Utc};
use redis::aio::ConnectionManager;
use redis::AsyncCommands;
use serde_json::json;
use thiserror::Error;
use uuid::Uuid;

use crate::events::{NotificationEvent, PollEndedEvent, PollVotedEvent};
use crate::models::{
    AuthenticatedPrincipal, CreatePollDraft, HotPollItem, PollDetail, PollOptionItem, PollRecord,
    PollResults, PollStatus, UpdatePollRequest, VotePollRequest, PERMISSION_ADMIN_ACCESS,
    PERMISSION_POLL_MANAGE, PERMISSION_POLL_VOTE, PERMISSION_TOPIC_CREATE,
};
use crate::realtime::RealtimeBus;
use crate::repositories::{
    option_item, PollRepository, PollUpdateError, TopicRepository, VoteError,
};
use crate::services::{ModerationService, NotificationService};

const RESULTS_CACHE_TTL_SECS: u64 = 60;
const HOT_CACHE_TTL_SECS: u64 = 300;
const HOT_POLLS_LIMIT: i64 = 10;
const MIN_OPTIONS: usize = 2;
const MAX_OPTIONS: usize = 20;
const MAX_OPTION_CHARS: usize = 500;

const RESULTS_CACHE_PREFIX: &str = "poll:results:";
const HOT_CACHE_KEY: &str = "poll:hot";

#[derive(Clone)]
pub struct PollService {
    repository: PollRepository,
    topics: TopicRepository,
    moderation: ModerationService,
    notifications: NotificationService,
    realtime: RealtimeBus,
    redis: ConnectionManager,
}

#[derive(Debug, Error)]
pub enum PollError {
    #[error("invalid poll input: {0}")]
    Validation(&'static str),
    #[error("poll not found")]
    NotFound,
    #[error("topic not found or unavailable")]
    TopicUnavailable,
    #[error("topic already has a poll")]
    PollExists,
    #[error("poll is closed")]
    PollClosed,
    #[error("poll has ended")]
    PollExpired,
    #[error("permission denied")]
    Forbidden,
    #[error("already voted for this option")]
    AlreadyVoted,
    #[error("you have already voted in this poll")]
    AlreadyParticipated,
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

impl PollService {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        repository: PollRepository,
        topics: TopicRepository,
        moderation: ModerationService,
        notifications: NotificationService,
        realtime: RealtimeBus,
        redis: ConnectionManager,
    ) -> Self {
        Self {
            repository,
            topics,
            moderation,
            notifications,
            realtime,
            redis,
        }
    }

    // ------------------------------------------------------------------
    // Create (attached to a topic)
    // ------------------------------------------------------------------

    pub async fn create_for_topic(
        &self,
        principal: &AuthenticatedPrincipal,
        topic_id: Uuid,
        draft: CreatePollDraft,
    ) -> Result<PollDetail, PollError> {
        require(principal, PERMISSION_TOPIC_CREATE)?;
        let topic = self
            .topics
            .find_by_id(topic_id)
            .await
            .map_err(internal)?
            .ok_or(PollError::TopicUnavailable)?;
        // Only the topic author may attach a poll.
        if topic.author_id != principal.user_id {
            return Err(PollError::Forbidden);
        }
        if topic.status != "published" {
            return Err(PollError::TopicUnavailable);
        }
        if self
            .repository
            .find_by_topic_id(topic_id)
            .await
            .map_err(internal)?
            .is_some()
        {
            return Err(PollError::PollExists);
        }

        let draft = normalize_draft(draft)?;

        // Auto-moderation screening on the poll title + options.
        let screening = self
            .moderation
            .screen_content(principal, "poll", &draft.title, &draft.options.join(" "))
            .await
            .map_err(map_moderation)?;
        if !screening.is_allowed() {
            return Err(PollError::Validation(
                "投票内容未通过自动审核，请修改后重试",
            ));
        }

        let options: Vec<String> = draft.options;
        let (poll, option_records) = self
            .repository
            .create(crate::repositories::NewPoll {
                topic_id,
                author_id: principal.user_id,
                title: &draft.title,
                description: draft.description.as_deref(),
                multiple_choice: draft.multiple_choice,
                anonymous: draft.anonymous,
                allow_cancel: draft.allow_cancel,
                max_choices: draft.max_choices,
                expires_at: draft.expires_at,
                options: options
                    .iter()
                    .enumerate()
                    .map(|(index, content)| crate::repositories::NewPollOption {
                        content,
                        sort_order: index as i32,
                    })
                    .collect(),
            })
            .await
            .map_err(internal)?;

        self.realtime
            .publish_poll_update(
                poll.id,
                json!({
                    "event": "created",
                    "poll_id": poll.id,
                    "topic_id": poll.topic_id,
                }),
            )
            .await;

        Ok(to_detail(
            &poll,
            topic.slug.as_str(),
            &topic.title,
            option_records.iter().map(option_item).collect(),
            0,
            0,
            Vec::new(),
            true,
            true,
        ))
    }

    // ------------------------------------------------------------------
    // Read
    // ------------------------------------------------------------------

    pub async fn get_by_id(
        &self,
        viewer: Option<&AuthenticatedPrincipal>,
        poll_id: Uuid,
    ) -> Result<PollDetail, PollError> {
        let poll = self
            .repository
            .find_by_id(poll_id)
            .await
            .map_err(internal)?
            .ok_or(PollError::NotFound)?;
        self.assemble_detail(viewer, poll).await
    }

    pub async fn get_by_topic(
        &self,
        viewer: Option<&AuthenticatedPrincipal>,
        topic_id: Uuid,
    ) -> Result<PollDetail, PollError> {
        let poll = self
            .repository
            .find_by_topic_id(topic_id)
            .await
            .map_err(internal)?
            .ok_or(PollError::NotFound)?;
        self.assemble_detail(viewer, poll).await
    }

    async fn assemble_detail(
        &self,
        viewer: Option<&AuthenticatedPrincipal>,
        poll: PollRecord,
    ) -> Result<PollDetail, PollError> {
        let (topic_slug, topic_title) = self
            .repository
            .topic_meta(poll.topic_id)
            .await
            .map_err(internal)?
            .ok_or(PollError::NotFound)?;
        let options = self
            .repository
            .list_options(poll.id)
            .await
            .map_err(internal)?;
        let total_votes: i64 = options.iter().map(|row| i64::from(row.vote_count)).sum();
        let participants = self
            .repository
            .count_participants(poll.id)
            .await
            .map_err(internal)?;

        let my_votes = match viewer {
            Some(principal) => self
                .repository
                .my_votes(poll.id, principal.user_id)
                .await
                .map_err(internal)?,
            None => Vec::new(),
        };

        let (can_vote, can_manage) = match viewer {
            Some(principal) => {
                let is_author = principal.user_id == poll.author_id;
                let elevated = principal.has_permission(PERMISSION_POLL_MANAGE);
                let can_manage = is_author || elevated;
                let can_vote = !is_author && principal.has_permission(PERMISSION_POLL_VOTE);
                (can_vote, can_manage)
            }
            None => (false, false),
        };

        Ok(to_detail(
            &poll,
            &topic_slug,
            &topic_title,
            options.iter().map(option_item).collect(),
            total_votes,
            participants,
            my_votes,
            can_vote,
            can_manage,
        ))
    }

    // ------------------------------------------------------------------
    // Voting (④ Vote Service)
    // ------------------------------------------------------------------

    pub async fn vote(
        &self,
        principal: &AuthenticatedPrincipal,
        poll_id: Uuid,
        request: VotePollRequest,
    ) -> Result<PollDetail, PollError> {
        require(principal, PERMISSION_POLL_VOTE)?;
        self.moderation
            .enforce_content_allowed(principal)
            .await
            .map_err(map_moderation)?;

        // Resolve the poll up-front for cheap validation before locking.
        let poll = self
            .repository
            .find_by_id(poll_id)
            .await
            .map_err(internal)?
            .ok_or(PollError::NotFound)?;
        self.ensure_votable(&poll)?;
        if poll.author_id == principal.user_id {
            return Err(PollError::Validation("作者不能参与自己的投票"));
        }

        let option_ids = dedupe_options(request.option_ids);
        if option_ids.is_empty() {
            return Err(PollError::Validation("请选择至少一个选项"));
        }
        if !poll.multiple_choice && option_ids.len() != 1 {
            return Err(PollError::Validation("单选投票每次只能选择一个选项"));
        }
        if option_ids.len() > poll.max_choices as usize {
            return Err(PollError::Validation("超出最多可选数量"));
        }

        let existing = self
            .repository
            .count_votes_by_user(poll_id, principal.user_id)
            .await
            .map_err(internal)?;
        if !poll.multiple_choice && existing > 0 {
            return Err(PollError::AlreadyParticipated);
        }
        if poll.multiple_choice && existing + option_ids.len() as i64 > i64::from(poll.max_choices)
        {
            return Err(PollError::Validation("加上已选选项后超出最多可选数量"));
        }

        // Each option is inserted under the poll row lock (serialized votes).
        let mut total_votes = 0_i64;
        let mut participants = 0_i64;
        for option_id in &option_ids {
            match self
                .repository
                .vote(poll_id, *option_id, principal.user_id)
                .await
            {
                Ok(outcome) => {
                    total_votes = outcome.total_votes;
                    participants = outcome.participants;
                }
                Err(VoteError::AlreadyVoted) => {
                    return Err(PollError::AlreadyVoted);
                }
                Err(VoteError::PollNotFound) => return Err(PollError::NotFound),
                Err(VoteError::OptionNotFound) => {
                    return Err(PollError::Validation("选项不存在或不属于该投票"));
                }
                Err(VoteError::Database(error)) => {
                    // Unique constraint backstop: (poll_id, user_id, option_id).
                    if is_unique_violation(&error) {
                        return Err(PollError::AlreadyVoted);
                    }
                    return Err(PollError::Internal(error.into()));
                }
            }
        }

        self.after_change(poll_id, poll.topic_id, "vote", total_votes, participants)
            .await;

        // Notify the poll author that someone participated.
        let (topic_slug, topic_title) = self
            .repository
            .topic_meta(poll.topic_id)
            .await
            .map_err(internal)?
            .unwrap_or_default();
        let _ = self
            .notifications
            .handle_event(NotificationEvent::PollVoted(PollVotedEvent {
                actor_id: principal.user_id,
                recipient_id: poll.author_id,
                poll_id,
                topic_id: poll.topic_id,
                topic_slug,
                topic_title,
                poll_title: poll.title.clone(),
            }))
            .await;

        self.get_by_id(Some(principal), poll_id).await
    }

    pub async fn cancel_vote(
        &self,
        principal: &AuthenticatedPrincipal,
        poll_id: Uuid,
        option_id: Option<Uuid>,
    ) -> Result<PollDetail, PollError> {
        require(principal, PERMISSION_POLL_VOTE)?;
        let poll = self
            .repository
            .find_by_id(poll_id)
            .await
            .map_err(internal)?
            .ok_or(PollError::NotFound)?;
        self.ensure_votable(&poll)?;
        if !poll.allow_cancel {
            return Err(PollError::Validation("该投票不允许取消已投出的票"));
        }

        let removed = self
            .repository
            .cancel_votes(poll_id, principal.user_id, option_id)
            .await
            .map_err(internal)?;
        if removed.is_empty() {
            return Err(PollError::Validation("你还没有参与该投票"));
        }

        let (total_votes, participants) =
            self.repository.totals(poll_id).await.map_err(internal)?;
        self.after_change(poll_id, poll.topic_id, "cancel", total_votes, participants)
            .await;

        self.get_by_id(Some(principal), poll_id).await
    }

    // ------------------------------------------------------------------
    // Results
    // ------------------------------------------------------------------

    pub async fn results(
        &self,
        principal: Option<&AuthenticatedPrincipal>,
        poll_id: Uuid,
    ) -> Result<PollResults, PollError> {
        let cache_key = format!("{RESULTS_CACHE_PREFIX}{poll_id}");
        if let Some(cached) = self.cached_results(&cache_key).await {
            return Ok(cached);
        }
        let poll = self
            .repository
            .find_by_id(poll_id)
            .await
            .map_err(internal)?
            .ok_or(PollError::NotFound)?;
        // Voter list is only exposed for public (non-anonymous) polls.
        let include_voters = !poll.anonymous;
        let results = self
            .repository
            .results(poll_id, include_voters)
            .await
            .map_err(internal)?
            .ok_or(PollError::NotFound)?;
        let _ = principal;
        self.set_results_cache(&cache_key, &results).await;
        Ok(results)
    }

    // ------------------------------------------------------------------
    // Author / staff management
    // ------------------------------------------------------------------

    pub async fn update(
        &self,
        principal: &AuthenticatedPrincipal,
        poll_id: Uuid,
        request: UpdatePollRequest,
    ) -> Result<PollDetail, PollError> {
        let poll = self
            .repository
            .find_by_id(poll_id)
            .await
            .map_err(internal)?
            .ok_or(PollError::NotFound)?;
        self.require_manager(principal, &poll)?;
        if poll.status == "closed" {
            return Err(PollError::PollClosed);
        }

        let title = request.title.map(normalize_title).transpose()?;
        let description = match request.description {
            None => None,
            Some(value) => Some(normalize_description(Some(value))?),
        };
        let expires_at = match request.expires_at {
            None => None,
            Some(None) => Some(None),
            Some(Some(value)) => {
                if value <= Utc::now() {
                    return Err(PollError::Validation("截止时间必须晚于当前时间"));
                }
                Some(Some(value))
            }
        };

        // --- Option edits -------------------------------------------------
        let existing_options = self
            .repository
            .list_options(poll_id)
            .await
            .map_err(internal)?;
        let options_to_add = normalize_options_to_add(request.options_to_add)?;
        let option_ids_to_remove = dedupe_options(request.option_ids_to_remove.clone());
        if existing_options.len() + options_to_add.len() > MAX_OPTIONS {
            return Err(PollError::Validation("投票最多支持 20 个选项"));
        }
        // Removing options that already received votes would corrupt results.
        if !option_ids_to_remove.is_empty() {
            let by_id = existing_options
                .iter()
                .map(|option| (option.id, option.vote_count))
                .collect::<std::collections::HashMap<_, _>>();
            for option_id in &option_ids_to_remove {
                match by_id.get(option_id) {
                    None => {
                        return Err(PollError::Validation("选项不存在或不属于该投票"));
                    }
                    Some(votes) if *votes > 0 => {
                        return Err(PollError::Validation("已有票数的选项无法删除"));
                    }
                    _ => {}
                }
            }
        }

        if title.is_none()
            && description.is_none()
            && expires_at.is_none()
            && request.allow_cancel.is_none()
            && options_to_add.is_empty()
            && option_ids_to_remove.is_empty()
        {
            return Err(PollError::Validation("投票更新不包含任何字段"));
        }

        self.repository
            .update_with_options(
                poll_id,
                title.as_deref(),
                description.as_ref().map(|inner| inner.as_deref()),
                expires_at,
                request.allow_cancel,
                &options_to_add,
                &option_ids_to_remove,
            )
            .await
            .map_err(map_update_error)?;
        self.after_change(poll_id, poll.topic_id, "update", 0, 0)
            .await;
        self.get_by_id(Some(principal), poll_id).await
    }

    /// Close a poll — author or staff (PERMISSION_POLL_MANAGE).
    pub async fn close(
        &self,
        principal: &AuthenticatedPrincipal,
        poll_id: Uuid,
    ) -> Result<PollDetail, PollError> {
        let poll = self
            .repository
            .find_by_id(poll_id)
            .await
            .map_err(internal)?
            .ok_or(PollError::NotFound)?;
        self.require_manager(principal, &poll)?;
        self.repository.close(poll_id).await.map_err(internal)?;
        self.after_change(poll_id, poll.topic_id, "close", 0, 0)
            .await;
        self.get_by_id(Some(principal), poll_id).await
    }

    /// Admin-only hard delete of a violating poll.
    pub async fn delete(
        &self,
        principal: &AuthenticatedPrincipal,
        poll_id: Uuid,
    ) -> Result<(), PollError> {
        require(principal, PERMISSION_ADMIN_ACCESS)?;
        let poll = self
            .repository
            .find_by_id(poll_id)
            .await
            .map_err(internal)?
            .ok_or(PollError::NotFound)?;
        let deleted = self.repository.delete(poll_id).await.map_err(internal)?;
        if !deleted {
            return Err(PollError::NotFound);
        }
        self.invalidate_results_cache(poll_id).await;
        self.invalidate_hot_cache().await;
        self.realtime
            .publish_poll_update(
                poll_id,
                json!({
                    "event": "deleted",
                    "poll_id": poll_id,
                    "topic_id": poll.topic_id,
                }),
            )
            .await;
        Ok(())
    }

    // ------------------------------------------------------------------
    // Hot polls (cached)
    // ------------------------------------------------------------------

    pub async fn hot(&self) -> Result<Vec<HotPollItem>, PollError> {
        if let Some(cached) = self.cached_hot().await {
            return Ok(cached);
        }
        let items = self
            .repository
            .hot(HOT_POLLS_LIMIT)
            .await
            .map_err(internal)?;
        self.set_hot_cache(&items).await;
        Ok(items)
    }

    // ------------------------------------------------------------------
    // Admin
    // ------------------------------------------------------------------

    pub async fn list_admin(
        &self,
        principal: &AuthenticatedPrincipal,
        query: crate::models::AdminPollListQuery,
    ) -> Result<crate::models::Paginated<crate::models::AdminPollItem>, PollError> {
        require(principal, PERMISSION_ADMIN_ACCESS)?;
        let page = query.page.unwrap_or(1);
        let page_size = query.page_size.unwrap_or(20);
        if page == 0 || page > 1_000_000 {
            return Err(PollError::Validation("page is out of range"));
        }
        if page_size == 0 || page_size > 100 {
            return Err(PollError::Validation("page size must be between 1 and 100"));
        }
        let q = query
            .q
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty());
        let status = query
            .status
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty());
        if let Some(status) = status.as_deref() {
            if !matches!(status, "active" | "closed") {
                return Err(PollError::Validation("invalid poll status filter"));
            }
        }
        let offset = i64::from(page - 1) * i64::from(page_size);
        let (items, total) = self
            .repository
            .list_admin(
                q.as_deref(),
                status.as_deref(),
                i64::from(page_size),
                offset,
            )
            .await
            .map_err(internal)?;
        let total =
            u64::try_from(total).map_err(|_| internal(anyhow::anyhow!("negative poll count")))?;
        Ok(crate::models::Paginated {
            items,
            pagination: crate::models::PaginationMeta::new(page, page_size, total),
        })
    }

    // ------------------------------------------------------------------
    // Expiry maintenance
    // ------------------------------------------------------------------

    /// Background sweep: close expired polls, notify authors, invalidate caches.
    pub async fn run_expiry_maintenance(&self) -> Result<usize, PollError> {
        let now = Utc::now();
        let expired = self.repository.close_expired(now).await.map_err(internal)?;
        for poll_id in &expired {
            self.invalidate_results_cache(*poll_id).await;
            if let Ok(Some(poll)) = self.repository.find_by_id(*poll_id).await {
                if let Ok(Some((slug, title))) = self.repository.topic_meta(poll.topic_id).await {
                    let _ = self
                        .notifications
                        .handle_event(NotificationEvent::PollEnded(PollEndedEvent {
                            recipient_id: poll.author_id,
                            poll_id: poll.id,
                            topic_id: poll.topic_id,
                            topic_slug: slug,
                            topic_title: title,
                            poll_title: poll.title,
                        }))
                        .await;
                    self.realtime
                        .publish_poll_update(
                            poll.id,
                            json!({
                                "event": "ended",
                                "poll_id": poll.id,
                                "topic_id": poll.topic_id,
                            }),
                        )
                        .await;
                }
            }
        }
        if !expired.is_empty() {
            self.invalidate_hot_cache().await;
        }
        Ok(expired.len())
    }

    // ------------------------------------------------------------------
    // Helpers
    // ------------------------------------------------------------------

    fn ensure_votable(&self, poll: &PollRecord) -> Result<(), PollError> {
        if poll.status == "closed" {
            return Err(PollError::PollClosed);
        }
        if let Some(expires_at) = poll.expires_at {
            if expires_at <= Utc::now() {
                return Err(PollError::PollExpired);
            }
        }
        Ok(())
    }

    fn require_manager(
        &self,
        principal: &AuthenticatedPrincipal,
        poll: &PollRecord,
    ) -> Result<(), PollError> {
        let allowed =
            principal.user_id == poll.author_id || principal.has_permission(PERMISSION_POLL_MANAGE);
        if allowed {
            Ok(())
        } else {
            Err(PollError::Forbidden)
        }
    }

    async fn after_change(
        &self,
        poll_id: Uuid,
        topic_id: Uuid,
        event: &str,
        total_votes: i64,
        participants: i64,
    ) {
        self.invalidate_results_cache(poll_id).await;
        self.invalidate_hot_cache().await;
        self.realtime
            .publish_poll_update(
                poll_id,
                json!({
                    "event": event,
                    "poll_id": poll_id,
                    "topic_id": topic_id,
                    "total_votes": total_votes,
                    "participants": participants,
                }),
            )
            .await;
    }

    async fn cached_results(&self, key: &str) -> Option<PollResults> {
        let mut redis = self.redis.clone();
        redis
            .get::<_, Option<Vec<u8>>>(key)
            .await
            .ok()
            .flatten()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
    }

    async fn set_results_cache(&self, key: &str, results: &PollResults) {
        let mut redis = self.redis.clone();
        let Ok(payload) = serde_json::to_vec(results) else {
            return;
        };
        let _ = redis
            .set_ex::<_, _, ()>(key, payload, RESULTS_CACHE_TTL_SECS)
            .await;
    }

    async fn invalidate_results_cache(&self, poll_id: Uuid) {
        let mut redis = self.redis.clone();
        let _ = redis
            .del::<_, ()>(format!("{RESULTS_CACHE_PREFIX}{poll_id}"))
            .await;
    }

    async fn cached_hot(&self) -> Option<Vec<HotPollItem>> {
        let mut redis = self.redis.clone();
        redis
            .get::<_, Option<Vec<u8>>>(HOT_CACHE_KEY)
            .await
            .ok()
            .flatten()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
    }

    async fn set_hot_cache(&self, items: &[HotPollItem]) {
        let mut redis = self.redis.clone();
        let Ok(payload) = serde_json::to_vec(items) else {
            return;
        };
        let _ = redis
            .set_ex::<_, _, ()>(HOT_CACHE_KEY, payload, HOT_CACHE_TTL_SECS)
            .await;
    }

    async fn invalidate_hot_cache(&self) {
        let mut redis = self.redis.clone();
        let _ = redis.del::<_, ()>(HOT_CACHE_KEY).await;
    }
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

struct NormalizedDraft {
    title: String,
    description: Option<String>,
    multiple_choice: bool,
    anonymous: bool,
    allow_cancel: bool,
    max_choices: i32,
    expires_at: Option<DateTime<Utc>>,
    options: Vec<String>,
}

fn normalize_draft(draft: CreatePollDraft) -> Result<NormalizedDraft, PollError> {
    let title = normalize_title(draft.title)?;
    let description = normalize_description(draft.description)?;
    let multiple_choice = draft.multiple_choice.unwrap_or(false);
    let anonymous = draft.anonymous.unwrap_or(false);
    let allow_cancel = draft.allow_cancel.unwrap_or(true);

    let mut options = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for raw in draft.options {
        let content = raw.trim().to_owned();
        if content.is_empty() {
            continue;
        }
        if content.chars().count() > MAX_OPTION_CHARS {
            return Err(PollError::Validation("选项内容不能超过 500 个字符"));
        }
        let normalized = normalize_option(&content);
        if seen.insert(normalized.clone()) {
            options.push(normalized);
        }
    }
    if options.len() < MIN_OPTIONS {
        return Err(PollError::Validation("投票至少需要 2 个选项"));
    }
    if options.len() > MAX_OPTIONS {
        return Err(PollError::Validation("投票最多支持 20 个选项"));
    }

    let max_choices = if multiple_choice {
        let requested = draft.max_choices.unwrap_or(1);
        if !(1..=20).contains(&requested) {
            return Err(PollError::Validation("max_choices 必须在 1 到 20 之间"));
        }
        requested.min(options.len() as i32)
    } else {
        1
    };

    let expires_at = match draft.expires_at {
        None => None,
        Some(value) => {
            if value <= Utc::now() {
                return Err(PollError::Validation("截止时间必须晚于当前时间"));
            }
            Some(value)
        }
    };

    Ok(NormalizedDraft {
        title,
        description,
        multiple_choice,
        anonymous,
        allow_cancel,
        max_choices,
        expires_at,
        options,
    })
}

fn normalize_title(title: String) -> Result<String, PollError> {
    let title = title.trim().to_owned();
    let length = title.chars().count();
    if !(1..=200).contains(&length) {
        return Err(PollError::Validation("投票标题必须在 1 到 200 个字符之间"));
    }
    Ok(title)
}

fn normalize_description(description: Option<String>) -> Result<Option<String>, PollError> {
    match description {
        None => Ok(None),
        Some(value) => {
            let value = value.trim().to_owned();
            if value.is_empty() {
                return Ok(None);
            }
            if value.chars().count() > 2000 {
                return Err(PollError::Validation("投票描述不能超过 2000 个字符"));
            }
            Ok(Some(value))
        }
    }
}

fn normalize_option(content: &str) -> String {
    // Collapse internal whitespace runs for clean display + dedup.
    content.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn normalize_options_to_add(options: Vec<String>) -> Result<Vec<String>, PollError> {
    let mut normalized = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for raw in options {
        let content = raw.trim().to_owned();
        if content.is_empty() {
            continue;
        }
        if content.chars().count() > MAX_OPTION_CHARS {
            return Err(PollError::Validation("选项内容不能超过 500 个字符"));
        }
        let value = normalize_option(&content);
        if seen.insert(value.clone()) {
            normalized.push(value);
        }
    }
    Ok(normalized)
}

fn dedupe_options(option_ids: Vec<Uuid>) -> Vec<Uuid> {
    let mut seen = std::collections::HashSet::new();
    option_ids
        .into_iter()
        .filter(|id| seen.insert(*id))
        .collect()
}

// ---------------------------------------------------------------------------
// Conversions
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn to_detail(
    poll: &PollRecord,
    topic_slug: &str,
    topic_title: &str,
    options: Vec<PollOptionItem>,
    total_votes: i64,
    participants: i64,
    my_votes: Vec<Uuid>,
    can_vote: bool,
    can_manage: bool,
) -> PollDetail {
    let _ = can_vote;
    PollDetail {
        id: poll.id,
        topic_id: poll.topic_id,
        topic_slug: topic_slug.to_owned(),
        topic_title: topic_title.to_owned(),
        author_id: poll.author_id,
        title: poll.title.clone(),
        description: poll.description.clone(),
        poll_type: crate::models::PollType::Standard,
        status: if poll.status == "closed" {
            PollStatus::Closed
        } else {
            PollStatus::Active
        },
        multiple_choice: poll.multiple_choice,
        anonymous: poll.anonymous,
        allow_cancel: poll.allow_cancel,
        max_choices: poll.max_choices,
        expires_at: poll.expires_at,
        created_at: poll.created_at,
        updated_at: poll.updated_at,
        options,
        total_votes,
        participant_count: participants,
        my_votes,
        can_vote,
        can_manage,
    }
}

fn require(principal: &AuthenticatedPrincipal, permission: &'static str) -> Result<(), PollError> {
    if principal.has_permission(permission) {
        Ok(())
    } else {
        Err(PollError::Forbidden)
    }
}

fn internal(error: impl std::fmt::Display + std::fmt::Debug + Send + Sync + 'static) -> PollError {
    PollError::Internal(anyhow::anyhow!("{error}"))
}

fn map_moderation(error: crate::services::ModerationError) -> PollError {
    match error {
        crate::services::ModerationError::Validation(message) => PollError::Validation(message),
        crate::services::ModerationError::Forbidden => PollError::Forbidden,
        crate::services::ModerationError::RateLimited => {
            PollError::Validation("内容发布过于频繁，请稍后再试")
        }
        _ => PollError::Internal(anyhow::anyhow!("{error}")),
    }
}

fn map_update_error(error: PollUpdateError) -> PollError {
    match error {
        PollUpdateError::NotFound => PollError::NotFound,
        PollUpdateError::OptionHasVotes => PollError::Validation("已有票数的选项无法删除"),
        PollUpdateError::UnknownOption => PollError::Validation("选项不存在或不属于该投票"),
        PollUpdateError::Database(error) => internal(error),
    }
}

fn is_unique_violation(error: &sqlx::Error) -> bool {
    matches!(
        error,
        sqlx::Error::Database(db_error) if db_error.is_unique_violation()
    )
}
