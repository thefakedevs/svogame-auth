pub mod handlers;

use axum::Router;
use axum::routing::{delete, get, post};

pub fn router() -> Router<crate::app::state::SharedAppState> {
    Router::new()
        .route("/api/lootboxes", get(handlers::list_public_lootboxes))
        .route(
            "/api/lootboxes/{lootbox_id}",
            get(handlers::get_public_lootbox),
        )
        .route("/api/user/me/lootboxes", get(handlers::get_my_lootboxes))
        .route(
            "/api/user/me/lootboxes/open-history",
            get(handlers::get_my_lootbox_open_history),
        )
        .route(
            "/api/user/me/lootboxes/{asset_key}/open",
            post(handlers::open_my_lootbox),
        )
        .route(
            "/api/admin/lootboxes",
            get(handlers::list_admin_lootboxes).post(handlers::create_lootbox),
        )
        .route(
            "/api/admin/lootboxes/{lootbox_id}",
            get(handlers::get_admin_lootbox).patch(handlers::patch_lootbox),
        )
        .route(
            "/api/admin/lootboxes/{lootbox_id}/drops",
            post(handlers::create_lootbox_drop),
        )
        .route(
            "/api/admin/lootboxes/{lootbox_id}/drops/{drop_id}",
            delete(handlers::delete_lootbox_drop).patch(handlers::patch_lootbox_drop),
        )
        .route(
            "/api/admin/users/{user_id}/lootboxes/{asset_key}/open",
            post(handlers::open_user_lootbox),
        )
        .route(
            "/api/admin/users/{user_id}/lootboxes/open-history",
            get(handlers::get_user_lootbox_open_history),
        )
        .route(
            "/api/admin/lootboxes/open-history",
            get(handlers::get_all_lootbox_open_history),
        )
}
