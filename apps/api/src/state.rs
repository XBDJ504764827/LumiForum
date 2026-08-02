use std::sync::Arc;

use redis::aio::ConnectionManager;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

use crate::config::Config;
use crate::realtime::{PresenceService, RealtimeBus, RealtimeHub};
use crate::repositories::{
    AdminRepository, AuthRepository, AuthorizationRepository, CategoryRepository,
    CommentRepository, ModerationRepository, NotificationRepository, PollRepository,
    ReactionRepository, SearchRepository, SteamAuthRepository, TopicRepository, UploadRepository,
    UserRepository,
};
use crate::services::{
    AdminService, AuthService, AuthServiceConfig, AuthorizationService, CategoryService,
    CommentService, MetricsRegistry, ModerationService, NotificationService, PollService,
    ReactionService, SearchService, SteamAuthService, SteamOpenIdClient, TopicService,
    UploadService, UserService,
};
use crate::storage::{LocalStorage, S3Storage, S3StorageConfig, StorageProvider};

#[derive(Clone)]
pub struct AppState {
    inner: Arc<AppStateInner>,
}

struct AppStateInner {
    pub config: Config,
    pub db: PgPool,
    pub redis: ConnectionManager,
    pub auth: AuthService,
    pub steam_auth: Option<SteamAuthService>,
    pub users: UserService,
    pub authorization: AuthorizationService,
    pub categories: CategoryService,
    pub topics: TopicService,
    pub polls: PollService,
    pub comments: CommentService,
    pub reactions: ReactionService,
    pub notifications: NotificationService,
    pub search: SearchService,
    pub uploads: UploadService,
    pub admin: AdminService,
    pub moderation: ModerationService,
    pub metrics: MetricsRegistry,
    pub realtime: RealtimeBus,
    pub presence: PresenceService,
}

