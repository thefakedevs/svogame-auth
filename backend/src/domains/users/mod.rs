pub mod handlers;

use axum::routing::{get, post};
use axum::Router;

pub fn router() -> Router<crate::app::state::SharedAppState> {
    Router::new()
        .route("/api/user/me", get(handlers::get_me))
        .route("/api/user/me/nickname", post(handlers::update_nickname))
}
