pub mod handlers;

use axum::Router;
use axum::routing::{get, post};

pub fn router() -> Router<crate::app::state::SharedAppState> {
    Router::new()
        .route("/api/users/search", get(handlers::search_users))
        .route("/api/user/me", get(handlers::get_me))
        .route("/api/user/me/nickname", post(handlers::update_nickname))
        .route(
            "/api/user/me/restrictions",
            get(handlers::get_my_restrictions),
        )
}
