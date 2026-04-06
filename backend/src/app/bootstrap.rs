use std::sync::Arc;

use anyhow::Result;
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use tracing::{info, warn};

use crate::app::config::AppConfig;
use crate::app::router::build_router;
use crate::app::state::{AppState, SharedAppState};
use crate::services;

pub async fn run() -> Result<()> {
    load_dotenv_for_local_dev();
    let config = AppConfig::from_env()?;

    #[cfg(debug_assertions)]
    warn!("Running in debug mode. This is NOT recommended for production!");

    let db = services::db::connect_db(&config.database).await?;
    services::db::run_migrations(&db).await?;

    let state: SharedAppState = Arc::new(RwLock::new(AppState::new(config.clone(), db)));
    spawn_auth_cleanup_worker(state.clone());

    let app = build_router(state);
    let listener = TcpListener::bind(&config.binding_address).await?;
    info!("API server listening on http://{}", listener.local_addr()?);
    log_frontend_dev_target();

    axum::serve(listener, app.into_make_service()).await?;
    unreachable!()
}

fn load_dotenv_for_local_dev() {
    #[cfg(debug_assertions)]
    {
        if let Err(e) = dotenvy::dotenv() {
            eprintln!("Warning: .env file not loaded: {}", e);
        }
    }
}

fn spawn_auth_cleanup_worker(state: SharedAppState) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
        loop {
            interval.tick().await;
            let state_guard = state.read().await;
            state_guard.auth.cleanup_cache().await;
        }
    });
}

fn log_frontend_dev_target() {
    #[cfg(debug_assertions)]
    {
        let frontend_host =
            std::env::var("FRONTEND_DEV_HOST").unwrap_or_else(|_| "localhost".to_string());
        let frontend_port =
            std::env::var("FRONTEND_DEV_PORT").unwrap_or_else(|_| "5173".to_string());

        info!(
            "Frontend dev server expected at http://{}:{}",
            frontend_host, frontend_port
        );
    }
}
