use std::sync::Arc;

use aws_sdk_s3::Client as S3Client;
use sea_orm::DatabaseConnection;
use tokio::sync::RwLock;

use crate::app::config::AppConfig;
use crate::domains::auth::runtime::AuthRuntime;

pub struct AppState {
    pub config: AppConfig,
    pub db: DatabaseConnection,
    pub s3: S3Client,
    pub auth: AuthRuntime,
}

impl AppState {
    pub fn new(config: AppConfig, db: DatabaseConnection, s3: S3Client) -> Self {
        Self {
            config,
            db,
            s3,
            auth: AuthRuntime::new(),
        }
    }
}

pub type SharedAppState = Arc<RwLock<AppState>>;
pub type AppStateExtractor = axum::extract::State<SharedAppState>;
