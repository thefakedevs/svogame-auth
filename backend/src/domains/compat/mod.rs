pub mod gamervii;

use axum::Router;
use axum::routing::post;

pub fn router() -> Router<crate::app::state::SharedAppState> {
    Router::new().route("/api/compat/gamervii/auth", post(gamervii::gamervii_auth))
}
