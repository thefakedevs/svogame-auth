mod common;

use std::collections::HashMap;
use std::sync::Arc;

use auth::app::config::{ShopConfig, ShopPaymentProviderKind};
use axum::Json;
use chrono::{DateTime, Duration, Utc};
use common::TestApp;
use reqwest::StatusCode;
use serial_test::serial;
use tokio::net::TcpListener;
use tokio::sync::RwLock;

#[tokio::test]
#[serial]
async fn public_shop_catalog_returns_localized_product_fields() {
    let app = TestApp::spawn().await;
    let admin = app.issue_user_token("ShopAdminRu", true, &[]).await;

    app.create_asset(
        &admin,
        serde_json::json!({
            "key": "repair_kit_shop",
            "display_name": "Repair Kit",
            "description": "Default description",
            "asset_kind": "item",
            "ownership_model": "stackable",
            "is_currency": false,
            "is_user_purchasable": true,
            "is_public": true,
            "metadata": {}
        }),
    )
    .await;

    let create_product = app
        .post_json(
            "/api/admin/shop/products",
            &admin.access_token,
            serde_json::json!({
                "key": "repair_kit_bundle",
                "asset_key": "repair_kit_shop",
                "price_rub": 149,
                "stackable_amount": 2,
                "max_per_purchase": 3,
                "max_owned_amount": 10,
                "locales": [
                    { "locale": "ru", "name": "Набор ремкомплектов", "description": "Два ремкомплекта" },
                    { "locale": "en", "name": "Repair Kit Bundle", "description": "Two repair kits" }
                ]
            }),
        )
        .await;
    assert!(
        create_product.status().is_success(),
        "{}",
        create_product.text().await.unwrap_or_default()
    );

    let response = app
        .get_without_auth("/api/shop/products/repair_kit_bundle?locale=ru")
        .await;
    assert!(response.status().is_success());
    let body: serde_json::Value = response.json().await.expect("shop product json");
    assert_eq!(body["localizedName"], "Набор ремкомплектов");
    assert_eq!(body["localizedDescription"], "Два ремкомплекта");
    assert_eq!(body["stackableAmount"], 2);
    assert_eq!(body["maxPerPurchase"], 3);
}

