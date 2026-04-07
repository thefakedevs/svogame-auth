pub mod handlers;

use axum::routing::{get, post};
use axum::Router;

pub fn router() -> Router<crate::app::state::SharedAppState> {
    Router::new()
        .route("/api/admin/health", get(handlers::health))
        .route("/api/admin/me", get(handlers::me))
        .route("/api/admin/users", get(handlers::list_users))
        .route("/api/admin/users/{user_id}", get(handlers::get_user).patch(handlers::patch_user))
        .route("/api/admin/users/{user_id}/deactivate", post(handlers::deactivate_user))
        .route("/api/admin/users/{user_id}/activate", post(handlers::activate_user))
        .route("/api/admin/users/{user_id}/reset-auth-epoch", post(handlers::reset_auth_epoch))
        .route("/api/admin/users/{user_id}/grant-superuser", post(handlers::grant_superuser))
        .route("/api/admin/users/{user_id}/revoke-superuser", post(handlers::revoke_superuser))
}
