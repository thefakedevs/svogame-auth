use sea_orm::DatabaseConnection;
use crate::state::config::AppConfig;

pub struct AppState {
    pub config: AppConfig,
    pub db: DatabaseConnection,
}

impl AppState {
    pub fn new(config: AppConfig, db: DatabaseConnection) -> Self {
        AppState { config, db }
    }
}