impl AppState {
    pub async fn new(config: Config) -> anyhow::Result<Self> {
        let db = PgPoolOptions::new()
            .max_connections(10)
            .connect(&config.database_url)
            .await?;
        sqlx::migrate!("./migrations").run(&db).await?;

        let redis_client = redis::Client::open(config.redis_url.as_str())?;
        let redis = ConnectionManager::new(redis_client).await?;
        let auth = AuthService::new(
            AuthRepository::new(db.clone()),
            AuthServiceConfig {
                jwt_secret: config.jwt_secret.clone(),
                jwt_issuer: config.jwt_issuer.clone(),
                jwt_audience: config.jwt_audience.clone(),
                access_token_ttl_seconds: config.access_token_ttl_seconds,
                refresh_token_ttl_seconds: config.refresh_token_ttl_seconds,
                password_hash_concurrency: config.password_hash_concurrency,
            },
        )?;
        let steam_auth = match (
            config.steam_api_key.clone(),
            config.steam_openid_realm.clone(),
            config.steam_return_url.clone(),
        ) {
            (Some(api_key), Some(realm), Some(return_url)) => Some(SteamAuthService::new(
                SteamAuthRepository::new(db.clone()),
                auth.clone(),
                config.password_hash_concurrency,
                redis.clone(),
                SteamOpenIdClient::new(
                    api_key,
                    realm,
                    return_url,
                    config.steam_proxy_url.clone(),
                    config.steam_http_timeout_seconds,
                )?,
            )?),
            _ => None,
        };
        let users = UserService::new(UserRepository::new(db.clone()));
        let authorization = AuthorizationService::new(
            AuthorizationRepository::new(db.clone()),
            redis.clone(),
            config.authorization_cache_ttl_seconds,
        )?;
        let category_repository = CategoryRepository::new(db.clone());
        let topic_repository = TopicRepository::new(db.clone());
        let categories = CategoryService::new(category_repository.clone());
        let notification_repository = NotificationRepository::new(db.clone());
        let hub = RealtimeHub::new(config.ws_max_connections_per_user);
        let realtime = RealtimeBus::new(redis.clone(), config.redis_url.clone(), hub);
        realtime.clone().spawn_subscriber();
        realtime.clone().spawn_poll_subscriber();
        let presence = PresenceService::new(redis.clone(), config.presence_ttl_secs);
        let notifications = NotificationService::new(
            notification_repository.clone(),
            redis.clone(),
            realtime.clone(),
        );
        let admin_repository = AdminRepository::new(db.clone());
        let metrics = MetricsRegistry::new();
        let moderation = ModerationService::new(
            ModerationRepository::new(db.clone()),
            category_repository.clone(),
            notifications.clone(),
            admin_repository.clone(),
            authorization.clone(),
            realtime.clone(),
            redis.clone(),
            metrics.clone(),
        );
        let polls = PollService::new(
            PollRepository::new(db.clone()),
            topic_repository.clone(),
            moderation.clone(),
            notifications.clone(),
            realtime.clone(),
            redis.clone(),
        );
        let topics = TopicService::new(
            topic_repository.clone(),
            category_repository,
            moderation.clone(),
            polls.clone(),
        );
        let comments = CommentService::new(
            CommentRepository::new(db.clone()),
            topic_repository,
            notifications.clone(),
            notification_repository.clone(),
            redis.clone(),
            moderation.clone(),
        );
        let reactions = ReactionService::new(
            ReactionRepository::new(db.clone()),
            notifications.clone(),
            notification_repository,
            redis.clone(),
        );
        let search = SearchService::new(SearchRepository::new(db.clone()), redis.clone());
        let storage: Arc<dyn StorageProvider> = match config.storage_provider.as_str() {
            "local" => Arc::new(
                LocalStorage::new(
                    &config.storage_local_root,
                    config.storage_public_url.clone(),
                )
                .await?,
            ),
            "s3" => Arc::new(S3Storage::new(S3StorageConfig {
                endpoint: config.s3_endpoint.clone(),
                region: config.s3_region.clone(),
                bucket: config.s3_bucket.clone(),
                access_key: config.s3_access_key.clone(),
                secret_key: config.s3_secret_key.clone(),
                force_path_style: config.s3_force_path_style,
                public_url: config.s3_public_url.clone(),
            })?),
            _ => unreachable!("storage provider is validated by Config"),
        };
        let uploads = UploadService::with_moderation(
            UploadRepository::new(db.clone()),
            storage,
            moderation.clone(),
        );
        let admin = AdminService::new(
            AdminRepository::new(db.clone()),
            categories.clone(),
            comments.clone(),
            uploads.clone(),
            authorization.clone(),
        );

        let maintenance = moderation.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
            interval.tick().await; // first tick completes immediately; skip
            loop {
                interval.tick().await;
                match maintenance.run_maintenance().await {
                    Ok(summary) => {
                        if summary.expired_sanctions > 0 || summary.expiry_reminders > 0 {
                            tracing::info!(?summary, "moderation maintenance run");
                        }
                    }
                    Err(error) => {
                        tracing::warn!(%error, "moderation maintenance failed");
                    }
                }
            }
        });

        let poll_maintenance = polls.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
            interval.tick().await; // first tick completes immediately; skip
            loop {
                interval.tick().await;
                match poll_maintenance.run_expiry_maintenance().await {
                    Ok(closed) => {
                        if closed > 0 {
                            tracing::info!(closed, "poll expiry maintenance run");
                        }
                    }
                    Err(error) => {
                        tracing::warn!(%error, "poll expiry maintenance failed");
                    }
                }
            }
        });

        Ok(Self {
            inner: Arc::new(AppStateInner {
                config,
                db,
                redis,
                auth,
                steam_auth,
                users,
                authorization,
                categories,
                topics,
                polls,
                comments,
                reactions,
                notifications,
                search,
                uploads,
                admin,
                moderation,
                metrics,
                realtime,
                presence,
            }),
        })
    }

    pub fn config(&self) -> &Config {
        &self.inner.config
    }

    pub fn db(&self) -> &PgPool {
        &self.inner.db
    }

    pub fn redis(&self) -> &ConnectionManager {
        &self.inner.redis
    }

    pub fn auth(&self) -> &AuthService {
        &self.inner.auth
    }

    pub fn steam_auth(&self) -> Option<&SteamAuthService> {
        self.inner.steam_auth.as_ref()
    }

    pub fn users(&self) -> &UserService {
        &self.inner.users
    }

    pub fn authorization(&self) -> &AuthorizationService {
        &self.inner.authorization
    }

    pub fn categories(&self) -> &CategoryService {
        &self.inner.categories
    }

    pub fn topics(&self) -> &TopicService {
        &self.inner.topics
    }

    pub fn polls(&self) -> &PollService {
        &self.inner.polls
    }

    pub fn comments(&self) -> &CommentService {
        &self.inner.comments
    }

    pub fn reactions(&self) -> &ReactionService {
        &self.inner.reactions
    }

    pub fn notifications(&self) -> &NotificationService {
        &self.inner.notifications
    }

    pub fn search(&self) -> &SearchService {
        &self.inner.search
    }

    pub fn uploads(&self) -> &UploadService {
        &self.inner.uploads
    }

    pub fn admin(&self) -> &AdminService {
        &self.inner.admin
    }

    pub fn moderation(&self) -> &ModerationService {
        &self.inner.moderation
    }

    pub fn metrics(&self) -> &MetricsRegistry {
        &self.inner.metrics
    }

    pub fn realtime(&self) -> &RealtimeBus {
        &self.inner.realtime
    }

    pub fn presence(&self) -> &PresenceService {
        &self.inner.presence
    }
}
