use std::sync::Arc;

use aws_sdk_s3::Client as S3Client;
use sea_orm::DatabaseConnection;
use tokio::sync::RwLock;

use crate::app::config::AppConfig;
use crate::domains::auth::runtime::AuthRuntime;
use crate::services::discord_events::DiscordEventsRuntime;

pub struct AppState {
    pub config: AppConfig,
    pub db: DatabaseConnection,
    pub s3: S3Client,
    pub auth: AuthRuntime,
    pub discord_events: DiscordEventsRuntime,
}

impl AppState {
    pub fn new(config: AppConfig, db: DatabaseConnection, s3: S3Client) -> Self {
        Self {
            config,
            db,
            s3,
            auth: AuthRuntime::new(),
            discord_events: DiscordEventsRuntime::new(),
        }
    }
}

pub type SharedAppState = Arc<RwLock<AppState>>;
pub type AppStateExtractor = axum::extract::State<SharedAppState>;
