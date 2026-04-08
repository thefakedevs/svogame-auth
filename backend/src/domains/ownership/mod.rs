pub mod handlers;

use axum::Router;
use axum::routing::{get, post};

pub fn router() -> Router<crate::app::state::SharedAppState> {
    Router::new()
        .route("/api/assets", get(handlers::list_public_assets))
        .route("/api/assets/{asset_id}", get(handlers::get_public_asset))
        .route("/api/user/me/inventory", get(handlers::get_my_inventory))
        .route(
            "/api/user/me/inventory/contains/{asset_key}",
            get(handlers::check_my_inventory_presence),
        )
        .route(
            "/api/user/me/inventory/stackables",
            get(handlers::get_my_stackables),
        )
        .route(
            "/api/user/me/inventory/entitlements",
            get(handlers::get_my_entitlements),
        )
        .route(
            "/api/user/me/inventory/expirables/active",
            get(handlers::get_my_active_expirables),
        )
        .route("/api/user/me/wallet", get(handlers::get_my_wallet))
        .route(
            "/api/user/me/wallet/{currency_key}",
            get(handlers::get_my_wallet_balance),
        )
        .route(
            "/api/user/me/wallet/{currency_key}/transactions",
            get(handlers::get_my_wallet_transactions),
        )
        .route("/api/admin/assets", get(handlers::list_admin_assets).post(handlers::create_asset))
        .route(
            "/api/admin/assets/{asset_id}",
            get(handlers::get_admin_asset).patch(handlers::patch_asset),
        )
        .route(
            "/api/admin/users/{user_id}/inventory",
            get(handlers::get_user_inventory),
        )
        .route(
            "/api/admin/users/{user_id}/inventory/contains/{asset_key}",
            get(handlers::check_user_inventory_presence),
        )
        .route(
            "/api/admin/users/{user_id}/inventory/stackables",
            get(handlers::get_user_stackables),
        )
        .route(
            "/api/admin/users/{user_id}/inventory/entitlements",
            get(handlers::get_user_entitlements),
        )
        .route(
            "/api/admin/users/{user_id}/inventory/expirables/active",
            get(handlers::get_user_active_expirables),
        )
        .route(
            "/api/admin/users/{user_id}/inventory/history",
            get(handlers::get_user_inventory_history),
        )
        .route(
            "/api/admin/users/{user_id}/inventory/entitlements/{asset_key}/grant",
            post(handlers::grant_entitlement),
        )
        .route(
            "/api/admin/users/{user_id}/inventory/entitlements/{asset_key}/revoke",
            post(handlers::revoke_entitlement),
        )
        .route(
            "/api/admin/users/{user_id}/inventory/stackables/{asset_key}/add",
            post(handlers::add_stackable),
        )
        .route(
            "/api/admin/users/{user_id}/inventory/stackables/{asset_key}/remove",
            post(handlers::remove_stackable),
        )
        .route(
            "/api/admin/users/{user_id}/inventory/stackables/{asset_key}",
            axum::routing::put(handlers::set_stackable),
        )
        .route(
            "/api/admin/users/{user_id}/inventory/expirables/{asset_key}/prolong",
            post(handlers::prolong_expirable),
        )
        .route(
            "/api/admin/users/{user_id}/inventory/expirables/{asset_key}/expiration",
            axum::routing::put(handlers::set_expiration),
        )
        .route(
            "/api/admin/users/{user_id}/inventory/expirables/{asset_key}",
            axum::routing::delete(handlers::revoke_expirable),
        )
        .route("/api/admin/users/{user_id}/wallet", get(handlers::get_user_wallet))
        .route(
            "/api/admin/users/{user_id}/wallet/{currency_key}",
            get(handlers::get_user_wallet_balance)
                .put(handlers::adjust_wallet_balance),
        )
        .route(
            "/api/admin/users/{user_id}/wallet/{currency_key}/transactions",
            get(handlers::get_user_wallet_transactions),
        )
        .route(
            "/api/admin/users/{user_id}/wallet/{currency_key}/credit",
            post(handlers::credit_wallet),
        )
        .route(
            "/api/admin/users/{user_id}/wallet/{currency_key}/debit",
            post(handlers::debit_wallet),
        )
}
