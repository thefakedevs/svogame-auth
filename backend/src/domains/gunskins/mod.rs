pub mod handlers;

use axum::Router;
use axum::routing::{get, put};

pub fn router() -> Router<crate::app::state::SharedAppState> {
    Router::new()
        .route(
            "/api/user/me/gunskins/selections",
            get(handlers::list_my_gunskin_selections),
        )
        .route(
            "/api/user/me/gunskins/{weapon_key}",
            get(handlers::get_my_gunskin_collection),
        )
        .route(
            "/api/user/me/gunskins/{weapon_key}/selected",
            put(handlers::select_my_gunskin).delete(handlers::reset_my_gunskin),
        )
        .route(
            "/api/system/users/{user_id}/gunskins/{weapon_key}/selected",
            get(handlers::get_user_selected_gunskin)
                .put(handlers::select_user_gunskin)
                .delete(handlers::reset_user_gunskin),
        )
        .route(
            "/api/system/users/{user_id}/gunskins/{weapon_key}/available",
            get(handlers::list_user_available_gunskins),
        )
        .route("/api/admin/gunskins/keys", get(handlers::list_admin_weapon_keys))
}
