mod routes;
mod state;
mod misc;
mod services;
mod entities;
mod docs;

use std::sync::Arc;
use axum::Router;
use axum::routing::{get, post};
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::{HttpMakeClassifier, TraceLayer};
use tokio::net::TcpListener;
use tower_http::trace;
use tracing::{error, info};
use crate::state::config::AppConfig;
use anyhow::Result;
use tokio::sync::RwLock;
use crate::state::AppState;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

#[tokio::main]
async fn main() -> Result<()> {
    // Загрузка .env файла для локальной разработки
    #[cfg(debug_assertions)]
    {
        if let Err(e) = dotenvy::dotenv() {
            eprintln!("Warning: .env file not loaded: {}", e);
        }
    }

    let config = AppConfig::from_env()?;

    #[cfg(debug_assertions)]
    error!("Running in debug mode. This is NOT recommended for production!");

    let tracing_layer = prepare_tracing();

    let db = services::db::connect_db(&config.database).await?;
    services::db::run_migrations(&db).await?;

    // CORS layer to allow requests from frontend on different port
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let state = Arc::new(RwLock::new(AppState::new(config.clone(), db)));

    {
        let state_clone = state.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
            loop {
                interval.tick().await;
                let state_guard = state_clone.read().await;
                state_guard.cleanup_auth_cache().await;
            }
        });
    }

    let app = Router::new()
        .merge(SwaggerUi::new("/api/docs").url("/api/openapi.json", docs::ApiDoc::openapi()))
        .route("/api/auth/prepare", post(routes::prepare_auth))
        .route("/api/auth/authorize", post(routes::authorize))
        .route("/api/auth/poll/{poll_id}", get(routes::poll_auth_status))
        .route("/api/auth/verify", get(routes::verify))
        .route("/api/compat/gamervii/auth", post(routes::gamervii_auth))
        .route("/api/user/me", get(routes::get_me))
        .route("/api/user/me/nickname", post(routes::update_nickname))
        .layer(cors)
        .layer(tracing_layer)
        .with_state(state);

    let listener = TcpListener::bind(&config.binding_address).await?;
    info!("API server listening on http://{}", listener.local_addr()?);

    // В debug режиме запускаем Vite dev server
    #[cfg(debug_assertions)]
    {
        use std::process::Command;

        let npm_cmd = if cfg!(target_os = "windows") { "npm.cmd" } else { "npm" };
        let api_target = format!("http://{}", config.binding_address);

        match Command::new(npm_cmd)
            .args(["run", "dev"])
            .current_dir("app")
            .env("VITE_API_TARGET", &api_target)
            .spawn()
        {
            Ok(_child) => {
                info!("Vite dev server started. Frontend available at http://localhost:5173");
                info!("API proxy target: {}", api_target);
            }
            Err(e) => {
                error!("Failed to start Vite dev server: {}. Run 'cd app && npm run dev' manually.", e);
            }
        }
    }

    axum::serve(listener, app.into_make_service())
        .await?;

    unreachable!()
}

fn prepare_tracing() -> TraceLayer<HttpMakeClassifier> {
    tracing_subscriber::fmt()
        .with_target(false)
        .with_timer(tracing_subscriber::fmt::time::uptime())
        .with_level(true)
        .with_env_filter("info,tower_http=debug")
        .init();

    TraceLayer::new_for_http()
        .make_span_with(trace::DefaultMakeSpan::new().level(tracing::Level::INFO))
        .on_response(trace::DefaultOnResponse::new().level(tracing::Level::INFO))
        .on_failure(trace::DefaultOnFailure::new().level(tracing::Level::ERROR))
}