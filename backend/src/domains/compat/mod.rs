pub mod gamervii;

use axum::routing::post;
use axum::Router;

pub fn router() -> Router<crate::app::state::SharedAppState> {
    Router::new().route("/api/compat/gamervii/auth", post(gamervii::gamervii_auth))
}
