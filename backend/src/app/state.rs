use std::sync::Arc;

use sea_orm::DatabaseConnection;
use tokio::sync::RwLock;

use crate::app::config::AppConfig;
use crate::domains::auth::runtime::AuthRuntime;

pub struct AppState {
    pub config: AppConfig,
    pub db: DatabaseConnection,
    pub auth: AuthRuntime,
}

impl AppState {
    pub fn new(config: AppConfig, db: DatabaseConnection) -> Self {
        Self {
            config,
            db,
            auth: AuthRuntime::new(),
        }
    }
}

pub type SharedAppState = Arc<RwLock<AppState>>;
pub type AppStateExtractor = axum::extract::State<SharedAppState>;
