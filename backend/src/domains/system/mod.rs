pub mod health;

use axum::Router;
use axum::routing::get;

pub fn router() -> Router<crate::app::state::SharedAppState> {
    Router::new().route("/api/health", get(health::health))
}
