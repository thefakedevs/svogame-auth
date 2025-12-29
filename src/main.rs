mod routes;
mod state;
mod misc;
mod services;
mod entities;

use std::sync::Arc;
use axum::Router;
use axum::routing::post;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::{HttpMakeClassifier, TraceLayer};
use tokio::net::TcpListener;
use tower_http::trace;
use tracing::{error, info};
use crate::state::config::AppConfig;
use anyhow::Result;
use tokio::sync::RwLock;
use crate::state::AppState;

#[tokio::main]
async fn main() -> Result<()> {
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
    let app = Router::new()
        .route("/api/auth/prepare", post(routes::prepare_auth))
        .route("/api/auth/authorize", post(routes::authorize))
        .layer(cors)
        .layer(tracing_layer)
        .with_state(state);

    let listener = TcpListener::bind(config.binding_address).await?;
    info!("API server listening on http://{}", listener.local_addr()?);

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