#[tokio::test]
#[serial]
async fn stackable_purchase_respects_max_owned_amount() {
    let app = TestApp::spawn().await;
    let admin = app.issue_user_token("ShopAdminStack", true, &[]).await;
    let user = app.issue_user_token("ShopBuyerStack", false, &[]).await;

    app.create_asset(
        &admin,
        serde_json::json!({
            "key": "ammo_crate",
            "display_name": "Ammo Crate",
            "description": null,
            "asset_kind": "item",
            "ownership_model": "stackable",
            "is_currency": false,
            "is_user_purchasable": true,
            "is_public": true,
            "metadata": {}
        }),
    )
    .await;

    let product = app
        .post_json(
            "/api/admin/shop/products",
            &admin.access_token,
            serde_json::json!({
                "key": "ammo_crate_bundle",
                "asset_key": "ammo_crate",
                "price_rub": 199,
                "stackable_amount": 2,
                "max_per_purchase": 2,
                "max_owned_amount": 5,
                "locales": [
                    { "locale": "en", "name": "Ammo Bundle", "description": "Two ammo crates" }
                ]
            }),
        )
        .await;
    assert!(product.status().is_success());

    let order = app
        .post_json(
            "/api/user/me/shop/orders",
            &user.access_token,
            serde_json::json!({
                "product_key": "ammo_crate_bundle",
                "quantity": 2
            }),
        )
        .await;
    assert!(
        order.status().is_success(),
        "{}",
        order.text().await.unwrap_or_default()
    );
    let order_body: serde_json::Value = order.json().await.expect("order json");
    let order_id = order_body["id"].as_str().expect("order id");

    let complete = app
        .post_json(
            &format!("/api/user/me/shop/orders/{order_id}/mock/complete"),
            &user.access_token,
            serde_json::json!({}),
        )
        .await;
    assert!(
        complete.status().is_success(),
        "{}",
        complete.text().await.unwrap_or_default()
    );
    let complete_body: serde_json::Value = complete.json().await.expect("completed order");
    assert_eq!(complete_body["status"], "fulfilled");
    assert_eq!(complete_body["grantedAmount"], 4);

    let stackables = app
        .get_json("/api/user/me/inventory/stackables", &user.access_token)
        .await;
    assert!(stackables.status().is_success());
    let stackables_body: serde_json::Value = stackables.json().await.expect("stackables json");
    assert_eq!(stackables_body[0]["assetKey"], "ammo_crate");
    assert_eq!(stackables_body[0]["amount"], 4);

    let overflow_order = app
        .post_json(
            "/api/user/me/shop/orders",
            &user.access_token,
            serde_json::json!({
                "product_key": "ammo_crate_bundle",
                "quantity": 1
            }),
        )
        .await;
    assert_eq!(overflow_order.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let overflow_body: serde_json::Value = overflow_order.json().await.expect("overflow body");
    assert_eq!(
        overflow_body["problem"]["detail"],
        "Purchase would exceed the ownership limit: current amount is 4, granted amount is 2, and maximum allowed is 5"
    );
}

#[tokio::test]
#[serial]
async fn entitlement_purchase_returns_conflict_if_user_already_owns_it() {
    let app = TestApp::spawn().await;
    let admin = app.issue_user_token("ShopAdminEnt", true, &[]).await;
    let user = app.issue_user_token("ShopBuyerEnt", false, &[]).await;

    app.create_asset(
        &admin,
        serde_json::json!({
            "key": "dragon_banner",
            "display_name": "Dragon Banner",
            "description": null,
            "asset_kind": "cosmetic",
            "ownership_model": "entitlement",
            "is_currency": false,
            "is_user_purchasable": true,
            "is_public": true,
            "metadata": {}
        }),
    )
    .await;

    let product = app
        .post_json(
            "/api/admin/shop/products",
            &admin.access_token,
            serde_json::json!({
                "key": "dragon_banner_unlock",
                "asset_key": "dragon_banner",
                "price_rub": 299,
                "locales": [
                    { "locale": "en", "name": "Dragon Banner", "description": "Permanent unlock" }
                ]
            }),
        )
        .await;
    assert!(product.status().is_success());

    let first_order = app
        .post_json(
            "/api/user/me/shop/orders",
            &user.access_token,
            serde_json::json!({
                "product_key": "dragon_banner_unlock"
            }),
        )
        .await;
    assert!(first_order.status().is_success());
    let first_order_body: serde_json::Value = first_order.json().await.expect("first order");
    let first_order_id = first_order_body["id"].as_str().expect("order id");

    let first_complete = app
        .post_json(
            &format!("/api/user/me/shop/orders/{first_order_id}/mock/complete"),
            &user.access_token,
            serde_json::json!({}),
        )
        .await;
    assert!(first_complete.status().is_success());
    let first_complete_body: serde_json::Value =
        first_complete.json().await.expect("first complete");
    assert_eq!(first_complete_body["status"], "fulfilled");

    let second_order = app
        .post_json(
            "/api/user/me/shop/orders",
            &user.access_token,
            serde_json::json!({
                "product_key": "dragon_banner_unlock"
            }),
        )
        .await;
    assert_eq!(second_order.status(), StatusCode::CONFLICT);
    let second_body: serde_json::Value = second_order.json().await.expect("conflict body");
    assert_eq!(second_body["error"], "User already owns this entitlement");
    assert_eq!(
        second_body["problem"]["detail"],
        "User already owns this entitlement"
    );
}

#[tokio::test]
#[serial]
async fn subscription_purchase_uses_duration_seconds_and_prolongs_existing_time() {
    let app = TestApp::spawn().await;
    let admin = app.issue_user_token("ShopAdminSub", true, &[]).await;
    let user = app.issue_user_token("ShopBuyerSub", false, &[]).await;

    let product = app
        .post_json(
            "/api/admin/shop/products",
            &admin.access_token,
            serde_json::json!({
                "key": "plus_hour",
                "asset_key": "subscription_plus",
                "price_rub": 99,
                "durationSeconds": 3600,
                "locales": [
                    { "locale": "en", "name": "Plus 1 Hour", "description": "Adds one hour of Plus" }
                ]
            }),
        )
        .await;
    assert!(
        product.status().is_success(),
        "{}",
        product.text().await.unwrap_or_default()
    );

    let first_order = app
        .post_json(
            "/api/user/me/shop/orders",
            &user.access_token,
            serde_json::json!({
                "product_key": "plus_hour"
            }),
        )
        .await;
    assert!(first_order.status().is_success());
    let first_order_body: serde_json::Value = first_order.json().await.expect("first order");
    let first_order_id = first_order_body["id"].as_str().expect("first order id");

    let first_complete = app
        .post_json(
            &format!("/api/user/me/shop/orders/{first_order_id}/mock/complete"),
            &user.access_token,
            serde_json::json!({}),
        )
        .await;
    assert!(first_complete.status().is_success());
    let first_complete_body: serde_json::Value =
        first_complete.json().await.expect("first complete");
    assert_eq!(first_complete_body["status"], "fulfilled");
    assert_eq!(first_complete_body["grantedDurationSeconds"], 3600);

    let expirables_after_first = app
        .get_json(
            "/api/user/me/inventory/expirables/active",
            &user.access_token,
        )
        .await;
    assert!(expirables_after_first.status().is_success());
    let expirables_after_first_body: serde_json::Value = expirables_after_first
        .json()
        .await
        .expect("expirables after first");
    let first_expiration: DateTime<Utc> = expirables_after_first_body[0]["expiresAt"]
        .as_str()
        .expect("first expiresAt")
        .parse()
        .expect("parse first expiration");

    let second_order = app
        .post_json(
            "/api/user/me/shop/orders",
            &user.access_token,
            serde_json::json!({
                "product_key": "plus_hour"
            }),
        )
        .await;
    assert!(second_order.status().is_success());
    let second_order_body: serde_json::Value = second_order.json().await.expect("second order");
    let second_order_id = second_order_body["id"].as_str().expect("second order id");

    let second_complete = app
        .post_json(
            &format!("/api/user/me/shop/orders/{second_order_id}/mock/complete"),
            &user.access_token,
            serde_json::json!({}),
        )
        .await;
    assert!(second_complete.status().is_success());

    let expirables_after_second = app
        .get_json(
            "/api/user/me/inventory/expirables/active",
            &user.access_token,
        )
        .await;
    assert!(expirables_after_second.status().is_success());
    let expirables_after_second_body: serde_json::Value = expirables_after_second
        .json()
        .await
        .expect("expirables after second");
    let second_expiration: DateTime<Utc> = expirables_after_second_body[0]["expiresAt"]
        .as_str()
        .expect("second expiresAt")
        .parse()
        .expect("parse second expiration");

    assert!(second_expiration > first_expiration);
}

#[tokio::test]
#[serial]
async fn shop_payments_can_be_disabled_via_config() {
    let app = TestApp::spawn_with_shop_payment_provider(ShopPaymentProviderKind::Disabled).await;
    let admin = app.issue_user_token("ShopAdminDisabled", true, &[]).await;
    let user = app.issue_user_token("ShopBuyerDisabled", false, &[]).await;

    create_stackable_asset(&app, &admin, "disabled_crate", "Disabled Crate").await;
    create_stackable_product(
        &app,
        &admin,
        "disabled_crate_bundle",
        "disabled_crate",
        1,
        Some(2),
        Some(10),
    )
    .await;

    let response = app
        .post_json(
            "/api/user/me/shop/orders",
            &user.access_token,
            serde_json::json!({
                "product_key": "disabled_crate_bundle"
            }),
        )
        .await;
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let body: serde_json::Value = response.json().await.expect("disabled payment body");
    assert_eq!(
        body["problem"]["detail"],
        "Shop payment provider is disabled by configuration"
    );
}

#[tokio::test]
#[serial]
async fn public_catalog_hides_private_inactive_and_future_products() {
    let app = TestApp::spawn().await;
    let admin = app.issue_user_token("ShopAdminCatalog", true, &[]).await;

    create_stackable_asset(&app, &admin, "public_box", "Public Box").await;
    create_stackable_asset(&app, &admin, "private_box", "Private Box").await;
    create_stackable_asset(&app, &admin, "inactive_box", "Inactive Box").await;
    create_stackable_asset(&app, &admin, "future_box", "Future Box").await;

    create_stackable_product(
        &app,
        &admin,
        "public_box_bundle",
        "public_box",
        1,
        Some(5),
        Some(20),
    )
    .await;

    let private_product = app
        .post_json(
            "/api/admin/shop/products",
            &admin.access_token,
            serde_json::json!({
                "key": "private_box_bundle",
                "asset_key": "private_box",
                "price_rub": 50,
                "stackable_amount": 1,
                "is_public": false,
                "locales": [{ "locale": "en", "name": "Private Box", "description": null }]
            }),
        )
        .await;
    assert!(private_product.status().is_success());

    let inactive_product = app
        .post_json(
            "/api/admin/shop/products",
            &admin.access_token,
            serde_json::json!({
                "key": "inactive_box_bundle",
                "asset_key": "inactive_box",
                "price_rub": 50,
                "stackable_amount": 1,
                "is_active": false,
                "locales": [{ "locale": "en", "name": "Inactive Box", "description": null }]
            }),
        )
        .await;
    assert!(inactive_product.status().is_success());

    let future_product = app
        .post_json(
            "/api/admin/shop/products",
            &admin.access_token,
            serde_json::json!({
                "key": "future_box_bundle",
                "asset_key": "future_box",
                "price_rub": 50,
                "stackable_amount": 1,
                "starts_at": (Utc::now() + Duration::days(1)),
                "locales": [{ "locale": "en", "name": "Future Box", "description": null }]
            }),
        )
        .await;
    assert!(future_product.status().is_success());

    let list_response = app.get_without_auth("/api/shop/products").await;
    assert!(list_response.status().is_success());
    let list_body: serde_json::Value = list_response.json().await.expect("catalog list");
    let items = list_body.as_array().expect("catalog items");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["key"], "public_box_bundle");

    for hidden_key in [
        "private_box_bundle",
        "inactive_box_bundle",
        "future_box_bundle",
    ] {
        let hidden_response = app
            .get_without_auth(&format!("/api/shop/products/{hidden_key}"))
            .await;
        assert_eq!(hidden_response.status(), StatusCode::NOT_FOUND);
    }
}

