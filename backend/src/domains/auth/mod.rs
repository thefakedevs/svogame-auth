pub mod handlers;
pub mod polling;
pub mod runtime;
pub mod verification;

use axum::Router;
use axum::routing::{get, post};

pub fn router() -> Router<crate::app::state::SharedAppState> {
    Router::new()
        .route("/api/auth/prepare", post(handlers::prepare_auth))
        .route("/api/auth/authorize", post(handlers::authorize))
        .route("/api/auth/register", post(handlers::register))
        .route("/api/auth/poll/{poll_id}", get(polling::poll_auth_status))
        .route("/api/auth/verify", get(verification::verify))
}
