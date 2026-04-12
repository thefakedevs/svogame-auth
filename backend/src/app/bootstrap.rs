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
    services::ownership::catalog::ensure_system_assets(&db).await?;
    let s3 = build_s3_client(&config).await?;
    verify_s3_access(&config, &s3).await?;

    let state: SharedAppState = Arc::new(RwLock::new(AppState::new(config.clone(), db, s3)));
    spawn_auth_cleanup_worker(state.clone());
    spawn_shop_reconciliation_worker(state.clone());

    let app = build_router(state);
    let listener = TcpListener::bind(&config.binding_address).await?;
    info!("API server listening on http://{}", listener.local_addr()?);
    log_frontend_dev_target();

    axum::serve(listener, app.into_make_service()).await?;
    unreachable!()
}

async fn build_s3_client(config: &AppConfig) -> Result<aws_sdk_s3::Client> {
    let s3_config = &config.s3;
    let credentials = aws_credential_types::Credentials::new(
        s3_config.access_key_id.clone(),
        s3_config.secret_access_key.clone(),
        None,
        None,
        "app-config",
    );

    let shared_config = aws_config::defaults(aws_config::BehaviorVersion::latest())
        .region(aws_config::Region::new(s3_config.region.clone()))
        .credentials_provider(credentials)
        .load()
        .await;

    let mut builder = aws_sdk_s3::config::Builder::from(&shared_config)
        .force_path_style(s3_config.force_path_style);

    if let Some(endpoint) = &s3_config.endpoint {
        builder = builder.endpoint_url(endpoint);
    }

    Ok(aws_sdk_s3::Client::from_conf(builder.build()))
}

async fn verify_s3_access(config: &AppConfig, s3: &aws_sdk_s3::Client) -> Result<()> {
    if let Err(head_error) = s3.head_bucket().bucket(&config.s3.bucket).send().await {
        warn!(
            "S3 bucket '{}' is not accessible yet, trying to create it: {}",
            config.s3.bucket, head_error
        );

        s3.create_bucket()
            .bucket(&config.s3.bucket)
            .send()
            .await
            .map_err(|create_error| {
                anyhow::anyhow!(
                    "Failed to access S3 bucket '{}' and failed to create it: head error: {}; create error: {}",
                    config.s3.bucket,
                    head_error,
                    create_error
                )
            })?;

        s3.head_bucket()
            .bucket(&config.s3.bucket)
            .send()
            .await
            .map_err(|retry_error| {
                anyhow::anyhow!(
                    "S3 bucket '{}' was created but is still not accessible: {}",
                    config.s3.bucket,
                    retry_error
                )
            })?;
    }

    info!(
        "S3 connection established for bucket '{}'",
        config.s3.bucket
    );
    Ok(())
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

fn spawn_shop_reconciliation_worker(state: SharedAppState) {
    tokio::spawn(async move {
        let interval_seconds = {
            let state_guard = state.read().await;
            state_guard.config.shop.reconciliation_interval_seconds
        };
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(interval_seconds));
        loop {
            interval.tick().await;
            let (db, shop_config) = {
                let state_guard = state.read().await;
                (state_guard.db.clone(), state_guard.config.shop.clone())
            };
            if let Err(error) =
                crate::services::shop::reconcile_pending_orders(&db, &shop_config).await
            {
                warn!("Shop reconciliation failed: {error}");
            }
        }
    });
}

fn log_frontend_dev_target() {
    #[cfg(debug_assertions)]
    {
        let frontend_host =
            std::env::var("FRONTEND_DEV_HOST").unwrap_or_else(|_| "localhost".to_string());
        let frontend_port = std::env::var("DEBUG_PORT")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .or_else(|| std::env::var("FRONTEND_DEV_PORT").ok())
            .unwrap_or_else(|| "5173".to_string());

        info!(
            "Frontend dev server expected at http://{}:{}",
            frontend_host, frontend_port
        );
    }
}
