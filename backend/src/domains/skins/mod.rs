pub mod handlers;
pub mod types;

use axum::Router;
use axum::routing::{get, post};

pub fn router() -> Router<crate::app::state::SharedAppState> {
    Router::new()
        .route("/api/skins/me", post(handlers::upload_my_skin))
        .route("/api/skins/default", get(handlers::get_default_skin))
        .route("/api/skins/{uuid}", get(handlers::get_skin))
}
