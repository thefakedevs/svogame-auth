pub mod handlers;

use axum::Router;
use axum::routing::get;

pub fn router() -> Router<crate::app::state::SharedAppState> {
    Router::new()
        .route("/api/referrals/{code}", get(handlers::get_referral))
        .route(
            "/api/referrals/me/campaigns",
            get(handlers::my_referral_campaigns),
        )
        .route("/api/referrals/me/stats", get(handlers::my_referral_stats))
}
