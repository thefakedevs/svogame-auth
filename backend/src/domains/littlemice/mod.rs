pub mod handlers;

use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::routing::{get, post};

const MAX_PUSH_BODY_BYTES: usize = 80 * 1024 * 1024;

pub fn router() -> Router<crate::app::state::SharedAppState> {
    Router::new()
        .route(
            "/api/littlemice/checks",
            post(handlers::create_check),
        )
        .route(
            "/api/littlemice/push/{push_token}",
            post(handlers::push_check),
        )
        .route(
            "/api/littlemice/checks/{check_id}",
            get(handlers::get_check_status),
        )
        .route(
            "/api/littlemice/checks/{check_id}/fail",
            post(handlers::fail_check),
        )
        .layer(DefaultBodyLimit::max(MAX_PUSH_BODY_BYTES))
}