#[tokio::test]
#[serial]
async fn product_creation_rejects_invalid_reward_configurations() {
    let app = TestApp::spawn().await;
    let admin = app.issue_user_token("ShopAdminInvalid", true, &[]).await;

    app.create_asset(
        &admin,
        serde_json::json!({
            "key": "invalid_entitlement_asset",
            "display_name": "Invalid Entitlement",
            "description": null,
            "asset_kind": "cosmetic",
            "ownership_model": "entitlement",
            "is_currency": false,
            "is_user_purchasable": true,
            "is_public": true,
            "metadata": {}
        }),
    )
    .await;
    create_stackable_asset(&app, &admin, "invalid_stackable_asset", "Invalid Stackable").await;

    let entitlement_with_amount = app
        .post_json(
            "/api/admin/shop/products",
            &admin.access_token,
            serde_json::json!({
                "key": "bad_entitlement_product",
                "asset_key": "invalid_entitlement_asset",
                "price_rub": 100,
                "stackable_amount": 1,
                "locales": [{ "locale": "en", "name": "Bad Entitlement", "description": null }]
            }),
        )
        .await;
    assert_eq!(entitlement_with_amount.status(), StatusCode::BAD_REQUEST);

    let subscription_without_duration = app
        .post_json(
            "/api/admin/shop/products",
            &admin.access_token,
            serde_json::json!({
                "key": "bad_subscription_product",
                "asset_key": "subscription_plus",
                "price_rub": 100,
                "locales": [{ "locale": "en", "name": "Bad Subscription", "description": null }]
            }),
        )
        .await;
    assert_eq!(
        subscription_without_duration.status(),
        StatusCode::BAD_REQUEST
    );

    let duplicate_locale = app
        .post_json(
            "/api/admin/shop/products",
            &admin.access_token,
            serde_json::json!({
                "key": "bad_locale_product",
                "asset_key": "invalid_stackable_asset",
                "price_rub": 100,
                "stackable_amount": 1,
                "locales": [
                    { "locale": "en", "name": "One", "description": null },
                    { "locale": "en", "name": "Two", "description": null }
                ]
            }),
        )
        .await;
    assert_eq!(duplicate_locale.status(), StatusCode::BAD_REQUEST);

    let invalid_limit = app
        .post_json(
            "/api/admin/shop/products",
            &admin.access_token,
            serde_json::json!({
                "key": "bad_limit_product",
                "asset_key": "invalid_stackable_asset",
                "price_rub": 100,
                "stackable_amount": 5,
                "max_owned_amount": 3,
                "locales": [{ "locale": "en", "name": "Bad Limit", "description": null }]
            }),
        )
        .await;
    assert_eq!(invalid_limit.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
#[serial]
async fn stackable_purchase_rejects_quantity_above_max_per_purchase() {
    let app = TestApp::spawn().await;
    let admin = app.issue_user_token("ShopAdminMax", true, &[]).await;
    let user = app.issue_user_token("ShopBuyerMax", false, &[]).await;

    create_stackable_asset(&app, &admin, "medkit", "Medkit").await;
    create_stackable_product(&app, &admin, "medkit_pack", "medkit", 1, Some(2), Some(20)).await;

    let response = app
        .post_json(
            "/api/user/me/shop/orders",
            &user.access_token,
            serde_json::json!({
                "product_key": "medkit_pack",
                "quantity": 3
            }),
        )
        .await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body: serde_json::Value = response.json().await.expect("max per purchase body");
    assert_eq!(
        body["problem"]["detail"],
        "Quantity exceeds max_per_purchase: requested 3, allowed 2"
    );
}

#[tokio::test]
#[serial]
async fn entitlement_and_subscription_products_reject_quantity_greater_than_one() {
    let app = TestApp::spawn().await;
    let admin = app.issue_user_token("ShopAdminQty", true, &[]).await;
    let user = app.issue_user_token("ShopBuyerQty", false, &[]).await;

    app.create_asset(
        &admin,
        serde_json::json!({
            "key": "vip_badge",
            "display_name": "VIP Badge",
            "description": null,
            "asset_kind": "cosmetic",
            "ownership_model": "entitlement",
            "is_currency": false,
            "is_user_purchasable": true,
            "is_public": true,
            "metadata": {}
        }),
    )
    .await;

    let entitlement_product = app
        .post_json(
            "/api/admin/shop/products",
            &admin.access_token,
            serde_json::json!({
                "key": "vip_badge_unlock",
                "asset_key": "vip_badge",
                "price_rub": 250,
                "locales": [{ "locale": "en", "name": "VIP Badge", "description": null }]
            }),
        )
        .await;
    assert!(entitlement_product.status().is_success());

    let subscription_product = app
        .post_json(
            "/api/admin/shop/products",
            &admin.access_token,
            serde_json::json!({
                "key": "plus_day",
                "asset_key": "subscription_plus",
                "price_rub": 150,
                "durationSeconds": 86400,
                "locales": [{ "locale": "en", "name": "Plus Day", "description": null }]
            }),
        )
        .await;
    assert!(subscription_product.status().is_success());

    for product_key in ["vip_badge_unlock", "plus_day"] {
        let response = app
            .post_json(
                "/api/user/me/shop/orders",
                &user.access_token,
                serde_json::json!({
                    "product_key": product_key,
                    "quantity": 2
                }),
            )
            .await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body: serde_json::Value = response.json().await.expect("quantity body");
        assert_eq!(
            body["problem"]["detail"],
            "Quantity must be exactly 1 for entitlement and subscription products"
        );
    }
}

#[tokio::test]
#[serial]
async fn mock_completion_is_idempotent_and_does_not_double_grant_assets() {
    let app = TestApp::spawn().await;
    let admin = app.issue_user_token("ShopAdminIdem", true, &[]).await;
    let user = app.issue_user_token("ShopBuyerIdem", false, &[]).await;

    create_stackable_asset(&app, &admin, "shield_booster", "Shield Booster").await;
    create_stackable_product(
        &app,
        &admin,
        "shield_booster_pack",
        "shield_booster",
        2,
        Some(1),
        Some(10),
    )
    .await;

    let order = app
        .post_json(
            "/api/user/me/shop/orders",
            &user.access_token,
            serde_json::json!({
                "product_key": "shield_booster_pack"
            }),
        )
        .await;
    assert!(order.status().is_success());
    let order_body: serde_json::Value = order.json().await.expect("idempotent order");
    let order_id = order_body["id"].as_str().expect("order id");

    let first_complete = app
        .post_json(
            &format!("/api/user/me/shop/orders/{order_id}/mock/complete"),
            &user.access_token,
            serde_json::json!({}),
        )
        .await;
    assert!(first_complete.status().is_success());

    let second_complete = app
        .post_json(
            &format!("/api/user/me/shop/orders/{order_id}/mock/complete"),
            &user.access_token,
            serde_json::json!({}),
        )
        .await;
    assert!(second_complete.status().is_success());
    let second_body: serde_json::Value = second_complete.json().await.expect("second complete");
    assert_eq!(second_body["status"], "fulfilled");
    assert_eq!(second_body["grantedAmount"], 2);

    let stackables = app
        .get_json("/api/user/me/inventory/stackables", &user.access_token)
        .await;
    let stackables_body: serde_json::Value = stackables.json().await.expect("stackables");
    assert_eq!(stackables_body[0]["amount"], 2);
}

#[tokio::test]
#[serial]
async fn completion_returns_conflict_when_entitlement_was_granted_after_order_creation() {
    let app = TestApp::spawn().await;
    let admin = app.issue_user_token("ShopAdminRaceEnt", true, &[]).await;
    let user = app.issue_user_token("ShopBuyerRaceEnt", false, &[]).await;

    app.create_asset(
        &admin,
        serde_json::json!({
            "key": "race_entitlement",
            "display_name": "Race Entitlement",
            "description": null,
            "asset_kind": "cosmetic",
            "ownership_model": "entitlement",
            "is_currency": false,
            "is_user_purchasable": true,
            "is_public": true,
            "metadata": {}
        }),
    )
    .await;

    let product = app
        .post_json(
            "/api/admin/shop/products",
            &admin.access_token,
            serde_json::json!({
                "key": "race_entitlement_product",
                "asset_key": "race_entitlement",
                "price_rub": 150,
                "locales": [{ "locale": "en", "name": "Race Entitlement", "description": null }]
            }),
        )
        .await;
    assert!(product.status().is_success());

    let order = app
        .post_json(
            "/api/user/me/shop/orders",
            &user.access_token,
            serde_json::json!({
                "product_key": "race_entitlement_product"
            }),
        )
        .await;
    assert!(order.status().is_success());
    let order_body: serde_json::Value = order.json().await.expect("race order");
    let order_id = order_body["id"].as_str().expect("race order id");

    let grant = app
        .post_json(
            &format!(
                "/api/admin/users/{}/inventory/entitlements/race_entitlement/grant",
                user.user_id
            ),
            &admin.access_token,
            serde_json::json!({ "reasonText": "manual grant before payment completion" }),
        )
        .await;
    assert!(grant.status().is_success());

    let complete = app
        .post_json(
            &format!("/api/user/me/shop/orders/{order_id}/mock/complete"),
            &user.access_token,
            serde_json::json!({}),
        )
        .await;
    assert_eq!(complete.status(), StatusCode::CONFLICT);
    let complete_body: serde_json::Value = complete.json().await.expect("race complete body");
    assert_eq!(
        complete_body["problem"]["detail"],
        "User already owns this entitlement"
    );

    let order_after = app
        .get_json(
            &format!("/api/user/me/shop/orders/{order_id}"),
            &user.access_token,
        )
        .await;
    assert!(order_after.status().is_success());
    let order_after_body: serde_json::Value = order_after.json().await.expect("race order after");
    assert_eq!(order_after_body["status"], "fulfillment_failed");
    assert_eq!(
        order_after_body["failureProblem"],
        "User already owns this entitlement"
    );
}

#[tokio::test]
#[serial]
async fn completion_returns_conflict_when_stackable_limit_changes_before_fulfillment() {
    let app = TestApp::spawn().await;
    let admin = app.issue_user_token("ShopAdminRaceStack", true, &[]).await;
    let user = app.issue_user_token("ShopBuyerRaceStack", false, &[]).await;

    create_stackable_asset(&app, &admin, "race_stackable", "Race Stackable").await;
    create_stackable_product(
        &app,
        &admin,
        "race_stackable_bundle",
        "race_stackable",
        2,
        Some(2),
        Some(5),
    )
    .await;

    let order = app
        .post_json(
            "/api/user/me/shop/orders",
            &user.access_token,
            serde_json::json!({
                "product_key": "race_stackable_bundle"
            }),
        )
        .await;
    assert!(order.status().is_success());
    let order_body: serde_json::Value = order.json().await.expect("race stack order");
    let order_id = order_body["id"].as_str().expect("race stack order id");

    let admin_grant = app
        .post_json(
            &format!(
                "/api/admin/users/{}/inventory/stackables/race_stackable/add",
                user.user_id
            ),
            &admin.access_token,
            serde_json::json!({ "amount": 4, "reasonText": "manual grant before payment completion" }),
        )
        .await;
    assert!(admin_grant.status().is_success());

    let complete = app
        .post_json(
            &format!("/api/user/me/shop/orders/{order_id}/mock/complete"),
            &user.access_token,
            serde_json::json!({}),
        )
        .await;
    assert_eq!(complete.status(), StatusCode::CONFLICT);
    let complete_body: serde_json::Value = complete.json().await.expect("stack conflict");
    assert_eq!(
        complete_body["problem"]["detail"],
        "Purchase would exceed the ownership limit: current amount is 4, granted amount is 2, and maximum allowed is 5"
    );

    let order_after = app
        .get_json(
            &format!("/api/user/me/shop/orders/{order_id}"),
            &user.access_token,
        )
        .await;
    assert!(order_after.status().is_success());
    let order_after_body: serde_json::Value = order_after.json().await.expect("stack order after");
    assert_eq!(order_after_body["status"], "fulfillment_failed");
}

#[tokio::test]
#[serial]
async fn existing_order_keeps_price_and_reward_snapshot_after_product_update() {
    let app = TestApp::spawn().await;
    let admin = app.issue_user_token("ShopAdminSnapshot", true, &[]).await;
    let user = app.issue_user_token("ShopBuyerSnapshot", false, &[]).await;

    create_stackable_asset(&app, &admin, "snapshot_case", "Snapshot Case").await;
    create_stackable_product(
        &app,
        &admin,
        "snapshot_case_bundle",
        "snapshot_case",
        2,
        Some(2),
        Some(20),
    )
    .await;

    let order = app
        .post_json(
            "/api/user/me/shop/orders",
            &user.access_token,
            serde_json::json!({
                "product_key": "snapshot_case_bundle",
                "quantity": 1
            }),
        )
        .await;
    assert!(order.status().is_success());
    let order_body: serde_json::Value = order.json().await.expect("snapshot order");
    let order_id = order_body["id"].as_str().expect("snapshot order id");
    assert_eq!(order_body["unitPriceRub"], 100);
    assert_eq!(order_body["grantedAmount"], 2);

    let product_id = order_body["productId"].as_str().expect("product id");
    let patch = app
        .patch_json(
            &format!("/api/admin/shop/products/{product_id}"),
            &admin.access_token,
            serde_json::json!({
                "price_rub": 999,
                "stackable_amount": 5,
                "max_per_purchase": 5,
                "max_owned_amount": 100,
                "locales": [{ "locale": "en", "name": "Snapshot Case Deluxe", "description": "Updated reward" }]
            }),
        )
        .await;
    assert!(
        patch.status().is_success(),
        "{}",
        patch.text().await.unwrap_or_default()
    );

    let complete = app
        .post_json(
            &format!("/api/user/me/shop/orders/{order_id}/mock/complete"),
            &user.access_token,
            serde_json::json!({}),
        )
        .await;
    assert!(complete.status().is_success());
    let complete_body: serde_json::Value = complete.json().await.expect("snapshot complete");
    assert_eq!(complete_body["unitPriceRub"], 100);
    assert_eq!(complete_body["totalPriceRub"], 100);
    assert_eq!(complete_body["grantedAmount"], 2);
    assert_eq!(complete_body["productName"], "snapshot_case_bundle");

    let next_order = app
        .post_json(
            "/api/user/me/shop/orders",
            &user.access_token,
            serde_json::json!({
                "product_key": "snapshot_case_bundle",
                "quantity": 1,
                "locale": "en"
            }),
        )
        .await;
    assert!(next_order.status().is_success());
    let next_order_body: serde_json::Value = next_order.json().await.expect("next snapshot order");
    assert_eq!(next_order_body["unitPriceRub"], 999);
    assert_eq!(next_order_body["grantedAmount"], 5);
    assert_eq!(next_order_body["productName"], "Snapshot Case Deluxe");
}

#[tokio::test]
#[serial]
async fn order_creation_rejects_private_inactive_and_time_locked_products() {
    let app = TestApp::spawn().await;
    let admin = app
        .issue_user_token("ShopAdminAvailability", true, &[])
        .await;
    let user = app
        .issue_user_token("ShopBuyerAvailability", false, &[])
        .await;

    create_stackable_asset(&app, &admin, "private_locked", "Private Locked").await;
    create_stackable_asset(&app, &admin, "inactive_locked", "Inactive Locked").await;
    create_stackable_asset(&app, &admin, "future_locked", "Future Locked").await;
    create_stackable_asset(&app, &admin, "expired_locked", "Expired Locked").await;

    let private_product = app
        .post_json(
            "/api/admin/shop/products",
            &admin.access_token,
            serde_json::json!({
                "key": "private_locked_bundle",
                "asset_key": "private_locked",
                "price_rub": 100,
                "stackable_amount": 1,
                "is_public": false,
                "locales": [{ "locale": "en", "name": "Private Locked", "description": null }]
            }),
        )
        .await;
    assert!(private_product.status().is_success());

    let inactive_product = app
        .post_json(
            "/api/admin/shop/products",
            &admin.access_token,
            serde_json::json!({
                "key": "inactive_locked_bundle",
                "asset_key": "inactive_locked",
                "price_rub": 100,
                "stackable_amount": 1,
                "is_active": false,
                "locales": [{ "locale": "en", "name": "Inactive Locked", "description": null }]
            }),
        )
        .await;
    assert!(inactive_product.status().is_success());

    let future_product = app
        .post_json(
            "/api/admin/shop/products",
            &admin.access_token,
            serde_json::json!({
                "key": "future_locked_bundle",
                "asset_key": "future_locked",
                "price_rub": 100,
                "stackable_amount": 1,
                "starts_at": Utc::now() + Duration::hours(2),
                "locales": [{ "locale": "en", "name": "Future Locked", "description": null }]
            }),
        )
        .await;
    assert!(future_product.status().is_success());

    let expired_product = app
        .post_json(
            "/api/admin/shop/products",
            &admin.access_token,
            serde_json::json!({
                "key": "expired_locked_bundle",
                "asset_key": "expired_locked",
                "price_rub": 100,
                "stackable_amount": 1,
                "ends_at": Utc::now() - Duration::hours(2),
                "locales": [{ "locale": "en", "name": "Expired Locked", "description": null }]
            }),
        )
        .await;
    assert!(expired_product.status().is_success());

    for hidden_key in ["private_locked_bundle", "inactive_locked_bundle"] {
        let response = app
            .post_json(
                "/api/user/me/shop/orders",
                &user.access_token,
                serde_json::json!({
                    "product_key": hidden_key
                }),
            )
            .await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let body: serde_json::Value = response.json().await.expect("hidden product body");
        assert_eq!(body["problem"]["detail"], "Shop product not found");
    }

    for unavailable_key in ["future_locked_bundle", "expired_locked_bundle"] {
        let response = app
            .post_json(
                "/api/user/me/shop/orders",
                &user.access_token,
                serde_json::json!({
                    "product_key": unavailable_key
                }),
            )
            .await;
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        let body: serde_json::Value = response.json().await.expect("time-locked body");
        assert_eq!(
            body["problem"]["detail"],
            "Shop product is not available for purchase right now"
        );
    }
}

#[tokio::test]
#[serial]
async fn product_creation_rejects_duplicate_keys_invalid_window_and_inactive_assets() {
    let app = TestApp::spawn().await;
    let admin = app
        .issue_user_token("ShopAdminCreateValidation", true, &[])
        .await;

    create_stackable_asset(&app, &admin, "duplicate_asset", "Duplicate Asset").await;

    let first_product = app
        .post_json(
            "/api/admin/shop/products",
            &admin.access_token,
            serde_json::json!({
                "key": "duplicate_bundle",
                "asset_key": "duplicate_asset",
                "price_rub": 100,
                "stackable_amount": 1,
                "locales": [{ "locale": "en", "name": "Duplicate Bundle", "description": null }]
            }),
        )
        .await;
    assert!(first_product.status().is_success());

    let duplicate_key = app
        .post_json(
            "/api/admin/shop/products",
            &admin.access_token,
            serde_json::json!({
                "key": "duplicate_bundle",
                "asset_key": "duplicate_asset",
                "price_rub": 200,
                "stackable_amount": 2,
                "locales": [{ "locale": "en", "name": "Duplicate Bundle Again", "description": null }]
            }),
        )
        .await;
    assert_eq!(duplicate_key.status(), StatusCode::CONFLICT);
    let duplicate_body: serde_json::Value = duplicate_key.json().await.expect("duplicate body");
    assert_eq!(
        duplicate_body["problem"]["detail"],
        "Shop product key already exists"
    );

    let invalid_window = app
        .post_json(
            "/api/admin/shop/products",
            &admin.access_token,
            serde_json::json!({
                "key": "invalid_window_bundle",
                "asset_key": "duplicate_asset",
                "price_rub": 100,
                "stackable_amount": 1,
                "starts_at": Utc::now() + Duration::days(2),
                "ends_at": Utc::now() + Duration::days(1),
                "locales": [{ "locale": "en", "name": "Invalid Window", "description": null }]
            }),
        )
        .await;
    assert_eq!(invalid_window.status(), StatusCode::BAD_REQUEST);
    let invalid_window_body: serde_json::Value =
        invalid_window.json().await.expect("invalid window body");
    assert_eq!(
        invalid_window_body["problem"]["detail"],
        "starts_at cannot be later than ends_at"
    );

    let inactive_asset = app
        .create_asset(
            &admin,
            serde_json::json!({
                "key": "inactive_shop_asset",
                "display_name": "Inactive Shop Asset",
                "description": null,
                "asset_kind": "item",
                "ownership_model": "stackable",
                "is_currency": false,
                "is_user_purchasable": true,
                "is_public": true,
                "is_active": false,
                "metadata": {}
            }),
        )
        .await;
    app.patch_asset(
        &admin,
        inactive_asset["id"].as_str().expect("inactive asset id"),
        serde_json::json!({
            "is_active": false
        }),
    )
    .await;

    let inactive_asset_product = app
        .post_json(
            "/api/admin/shop/products",
            &admin.access_token,
            serde_json::json!({
                "key": "inactive_asset_bundle",
                "asset_key": "inactive_shop_asset",
                "price_rub": 100,
                "stackable_amount": 1,
                "locales": [{ "locale": "en", "name": "Inactive Asset Bundle", "description": null }]
            }),
        )
        .await;
    assert_eq!(
        inactive_asset_product.status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let inactive_asset_body: serde_json::Value = inactive_asset_product
        .json()
        .await
        .expect("inactive asset body");
    assert_eq!(
        inactive_asset_body["problem"]["detail"],
        "Shop product asset must be active"
    );
}

#[tokio::test]
#[serial]
async fn locale_resolution_supports_primary_language_fallback_and_rejects_invalid_locale() {
    let app = TestApp::spawn().await;
    let admin = app.issue_user_token("ShopAdminLocale", true, &[]).await;

    create_stackable_asset(&app, &admin, "locale_crate", "Locale Crate").await;
    let product = app
        .post_json(
            "/api/admin/shop/products",
            &admin.access_token,
            serde_json::json!({
                "key": "locale_crate_bundle",
                "asset_key": "locale_crate",
                "price_rub": 100,
                "stackable_amount": 1,
                "locales": [
                    { "locale": "ru", "name": "Ящик", "description": "Русская локаль" },
                    { "locale": "en", "name": "Crate", "description": "English locale" }
                ]
            }),
        )
        .await;
    assert!(product.status().is_success());

    let fallback_response = app
        .get_without_auth("/api/shop/products/locale_crate_bundle?locale=ru-RU")
        .await;
    assert!(fallback_response.status().is_success());
    let fallback_body: serde_json::Value = fallback_response
        .json()
        .await
        .expect("fallback locale body");
    assert_eq!(fallback_body["localizedName"], "Ящик");
    assert_eq!(fallback_body["localizedDescription"], "Русская локаль");

    let invalid_locale_response = app
        .get_without_auth("/api/shop/products/locale_crate_bundle?locale=RU_RU")
        .await;
    assert_eq!(invalid_locale_response.status(), StatusCode::BAD_REQUEST);
    let invalid_locale_body: serde_json::Value = invalid_locale_response
        .json()
        .await
        .expect("invalid locale body");
    assert_eq!(
        invalid_locale_body["problem"]["detail"],
        "locale must contain only lowercase latin letters, numbers, and hyphens"
    );
}

#[tokio::test]
#[serial]
async fn yookassa_order_creation_returns_redirect_checkout_url_and_metadata() {
    let fake_yookassa = FakeYooKassaServer::spawn().await;
    let app = TestApp::spawn_with_yookassa(
        fake_yookassa.base_url(),
        "http://localhost:5173/shop/checkout/return".to_string(),
    )
    .await;
    let admin = app.issue_user_token("ShopAdminYkCreate", true, &[]).await;
    let user = app.issue_user_token("ShopBuyerYkCreate", false, &[]).await;

    create_stackable_asset(&app, &admin, "yk_crate", "YK Crate").await;
    create_stackable_product(
        &app,
        &admin,
        "yk_crate_bundle",
        "yk_crate",
        2,
        Some(3),
        Some(20),
    )
    .await;

    let response = app
        .post_json(
            "/api/user/me/shop/orders",
            &user.access_token,
            serde_json::json!({
                "product_key": "yk_crate_bundle",
                "quantity": 1
            }),
        )
        .await;
    assert!(
        response.status().is_success(),
        "{}",
        response.text().await.unwrap_or_default()
    );
    let body: serde_json::Value = response.json().await.expect("yookassa order body");
    let order_id = body["id"].as_str().expect("order id");
    let payment_id = body["payment"]["providerPaymentId"]
        .as_str()
        .expect("provider payment id");
    assert_eq!(body["paymentProvider"], "yookassa");
    assert_eq!(body["status"], "pending_payment");
    assert_eq!(
        body["payment"]["checkoutUrl"],
        format!("https://checkout.test/{payment_id}")
    );

    let create_request = fake_yookassa
        .last_create_payment_request()
        .await
        .expect("create payment request");
    assert_eq!(create_request["metadata"]["orderId"], order_id);
    assert_eq!(
        create_request["confirmation"]["return_url"],
        format!("http://localhost:5173/shop/checkout/return?orderId={order_id}")
    );
    assert_eq!(create_request["amount"]["value"], "100.00");
    assert_eq!(create_request["capture"], true);
}

#[tokio::test]
#[serial]
async fn yookassa_success_webhook_fulfills_order_and_is_idempotent() {
    let fake_yookassa = FakeYooKassaServer::spawn().await;
    let app = TestApp::spawn_with_yookassa(
        fake_yookassa.base_url(),
        "http://localhost:5173/shop/checkout/return".to_string(),
    )
    .await;
    let admin = app.issue_user_token("ShopAdminYkSuccess", true, &[]).await;
    let user = app.issue_user_token("ShopBuyerYkSuccess", false, &[]).await;

    create_stackable_asset(&app, &admin, "yk_success_crate", "YK Success Crate").await;
    create_stackable_product(
        &app,
        &admin,
        "yk_success_bundle",
        "yk_success_crate",
        2,
        Some(5),
        Some(20),
    )
    .await;

    let order = app
        .post_json(
            "/api/user/me/shop/orders",
            &user.access_token,
            serde_json::json!({
                "product_key": "yk_success_bundle"
            }),
        )
        .await;
    assert!(order.status().is_success());
    let order_body: serde_json::Value = order.json().await.expect("yk order");
    let order_id = order_body["id"].as_str().expect("order id");
    let payment_id = order_body["payment"]["providerPaymentId"]
        .as_str()
        .expect("payment id")
        .to_string();

    fake_yookassa.set_payment_succeeded(&payment_id).await;

    let webhook_body = serde_json::json!({
        "type": "notification",
        "event": "payment.succeeded",
        "object": {
            "id": payment_id,
            "status": "succeeded"
        }
    });
    let webhook = app
        .post_raw_without_auth("/api/payments/yookassa/webhook", webhook_body.to_string())
        .await;
    assert!(
        webhook.status().is_success(),
        "{}",
        webhook.text().await.unwrap_or_default()
    );
    let webhook_ack: serde_json::Value = webhook.json().await.expect("webhook ack");
    assert_eq!(webhook_ack["orderStatus"], "fulfilled");

    let duplicate = app
        .post_raw_without_auth("/api/payments/yookassa/webhook", webhook_body.to_string())
        .await;
    assert!(
        duplicate.status().is_success(),
        "{}",
        duplicate.text().await.unwrap_or_default()
    );
    let duplicate_ack: serde_json::Value = duplicate.json().await.expect("duplicate ack");
    assert_eq!(duplicate_ack["orderStatus"], "fulfilled");

    let order_after = app
        .get_json(
            &format!("/api/user/me/shop/orders/{order_id}"),
            &user.access_token,
        )
        .await;
    assert!(order_after.status().is_success());
    let order_after_body: serde_json::Value = order_after.json().await.expect("order after");
    assert_eq!(order_after_body["status"], "fulfilled");

    let stackables = app
        .get_json("/api/user/me/inventory/stackables", &user.access_token)
        .await;
    assert!(stackables.status().is_success());
    let stackables_body: serde_json::Value = stackables.json().await.expect("stackables body");
    assert_eq!(stackables_body[0]["assetKey"], "yk_success_crate");
    assert_eq!(stackables_body[0]["amount"], 2);
}

#[tokio::test]
#[serial]
async fn yookassa_canceled_webhook_marks_order_without_fulfillment() {
    let fake_yookassa = FakeYooKassaServer::spawn().await;
    let app = TestApp::spawn_with_yookassa(
        fake_yookassa.base_url(),
        "http://localhost:5173/shop/checkout/return".to_string(),
    )
    .await;
    let admin = app.issue_user_token("ShopAdminYkCancel", true, &[]).await;
    let user = app.issue_user_token("ShopBuyerYkCancel", false, &[]).await;

    create_stackable_asset(&app, &admin, "yk_cancel_crate", "YK Cancel Crate").await;
    create_stackable_product(
        &app,
        &admin,
        "yk_cancel_bundle",
        "yk_cancel_crate",
        1,
        Some(2),
        Some(10),
    )
    .await;

    let order = app
        .post_json(
            "/api/user/me/shop/orders",
            &user.access_token,
            serde_json::json!({
                "product_key": "yk_cancel_bundle"
            }),
        )
        .await;
    assert!(order.status().is_success());
    let order_body: serde_json::Value = order.json().await.expect("order body");
    let order_id = order_body["id"].as_str().expect("order id");
    let payment_id = order_body["payment"]["providerPaymentId"]
        .as_str()
        .expect("payment id")
        .to_string();

    fake_yookassa
        .set_payment_canceled(&payment_id, "permission_revocation")
        .await;

    let webhook = app
        .post_raw_without_auth(
            "/api/payments/yookassa/webhook",
            serde_json::json!({
                "type": "notification",
                "event": "payment.canceled",
                "object": {
                    "id": payment_id,
                    "status": "canceled"
                }
            })
            .to_string(),
        )
        .await;
    assert!(
        webhook.status().is_success(),
        "{}",
        webhook.text().await.unwrap_or_default()
    );
    let webhook_body: serde_json::Value = webhook.json().await.expect("cancel ack");
    assert_eq!(webhook_body["orderStatus"], "payment_canceled");

    let order_after = app
        .get_json(
            &format!("/api/user/me/shop/orders/{order_id}"),
            &user.access_token,
        )
        .await;
    assert!(order_after.status().is_success());
    let order_after_body: serde_json::Value = order_after.json().await.expect("order after");
    assert_eq!(order_after_body["status"], "payment_canceled");
    assert!(
        order_after_body["failureProblem"]
            .as_str()
            .expect("failure problem")
            .contains("permission_revocation")
    );

    let stackables = app
        .get_json("/api/user/me/inventory/stackables", &user.access_token)
        .await;
    assert!(stackables.status().is_success());
    let stackables_body: serde_json::Value = stackables.json().await.expect("stackables body");
    assert!(stackables_body.as_array().expect("array").is_empty());
}

#[tokio::test]
#[serial]
async fn yookassa_webhook_rejects_malformed_payloads_with_human_readable_problem() {
    let fake_yookassa = FakeYooKassaServer::spawn().await;
    let app = TestApp::spawn_with_yookassa(
        fake_yookassa.base_url(),
        "http://localhost:5173/shop/checkout/return".to_string(),
    )
    .await;

    let response = app
        .post_raw_without_auth(
            "/api/payments/yookassa/webhook",
            serde_json::json!({
                "type": "notification",
                "object": {
                    "id": "missing-event"
                }
            })
            .to_string(),
        )
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body: serde_json::Value = response.json().await.expect("bad webhook body");
    assert_eq!(
        body["problem"]["detail"],
        "YooKassa webhook payload must include string field 'event'"
    );
}

#[tokio::test]
#[serial]
async fn yookassa_webhook_acknowledges_unknown_payment_ids_idempotently() {
    let fake_yookassa = FakeYooKassaServer::spawn().await;
    fake_yookassa
        .register_external_payment("external-payment-1", "pending")
        .await;
    let app = TestApp::spawn_with_yookassa(
        fake_yookassa.base_url(),
        "http://localhost:5173/shop/checkout/return".to_string(),
    )
    .await;

    let response = app
        .post_raw_without_auth(
            "/api/payments/yookassa/webhook",
            serde_json::json!({
                "type": "notification",
                "event": "payment.waiting_for_capture",
                "object": {
                    "id": "external-payment-1",
                    "status": "waiting_for_capture"
                }
            })
            .to_string(),
        )
        .await;
    assert!(
        response.status().is_success(),
        "{}",
        response.text().await.unwrap_or_default()
    );
    let body: serde_json::Value = response.json().await.expect("unknown payment ack");
    assert_eq!(body["ok"], true);
    assert!(body["orderId"].is_null());
    assert!(body["orderStatus"].is_null());
}

#[tokio::test]
#[serial]
async fn mock_orders_expire_and_cannot_be_completed_after_ttl() {
    let app = TestApp::spawn_with_shop_config(ShopConfig {
        payment_provider: ShopPaymentProviderKind::Mock,
        yookassa: None,
        pending_payment_ttl_seconds: 1,
        reconciliation_interval_seconds: 1,
    })
    .await;
    let admin = app.issue_user_token("ShopAdminMockTtl", true, &[]).await;
    let user = app.issue_user_token("ShopBuyerMockTtl", false, &[]).await;

    create_stackable_asset(&app, &admin, "ttl_mock_crate", "TTL Mock Crate").await;
    create_stackable_product(
        &app,
        &admin,
        "ttl_mock_bundle",
        "ttl_mock_crate",
        1,
        Some(2),
        Some(10),
    )
    .await;

    let order = app
        .post_json(
            "/api/user/me/shop/orders",
            &user.access_token,
            serde_json::json!({ "product_key": "ttl_mock_bundle" }),
        )
        .await;
    assert!(order.status().is_success());
    let order_body: serde_json::Value = order.json().await.expect("ttl order");
    let order_id = order_body["id"].as_str().expect("order id");

    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    let expired = app
        .get_json(
            &format!("/api/user/me/shop/orders/{order_id}"),
            &user.access_token,
        )
        .await;
    assert!(expired.status().is_success());
    let expired_body: serde_json::Value = expired.json().await.expect("expired order");
    assert_eq!(expired_body["status"], "payment_expired");
    assert_eq!(
        expired_body["failureProblem"],
        "Payment session expired before the shop order was paid"
    );

    let complete = app
        .post_json(
            &format!("/api/user/me/shop/orders/{order_id}/mock/complete"),
            &user.access_token,
            serde_json::json!({}),
        )
        .await;
    assert_eq!(complete.status(), StatusCode::CONFLICT);
    let complete_body: serde_json::Value = complete.json().await.expect("complete body");
    assert_eq!(
        complete_body["problem"]["detail"],
        "Shop order payment has expired"
    );
}

#[tokio::test]
#[serial]
async fn yookassa_get_order_reconciles_success_without_webhook() {
    let fake_yookassa = FakeYooKassaServer::spawn().await;
    let app = TestApp::spawn_with_yookassa(
        fake_yookassa.base_url(),
        "http://localhost:5173/shop/checkout/return".to_string(),
    )
    .await;
    let admin = app
        .issue_user_token("ShopAdminYkPollSuccess", true, &[])
        .await;
    let user = app
        .issue_user_token("ShopBuyerYkPollSuccess", false, &[])
        .await;

    create_stackable_asset(&app, &admin, "yk_poll_crate", "YK Poll Crate").await;
    create_stackable_product(
        &app,
        &admin,
        "yk_poll_bundle",
        "yk_poll_crate",
        2,
        Some(2),
        Some(10),
    )
    .await;

    let order = app
        .post_json(
            "/api/user/me/shop/orders",
            &user.access_token,
            serde_json::json!({ "product_key": "yk_poll_bundle" }),
        )
        .await;
    assert!(order.status().is_success());
    let order_body: serde_json::Value = order.json().await.expect("order body");
    let order_id = order_body["id"].as_str().expect("order id");
    let payment_id = order_body["payment"]["providerPaymentId"]
        .as_str()
        .expect("payment id")
        .to_string();

    fake_yookassa.set_payment_succeeded(&payment_id).await;

    let reconciled = app
        .get_json(
            &format!("/api/user/me/shop/orders/{order_id}"),
            &user.access_token,
        )
        .await;
    assert!(reconciled.status().is_success());
    let reconciled_body: serde_json::Value = reconciled.json().await.expect("reconciled body");
    assert_eq!(reconciled_body["status"], "fulfilled");

    let stackables = app
        .get_json("/api/user/me/inventory/stackables", &user.access_token)
        .await;
    assert!(stackables.status().is_success());
    let stackables_body: serde_json::Value = stackables.json().await.expect("stackables");
    assert_eq!(stackables_body[0]["assetKey"], "yk_poll_crate");
    assert_eq!(stackables_body[0]["amount"], 2);
}

#[tokio::test]
#[serial]
async fn yookassa_pending_orders_expire_when_payment_never_completes() {
    let fake_yookassa = FakeYooKassaServer::spawn().await;
    let app = TestApp::spawn_with_shop_config(ShopConfig {
        payment_provider: ShopPaymentProviderKind::YooKassa,
        yookassa: Some(auth::app::config::YooKassaConfig {
            shop_id: "test-shop".to_string(),
            secret_key: "test-secret".to_string(),
            api_base_url: fake_yookassa.base_url(),
            return_url: "http://localhost:5173/shop/checkout/return".to_string(),
        }),
        pending_payment_ttl_seconds: 1,
        reconciliation_interval_seconds: 1,
    })
    .await;
    let admin = app.issue_user_token("ShopAdminYkExpire", true, &[]).await;
    let user = app.issue_user_token("ShopBuyerYkExpire", false, &[]).await;

    create_stackable_asset(&app, &admin, "yk_expire_crate", "YK Expire Crate").await;
    create_stackable_product(
        &app,
        &admin,
        "yk_expire_bundle",
        "yk_expire_crate",
        1,
        Some(2),
        Some(10),
    )
    .await;

    let order = app
        .post_json(
            "/api/user/me/shop/orders",
            &user.access_token,
            serde_json::json!({ "product_key": "yk_expire_bundle" }),
        )
        .await;
    assert!(order.status().is_success());
    let order_body: serde_json::Value = order.json().await.expect("order body");
    let order_id = order_body["id"].as_str().expect("order id");

    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    let expired = app
        .get_json(
            &format!("/api/user/me/shop/orders/{order_id}"),
            &user.access_token,
        )
        .await;
    assert!(expired.status().is_success());
    let expired_body: serde_json::Value = expired.json().await.expect("expired body");
    assert_eq!(expired_body["status"], "payment_expired");
}

#[tokio::test]
#[serial]
async fn yookassa_validation_failure_blocks_fulfillment_on_amount_mismatch() {
    let fake_yookassa = FakeYooKassaServer::spawn().await;
    let app = TestApp::spawn_with_yookassa(
        fake_yookassa.base_url(),
        "http://localhost:5173/shop/checkout/return".to_string(),
    )
    .await;
    let admin = app.issue_user_token("ShopAdminYkValidate", true, &[]).await;
    let user = app
        .issue_user_token("ShopBuyerYkValidate", false, &[])
        .await;

    create_stackable_asset(&app, &admin, "yk_validate_crate", "YK Validate Crate").await;
    create_stackable_product(
        &app,
        &admin,
        "yk_validate_bundle",
        "yk_validate_crate",
        1,
        Some(2),
        Some(10),
    )
    .await;

    let order = app
        .post_json(
            "/api/user/me/shop/orders",
            &user.access_token,
            serde_json::json!({ "product_key": "yk_validate_bundle" }),
        )
        .await;
    assert!(order.status().is_success());
    let order_body: serde_json::Value = order.json().await.expect("order body");
    let order_id = order_body["id"].as_str().expect("order id");
    let payment_id = order_body["payment"]["providerPaymentId"]
        .as_str()
        .expect("payment id")
        .to_string();

    fake_yookassa.set_payment_succeeded(&payment_id).await;
    fake_yookassa
        .set_payment_amount(&payment_id, "1.00", "RUB")
        .await;

    let reconciled = app
        .get_json(
            &format!("/api/user/me/shop/orders/{order_id}"),
            &user.access_token,
        )
        .await;
    assert!(reconciled.status().is_success());
    let reconciled_body: serde_json::Value = reconciled.json().await.expect("reconciled body");
    assert_eq!(reconciled_body["status"], "payment_validation_failed");
    assert!(
        reconciled_body["failureProblem"]
            .as_str()
            .expect("failure")
            .contains("amount mismatch")
    );

    let stackables = app
        .get_json("/api/user/me/inventory/stackables", &user.access_token)
        .await;
    assert!(stackables.status().is_success());
    let stackables_body: serde_json::Value = stackables.json().await.expect("stackables");
    assert!(stackables_body.as_array().expect("array").is_empty());
}

#[tokio::test]
#[serial]
async fn unknown_yookassa_webhook_does_not_call_provider_api() {
    let fake_yookassa = FakeYooKassaServer::spawn().await;
    let app = TestApp::spawn_with_yookassa(
        fake_yookassa.base_url(),
        "http://localhost:5173/shop/checkout/return".to_string(),
    )
    .await;

    let response = app
        .post_raw_without_auth(
            "/api/payments/yookassa/webhook",
            serde_json::json!({
                "type": "notification",
                "event": "payment.succeeded",
                "object": {
                    "id": "unknown-payment-id",
                    "status": "succeeded"
                }
            })
            .to_string(),
        )
        .await;
    assert!(response.status().is_success());
    let body: serde_json::Value = response.json().await.expect("webhook body");
    assert_eq!(body["ok"], true);
    assert_eq!(fake_yookassa.get_payment_call_count().await, 0);
}

async fn create_stackable_asset(
    app: &TestApp,
    admin: &common::IssuedUser,
    key: &str,
    display_name: &str,
) {
    app.create_asset(
        admin,
        serde_json::json!({
            "key": key,
            "display_name": display_name,
            "description": null,
            "asset_kind": "item",
            "ownership_model": "stackable",
            "is_currency": false,
            "is_user_purchasable": true,
            "is_public": true,
            "metadata": {}
        }),
    )
    .await;
}

async fn create_stackable_product(
    app: &TestApp,
    admin: &common::IssuedUser,
    product_key: &str,
    asset_key: &str,
    stackable_amount: i64,
    max_per_purchase: Option<i64>,
    max_owned_amount: Option<i64>,
) {
    let response = app
        .post_json(
            "/api/admin/shop/products",
            &admin.access_token,
            serde_json::json!({
                "key": product_key,
                "asset_key": asset_key,
                "price_rub": 100,
                "stackable_amount": stackable_amount,
                "max_per_purchase": max_per_purchase,
                "max_owned_amount": max_owned_amount,
                "locales": [{ "locale": "en", "name": product_key, "description": null }]
            }),
        )
        .await;
    assert!(
        response.status().is_success(),
        "{}",
        response.text().await.unwrap_or_default()
    );
}

#[derive(Clone)]
struct FakeYooKassaServer {
    base_url: String,
    state: Arc<RwLock<FakeYooKassaState>>,
    _server_task: Arc<tokio::task::JoinHandle<()>>,
}

#[derive(Default)]
struct FakeYooKassaState {
    next_payment_number: usize,
    get_payment_calls: usize,
    payments: HashMap<String, serde_json::Value>,
    create_requests: Vec<serde_json::Value>,
}

impl FakeYooKassaServer {
    async fn spawn() -> Self {
        let state = Arc::new(RwLock::new(FakeYooKassaState::default()));
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind fake yookassa");
        let address = listener.local_addr().expect("fake yookassa addr");
        let app = axum::Router::new()
            .route(
                "/v3/payments",
                axum::routing::post(fake_yookassa_create_payment),
            )
            .route(
                "/v3/payments/{payment_id}",
                axum::routing::get(fake_yookassa_get_payment),
            )
            .with_state(state.clone());

        let server_task = Arc::new(tokio::spawn(async move {
            if let Err(error) = axum::serve(listener, app.into_make_service()).await {
                panic!("serve fake yookassa: {error}");
            }
        }));

        for _ in 0..50 {
            if tokio::net::TcpStream::connect(address).await.is_ok() {
                return Self {
                    base_url: format!("http://{address}/v3"),
                    state,
                    _server_task: server_task,
                };
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }

        panic!("fake yookassa server did not become ready in time");
    }

    fn base_url(&self) -> String {
        self.base_url.clone()
    }

    async fn last_create_payment_request(&self) -> Option<serde_json::Value> {
        self.state.read().await.create_requests.last().cloned()
    }

    async fn get_payment_call_count(&self) -> usize {
        self.state.read().await.get_payment_calls
    }

    async fn set_payment_succeeded(&self, payment_id: &str) {
        self.set_payment_status(payment_id, "succeeded", None).await;
    }

    async fn set_payment_canceled(&self, payment_id: &str, reason: &str) {
        self.set_payment_status(payment_id, "canceled", Some(reason))
            .await;
    }

    async fn set_payment_amount(&self, payment_id: &str, value: &str, currency: &str) {
        let mut state = self.state.write().await;
        let payment = state
            .payments
            .get_mut(payment_id)
            .expect("payment exists for amount change");
        payment["amount"] = serde_json::json!({
            "value": value,
            "currency": currency
        });
    }

    async fn register_external_payment(&self, payment_id: &str, status: &str) {
        self.insert_payment(payment_id.to_string(), status, None)
            .await;
    }

    async fn set_payment_status(
        &self,
        payment_id: &str,
        status: &str,
        cancellation_reason: Option<&str>,
    ) {
        self.insert_payment(payment_id.to_string(), status, cancellation_reason)
            .await;
    }

    async fn insert_payment(
        &self,
        payment_id: String,
        status: &str,
        cancellation_reason: Option<&str>,
    ) {
        let mut state = self.state.write().await;
        let mut payment = state
            .payments
            .get(&payment_id)
            .cloned()
            .unwrap_or_else(|| fake_payment_json(&payment_id, status, None));
        payment["status"] = serde_json::json!(status);
        payment["paid"] = serde_json::json!(status == "succeeded");
        if status == "canceled" {
            payment["cancellation_details"] = serde_json::json!({
                "party": "yookassa",
                "reason": cancellation_reason.unwrap_or("unknown")
            });
        } else {
            payment
                .as_object_mut()
                .expect("payment object")
                .remove("cancellation_details");
        }
        state.payments.insert(payment_id, payment);
    }
}

impl Drop for FakeYooKassaServer {
    fn drop(&mut self) {
        self._server_task.abort();
    }
}

async fn fake_yookassa_create_payment(
    axum::extract::State(state): axum::extract::State<Arc<RwLock<FakeYooKassaState>>>,
    Json(payload): Json<serde_json::Value>,
) -> (StatusCode, Json<serde_json::Value>) {
    let mut state = state.write().await;
    state.next_payment_number += 1;
    let payment_id = format!("yk-test-payment-{}", state.next_payment_number);
    state.create_requests.push(payload.clone());
    let payment = fake_payment_json(&payment_id, "pending", Some(payload));
    state.payments.insert(payment_id.clone(), payment.clone());
    (StatusCode::OK, Json(payment))
}

async fn fake_yookassa_get_payment(
    axum::extract::State(state): axum::extract::State<Arc<RwLock<FakeYooKassaState>>>,
    axum::extract::Path(payment_id): axum::extract::Path<String>,
) -> (StatusCode, Json<serde_json::Value>) {
    let mut state = state.write().await;
    state.get_payment_calls += 1;
    let payment = state
        .payments
        .get(&payment_id)
        .cloned()
        .unwrap_or_else(|| serde_json::json!({ "type": "error", "description": "not found" }));
    let status = if state.payments.contains_key(&payment_id) {
        StatusCode::OK
    } else {
        StatusCode::NOT_FOUND
    };
    (status, Json(payment))
}

fn fake_payment_json(
    payment_id: &str,
    status: &str,
    create_payload: Option<serde_json::Value>,
) -> serde_json::Value {
    let metadata = create_payload
        .as_ref()
        .and_then(|payload| payload.get("metadata").cloned())
        .unwrap_or_else(|| serde_json::json!({}));
    let description = create_payload
        .as_ref()
        .and_then(|payload| payload.get("description").cloned())
        .unwrap_or_else(|| serde_json::json!(null));
    serde_json::json!({
        "id": payment_id,
        "status": status,
        "paid": status == "succeeded",
        "amount": {
            "value": create_payload
                .as_ref()
                .and_then(|payload| payload.get("amount"))
                .and_then(|amount| amount.get("value"))
                .cloned()
                .unwrap_or_else(|| serde_json::json!("100.00")),
            "currency": "RUB"
        },
        "description": description,
        "confirmation": {
            "type": "redirect",
            "confirmation_url": format!("https://checkout.test/{payment_id}")
        },
        "metadata": metadata
    })
}
