pub mod handlers;

use axum::routing::get;
use axum::Router;

pub fn router() -> Router<crate::app::state::SharedAppState> {
    Router::new()
        .route("/api/meta/restrictions", get(handlers::get_restrictions_meta))
        .route("/api/meta/squads/config", get(handlers::get_squads_config))
}
