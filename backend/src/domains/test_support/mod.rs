pub mod handlers;

use axum::Router;
use axum::routing::post;

pub fn router() -> Router<crate::app::state::SharedAppState> {
    Router::new().route("/api/test/issue-token", post(handlers::issue_token))
}
