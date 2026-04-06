pub mod health;

use axum::routing::get;
use axum::Router;

pub fn router() -> Router<crate::app::state::SharedAppState> {
    Router::new().route("/api/health", get(health::health))
}
