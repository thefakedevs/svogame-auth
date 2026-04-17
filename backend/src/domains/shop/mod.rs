pub mod handlers;

use axum::Router;
use axum::routing::{get, post};

pub fn router() -> Router<crate::app::state::SharedAppState> {
    Router::new()
        .route("/api/shop/products", get(handlers::list_public_products))
        .route(
            "/api/shop/products/{product_key}",
            get(handlers::get_public_product),
        )
        .route(
            "/api/payments/yookassa/webhook",
            post(handlers::yookassa_webhook),
        )
        .route(
            "/api/user/me/shop/orders",
            get(handlers::list_my_orders).post(handlers::create_my_order),
        )
        .route(
            "/api/user/me/shop/orders/{order_id}",
            get(handlers::get_my_order),
        )
        .route(
            "/api/user/me/shop/orders/{order_id}/receipt",
            get(handlers::get_my_order_receipt),
        )
        .route(
            "/api/user/me/shop/orders/{order_id}/receipt/print",
            get(handlers::print_my_order_receipt),
        )
        .route(
            "/api/user/me/shop/orders/{order_id}/mock/complete",
            post(handlers::complete_my_mock_order),
        )
        .route(
            "/api/admin/shop/products",
            get(handlers::list_admin_products).post(handlers::create_product),
        )
        .route(
            "/api/admin/shop/products/{product_id}",
            get(handlers::get_admin_product).patch(handlers::patch_product),
        )
}
