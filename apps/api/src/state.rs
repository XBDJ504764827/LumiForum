use std::sync::Arc;

use redis::aio::ConnectionManager;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

use crate::config::Config;
use crate::repositories::{
    AuthRepository, AuthorizationRepository, CategoryRepository, CommentRepository,
    NotificationRepository, ReactionRepository, SearchRepository, TopicRepository,
    UploadRepository, UserRepository,
};
use crate::services::{
    AuthService, AuthServiceConfig, AuthorizationService, CategoryService, CommentService,
    NotificationService, ReactionService, SearchService, TopicService, UploadService, UserService,
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
    pub users: UserService,
    pub authorization: AuthorizationService,
    pub categories: CategoryService,
    pub topics: TopicService,
    pub comments: CommentService,
    pub reactions: ReactionService,
    pub notifications: NotificationService,
    pub search: SearchService,
    pub uploads: UploadService,
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
        let users = UserService::new(UserRepository::new(db.clone()));
        let authorization = AuthorizationService::new(
            AuthorizationRepository::new(db.clone()),
            redis.clone(),
            config.authorization_cache_ttl_seconds,
        )?;
        let category_repository = CategoryRepository::new(db.clone());
        let topic_repository = TopicRepository::new(db.clone());
        let categories = CategoryService::new(category_repository.clone());
        let topics = TopicService::new(topic_repository.clone(), category_repository);
        let notification_repository = NotificationRepository::new(db.clone());
        let notifications =
            NotificationService::new(notification_repository.clone(), redis.clone());
        let comments = CommentService::new(
            CommentRepository::new(db.clone()),
            topic_repository,
            notifications.clone(),
            notification_repository.clone(),
            redis.clone(),
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
        let uploads = UploadService::new(UploadRepository::new(db.clone()), storage);

        Ok(Self {
            inner: Arc::new(AppStateInner {
                config,
                db,
                redis,
                auth,
                users,
                authorization,
                categories,
                topics,
                comments,
                reactions,
                notifications,
                search,
                uploads,
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
}
