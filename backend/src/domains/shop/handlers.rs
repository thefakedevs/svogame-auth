use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::response::Redirect;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::app::auth::{get_user_from_headers, require_human_superuser};
use crate::app::http::{HttpError, HttpResult, ProblemResponse};
use crate::app::state::AppStateExtractor;
use crate::services::audit::{
    ACTION_ADMIN_SHOP_PRODUCT_CREATED, ACTION_ADMIN_SHOP_PRODUCT_UPDATED,
    ACTION_USER_SHOP_ORDER_CREATED, ACTION_USER_SHOP_ORDER_FULFILLED, write_audit_log,
};
use crate::services::receipts;
use crate::services::shop::{
    self, CreateShopOrderInput, CreateShopProductInput, ShopCatalogQuery, UpdateShopProductInput,
};

#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct ShopQuery {
    pub locale: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ShopProductLocaleResponse {
    pub locale: String,
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ShopProductResponse {
    pub id: String,
    pub key: String,
    #[schema(rename = "assetDefinitionId")]
    pub asset_definition_id: String,
    #[schema(rename = "assetKey")]
    pub asset_key: String,
    #[schema(rename = "assetDisplayName")]
    pub asset_display_name: String,
    #[schema(rename = "ownershipModel")]
    pub ownership_model: String,
    #[schema(rename = "priceRub")]
    pub price_rub: i64,
    #[schema(rename = "stackableAmount")]
    pub stackable_amount: Option<i64>,
    #[schema(rename = "durationSeconds")]
    pub duration_seconds: Option<i64>,
    #[schema(rename = "maxPerPurchase")]
    pub max_per_purchase: Option<i64>,
    #[schema(rename = "maxOwnedAmount")]
    pub max_owned_amount: Option<i64>,
    #[schema(rename = "localizedName")]
    pub localized_name: String,
    #[schema(rename = "localizedDescription")]
    pub localized_description: Option<String>,
    pub locales: Vec<ShopProductLocaleResponse>,
    #[schema(rename = "isActive")]
    pub is_active: bool,
    #[schema(rename = "isPublic")]
    pub is_public: bool,
    #[schema(rename = "isAvailableNow")]
    pub is_available_now: bool,
    #[schema(rename = "sortOrder")]
    pub sort_order: i32,
    #[schema(rename = "startsAt")]
    pub starts_at: Option<chrono::DateTime<chrono::Utc>>,
    #[schema(rename = "endsAt")]
    pub ends_at: Option<chrono::DateTime<chrono::Utc>>,
    pub metadata: Value,
    #[schema(rename = "createdAt")]
    pub created_at: chrono::DateTime<chrono::Utc>,
    #[schema(rename = "updatedAt")]
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ShopPaymentAttemptResponse {
    pub id: String,
    pub provider: String,
    #[schema(rename = "providerPaymentId")]
    pub provider_payment_id: String,
    #[schema(rename = "checkoutToken")]
    pub checkout_token: Option<String>,
    #[schema(rename = "checkoutUrl")]
    pub checkout_url: Option<String>,
    pub status: String,
    #[schema(rename = "createdAt")]
    pub created_at: chrono::DateTime<chrono::Utc>,
    #[schema(rename = "updatedAt")]
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ShopReceiptResponse {
    pub id: String,
    pub provider: String,
    pub status: String,
    #[schema(rename = "displayStatus")]
    pub display_status: String,
    #[schema(rename = "receiptUuid")]
    pub receipt_uuid: Option<String>,
    #[schema(rename = "printUrl")]
    pub print_url: Option<String>,
    #[schema(rename = "jsonUrl")]
    pub json_url: Option<String>,
    #[schema(rename = "failureProblem")]
    pub failure_problem: Option<String>,
    #[schema(rename = "attemptCount")]
    pub attempt_count: i32,
    #[schema(rename = "nextAttemptAt")]
    pub next_attempt_at: Option<chrono::DateTime<chrono::Utc>>,
    #[schema(rename = "deadlineAt")]
    pub deadline_at: Option<chrono::DateTime<chrono::Utc>>,
    #[schema(rename = "completedAt")]
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    #[schema(rename = "createdAt")]
    pub created_at: chrono::DateTime<chrono::Utc>,
    #[schema(rename = "updatedAt")]
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ShopOrderResponse {
    pub id: String,
    #[schema(rename = "userId")]
    pub user_id: String,
    #[schema(rename = "productId")]
    pub product_id: String,
    #[schema(rename = "productKey")]
    pub product_key: String,
    #[schema(rename = "productName")]
    pub product_name: String,
    #[schema(rename = "productDescription")]
    pub product_description: Option<String>,
    #[schema(rename = "productLocale")]
    pub product_locale: Option<String>,
    #[schema(rename = "assetDefinitionId")]
    pub asset_definition_id: String,
    #[schema(rename = "assetKey")]
    pub asset_key: String,
    #[schema(rename = "ownershipModel")]
    pub ownership_model: String,
    pub quantity: i64,
    #[schema(rename = "unitPriceRub")]
    pub unit_price_rub: i64,
    #[schema(rename = "totalPriceRub")]
    pub total_price_rub: i64,
    #[schema(rename = "grantedAmount")]
    pub granted_amount: Option<i64>,
    #[schema(rename = "grantedDurationSeconds")]
    pub granted_duration_seconds: Option<i64>,
    #[schema(rename = "maxOwnedAmount")]
    pub max_owned_amount: Option<i64>,
    #[schema(rename = "paymentProvider")]
    pub payment_provider: String,
    pub status: String,
    #[schema(rename = "failureProblem")]
    pub failure_problem: Option<String>,
    #[schema(rename = "paymentExpiresAt")]
    pub payment_expires_at: Option<chrono::DateTime<chrono::Utc>>,
    #[schema(rename = "paidAt")]
    pub paid_at: Option<chrono::DateTime<chrono::Utc>>,
    #[schema(rename = "fulfilledAt")]
    pub fulfilled_at: Option<chrono::DateTime<chrono::Utc>>,
    pub payment: Option<ShopPaymentAttemptResponse>,
    pub receipt: Option<ShopReceiptResponse>,
    pub metadata: Value,
    #[schema(rename = "createdAt")]
    pub created_at: chrono::DateTime<chrono::Utc>,
    #[schema(rename = "updatedAt")]
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ShopWebhookAckResponse {
    pub ok: bool,
    pub provider: String,
    pub event: String,
    #[schema(rename = "providerPaymentId")]
    pub provider_payment_id: String,
    #[schema(rename = "orderId")]
    pub order_id: Option<String>,
    #[schema(rename = "orderStatus")]
    pub order_status: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct YooKassaWebhookRequest {
    #[schema(rename = "type")]
    pub kind: String,
    pub event: String,
    pub object: YooKassaWebhookObjectRequest,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct YooKassaWebhookObjectRequest {
    pub id: String,
    pub status: Option<String>,
}

#[utoipa::path(
    get,
    path = "/api/shop/products",
    params(ShopQuery),
    responses(
        (status = 200, description = "List publicly visible shop products that are currently available for purchase.", body = [ShopProductResponse])
    ),
    tag = "shop"
)]
pub async fn list_public_products(
    State(state): AppStateExtractor,
    Query(query): Query<ShopQuery>,
) -> HttpResult<Json<Value>> {
    let state = state.read().await;
    let items = shop::list_products(
        &state.db,
        ShopCatalogQuery {
            locale: query.locale,
        },
        false,
    )
    .await
    .map_err(map_shop_error)?;
    Ok(Json(Value::Array(
        items.into_iter().map(product_json).collect(),
    )))
}

#[utoipa::path(
    get,
    path = "/api/shop/products/{product_key}",
    params(
        ("product_key" = String, Path, description = "Shop product key."),
        ShopQuery
    ),
    responses(
        (status = 200, description = "Get one publicly visible shop product.", body = ShopProductResponse),
        (status = 404, description = "Shop product not found.", body = ProblemResponse)
    ),
    tag = "shop"
)]
pub async fn get_public_product(
    State(state): AppStateExtractor,
    Path(product_key): Path<String>,
    Query(query): Query<ShopQuery>,
) -> HttpResult<Json<Value>> {
    let state = state.read().await;
    let item = shop::get_product_by_key(&state.db, &product_key, query.locale, false)
        .await
        .map_err(map_shop_error)?
        .ok_or_else(|| HttpError::not_found("Shop product not found"))?;
    Ok(Json(product_json(item)))
}

#[utoipa::path(
    get,
    path = "/api/user/me/shop/orders",
    responses(
        (status = 200, description = "List shop orders created by the authenticated user.", body = [ShopOrderResponse]),
        (status = 401, description = "Missing bearer token.", body = ProblemResponse),
        (status = 403, description = "Token invalid or user inactive.", body = ProblemResponse)
    ),
    security(("bearer_auth" = [])),
    tag = "shop"
)]
pub async fn list_my_orders(
    State(state): AppStateExtractor,
    headers: HeaderMap,
) -> HttpResult<Json<Value>> {
    let state = state.read().await;
    let user = get_user_from_headers(&headers, &state).await?;
    let items = shop::list_orders_for_user(&state.db, user.id, &state.config.shop)
        .await
        .map_err(map_shop_error)?;
    Ok(Json(Value::Array(
        items.into_iter().map(order_json).collect(),
    )))
}

#[utoipa::path(
    get,
    path = "/api/user/me/shop/orders/{order_id}",
    params(
        ("order_id" = String, Path, description = "Shop order UUID.")
    ),
    responses(
        (status = 200, description = "Get one shop order created by the authenticated user.", body = ShopOrderResponse),
        (status = 404, description = "Shop order not found.", body = ProblemResponse)
    ),
    security(("bearer_auth" = [])),
    tag = "shop"
)]
pub async fn get_my_order(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Path(order_id): Path<String>,
) -> HttpResult<Json<Value>> {
    let state = state.read().await;
    let user = get_user_from_headers(&headers, &state).await?;
    let order_id = parse_uuid(&order_id, "Invalid shop order ID")?;
    let item = shop::get_order_for_user(&state.db, user.id, order_id, &state.config.shop)
        .await
        .map_err(map_shop_error)?
        .ok_or_else(|| HttpError::not_found("Shop order not found"))?;
    Ok(Json(order_json(item)))
}

#[utoipa::path(
    get,
    path = "/api/user/me/shop/orders/{order_id}/receipt",
    params(
        ("order_id" = String, Path, description = "Shop order UUID.")
    ),
    responses(
        (status = 200, description = "Get receipt status for a completed shop order.", body = ShopReceiptResponse),
        (status = 404, description = "Receipt or shop order not found.", body = ProblemResponse)
    ),
    security(("bearer_auth" = [])),
    tag = "shop"
)]
pub async fn get_my_order_receipt(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Path(order_id): Path<String>,
) -> HttpResult<Json<Value>> {
    let state = state.read().await;
    let user = get_user_from_headers(&headers, &state).await?;
    let order_id = parse_uuid(&order_id, "Invalid shop order ID")?;
    let receipt = receipts::get_receipt_for_user_order(&state.db, user.id, order_id)
        .await
        .map_err(map_shop_error)?
        .ok_or_else(|| HttpError::not_found("Shop receipt not found"))?;
    Ok(Json(receipt_json(receipt)))
}

#[utoipa::path(
    get,
    path = "/api/user/me/shop/orders/{order_id}/receipt/print",
    params(
        ("order_id" = String, Path, description = "Shop order UUID.")
    ),
    responses(
        (status = 307, description = "Redirects to printable receipt URL."),
        (status = 404, description = "Receipt or shop order not found.", body = ProblemResponse),
        (status = 409, description = "Receipt is not completed yet.", body = ProblemResponse)
    ),
    security(("bearer_auth" = [])),
    tag = "shop"
)]
pub async fn print_my_order_receipt(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Path(order_id): Path<String>,
) -> HttpResult<Redirect> {
    let state = state.read().await;
    let user = get_user_from_headers(&headers, &state).await?;
    let order_id = parse_uuid(&order_id, "Invalid shop order ID")?;
    let receipt = receipts::get_receipt_for_user_order(&state.db, user.id, order_id)
        .await
        .map_err(map_shop_error)?
        .ok_or_else(|| HttpError::not_found("Shop receipt not found"))?;
    if receipt.status != receipts::RECEIPT_STATUS_COMPLETED {
        return Err(HttpError::new(
            axum::http::StatusCode::CONFLICT,
            "Shop receipt is not completed yet",
        ));
    }
    let print_url = receipt
        .print_url
        .ok_or_else(|| HttpError::not_found("Shop receipt print URL not found"))?;
    Ok(Redirect::temporary(&print_url))
}

#[utoipa::path(
    post,
    path = "/api/user/me/shop/orders",
    request_body(
        content = CreateShopOrderInput,
        description = "Create a new shop order and initialize a payment attempt. For stackable products `quantity` is the number of bundles purchased; for entitlement and subscription products it must be omitted or equal to 1."
    ),
    responses(
        (status = 200, description = "Shop order created.", body = ShopOrderResponse),
        (status = 400, description = "Invalid request payload.", body = ProblemResponse),
        (status = 404, description = "Shop product not found.", body = ProblemResponse),
        (status = 409, description = "Product cannot be purchased because the user already owns the entitlement.", body = ProblemResponse),
        (status = 422, description = "Purchase violates availability or ownership limit rules.", body = ProblemResponse),
        (status = 503, description = "Payment provider is disabled by configuration.", body = ProblemResponse)
    ),
    security(("bearer_auth" = [])),
    tag = "shop"
)]
pub async fn create_my_order(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Json(body): Json<CreateShopOrderInput>,
) -> HttpResult<Json<Value>> {
    let state = state.read().await;
    let user = get_user_from_headers(&headers, &state).await?;
    let item = shop::create_order(&state.db, user.id, body, state.config.shop.clone())
        .await
        .map_err(map_shop_error)?;
    write_audit_log(
        &state.db,
        ACTION_USER_SHOP_ORDER_CREATED,
        Some(user.id),
        Some(user.id),
        None,
        Some(json!({ "orderId": item.id, "productKey": item.product_key })),
    )
    .await
    .map_err(|error| HttpError::internal_error(format!("Failed to write audit log: {error}")))?;
    Ok(Json(order_json(item)))
}

#[utoipa::path(
    post,
    path = "/api/payments/yookassa/webhook",
    request_body(
        content = YooKassaWebhookRequest,
        description = "Raw YooKassa notification payload."
    ),
    responses(
        (status = 200, description = "Webhook processed or acknowledged idempotently.", body = ShopWebhookAckResponse),
        (status = 400, description = "Malformed YooKassa notification payload.", body = ProblemResponse),
        (status = 503, description = "YooKassa provider is not enabled in configuration.", body = ProblemResponse)
    ),
    tag = "shop"
)]
pub async fn yookassa_webhook(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    body: String,
) -> HttpResult<Json<Value>> {
    let state = state.read().await;
    let ack = shop::handle_yookassa_webhook(&state.db, &state.config.shop, &headers, &body)
        .await
        .map_err(map_shop_error)?;
    Ok(Json(webhook_ack_json(ack)))
}

#[utoipa::path(
    post,
    path = "/api/user/me/shop/orders/{order_id}/mock/complete",
    params(
        ("order_id" = String, Path, description = "Shop order UUID.")
    ),
    responses(
        (status = 200, description = "Complete a mock payment and attempt to fulfill the order.", body = ShopOrderResponse),
        (status = 400, description = "Order is invalid for mock completion.", body = ProblemResponse),
        (status = 404, description = "Shop order not found.", body = ProblemResponse),
        (status = 409, description = "Order is in a conflicting state or fulfillment failed after payment.", body = ProblemResponse)
    ),
    security(("bearer_auth" = [])),
    tag = "shop"
)]
pub async fn complete_my_mock_order(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Path(order_id): Path<String>,
) -> HttpResult<Json<Value>> {
    let state = state.read().await;
    let user = get_user_from_headers(&headers, &state).await?;
    let order_id = parse_uuid(&order_id, "Invalid shop order ID")?;
    let item = shop::complete_mock_order_payment(&state.db, user.id, order_id, &state.config.shop)
        .await
        .map_err(map_shop_error)?;
    if item.status == "fulfilled" {
        write_audit_log(
            &state.db,
            ACTION_USER_SHOP_ORDER_FULFILLED,
            Some(user.id),
            Some(user.id),
            None,
            Some(json!({ "orderId": item.id, "productKey": item.product_key })),
        )
        .await
        .map_err(|error| {
            HttpError::internal_error(format!("Failed to write audit log: {error}"))
        })?;
    }
    if item.status == "fulfillment_failed" {
        return Err(HttpError::new(
            axum::http::StatusCode::CONFLICT,
            item.failure_problem.unwrap_or_else(|| {
                "Order payment succeeded, but asset fulfillment failed".to_string()
            }),
        ));
    }
    Ok(Json(order_json(item)))
}

#[utoipa::path(
    get,
    path = "/api/admin/shop/products",
    params(ShopQuery),
    responses(
        (status = 200, description = "Administrative list of all shop products.", body = [ShopProductResponse])
    ),
    security(("bearer_auth" = [])),
    tag = "shop-admin"
)]
pub async fn list_admin_products(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Query(query): Query<ShopQuery>,
) -> HttpResult<Json<Value>> {
    let state = state.read().await;
    require_human_superuser(&headers, &state).await?;
    let items = shop::list_products(
        &state.db,
        ShopCatalogQuery {
            locale: query.locale,
        },
        true,
    )
    .await
    .map_err(map_shop_error)?;
    Ok(Json(Value::Array(
        items.into_iter().map(product_json).collect(),
    )))
}

#[utoipa::path(
    get,
    path = "/api/admin/shop/products/{product_id}",
    params(
        ("product_id" = String, Path, description = "Shop product UUID."),
        ShopQuery
    ),
    responses(
        (status = 200, description = "Get one shop product in administrative view.", body = ShopProductResponse),
        (status = 404, description = "Shop product not found.", body = ProblemResponse)
    ),
    security(("bearer_auth" = [])),
    tag = "shop-admin"
)]
pub async fn get_admin_product(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Path(product_id): Path<String>,
    Query(query): Query<ShopQuery>,
) -> HttpResult<Json<Value>> {
    let state = state.read().await;
    require_human_superuser(&headers, &state).await?;
    let product_id = parse_uuid(&product_id, "Invalid shop product ID")?;
    let item = shop::get_product_by_id(&state.db, product_id, query.locale)
        .await
        .map_err(map_shop_error)?
        .ok_or_else(|| HttpError::not_found("Shop product not found"))?;
    Ok(Json(product_json(item)))
}

#[utoipa::path(
    post,
    path = "/api/admin/shop/products",
    request_body(
        content = CreateShopProductInput,
        description = "Create a shop product bound to an existing non-currency asset. Stackable products require `stackable_amount`; subscription products require `durationSeconds`; entitlement products must not specify either."
    ),
    responses(
        (status = 200, description = "Shop product created.", body = ShopProductResponse),
        (status = 400, description = "Validation error.", body = ProblemResponse),
        (status = 409, description = "Shop product key already exists.", body = ProblemResponse)
    ),
    security(("bearer_auth" = [])),
    tag = "shop-admin"
)]
pub async fn create_product(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Json(body): Json<CreateShopProductInput>,
) -> HttpResult<Json<Value>> {
    let state = state.read().await;
    let actor = require_human_superuser(&headers, &state).await?;
    let item = shop::create_product(&state.db, body)
        .await
        .map_err(map_shop_error)?;
    write_audit_log(
        &state.db,
        ACTION_ADMIN_SHOP_PRODUCT_CREATED,
        Some(actor.id),
        None,
        None,
        Some(json!({ "productId": item.id, "productKey": item.key })),
    )
    .await
    .map_err(|error| HttpError::internal_error(format!("Failed to write audit log: {error}")))?;
    Ok(Json(product_json(item)))
}

#[utoipa::path(
    patch,
    path = "/api/admin/shop/products/{product_id}",
    params(
        ("product_id" = String, Path, description = "Shop product UUID.")
    ),
    request_body(
        content = UpdateShopProductInput,
        description = "Patch mutable shop product fields and optionally replace the localized title set."
    ),
    responses(
        (status = 200, description = "Shop product updated.", body = ShopProductResponse),
        (status = 400, description = "Validation error.", body = ProblemResponse),
        (status = 404, description = "Shop product not found.", body = ProblemResponse)
    ),
    security(("bearer_auth" = [])),
    tag = "shop-admin"
)]
pub async fn patch_product(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Path(product_id): Path<String>,
    Json(body): Json<UpdateShopProductInput>,
) -> HttpResult<Json<Value>> {
    let state = state.read().await;
    let actor = require_human_superuser(&headers, &state).await?;
    let product_id = parse_uuid(&product_id, "Invalid shop product ID")?;
    let item = shop::update_product(&state.db, product_id, body)
        .await
        .map_err(map_shop_error)?;
    write_audit_log(
        &state.db,
        ACTION_ADMIN_SHOP_PRODUCT_UPDATED,
        Some(actor.id),
        None,
        None,
        Some(json!({ "productId": item.id, "productKey": item.key })),
    )
    .await
    .map_err(|error| HttpError::internal_error(format!("Failed to write audit log: {error}")))?;
    Ok(Json(product_json(item)))
}

fn product_json(item: shop::ShopProductView) -> Value {
    json!({
        "id": item.id,
        "key": item.key,
        "assetDefinitionId": item.asset_definition_id,
        "assetKey": item.asset_key,
        "assetDisplayName": item.asset_display_name,
        "ownershipModel": item.ownership_model,
        "priceRub": item.price_rub,
        "stackableAmount": item.stackable_amount,
        "durationSeconds": item.duration_seconds,
        "maxPerPurchase": item.max_per_purchase,
        "maxOwnedAmount": item.max_owned_amount,
        "localizedName": item.localized_name,
        "localizedDescription": item.localized_description,
        "locales": item.locales.into_iter().map(locale_json).collect::<Vec<_>>(),
        "isActive": item.is_active,
        "isPublic": item.is_public,
        "isAvailableNow": item.is_available_now,
        "sortOrder": item.sort_order,
        "startsAt": item.starts_at,
        "endsAt": item.ends_at,
        "metadata": item.metadata,
        "createdAt": item.created_at,
        "updatedAt": item.updated_at
    })
}

fn locale_json(item: shop::ShopProductLocaleView) -> Value {
    json!({
        "locale": item.locale,
        "name": item.name,
        "description": item.description
    })
}

fn payment_json(item: shop::ShopPaymentAttemptView) -> Value {
    json!({
        "id": item.id,
        "provider": item.provider,
        "providerPaymentId": item.provider_payment_id,
        "checkoutToken": item.checkout_token,
        "checkoutUrl": item.checkout_url,
        "status": item.status,
        "createdAt": item.created_at,
        "updatedAt": item.updated_at
    })
}

fn receipt_json(item: receipts::ShopReceiptView) -> Value {
    json!({
        "id": item.id,
        "provider": item.provider,
        "status": item.status,
        "displayStatus": item.display_status,
        "receiptUuid": item.receipt_uuid,
        "printUrl": item.print_url,
        "jsonUrl": item.json_url,
        "failureProblem": item.failure_problem,
        "attemptCount": item.attempt_count,
        "nextAttemptAt": item.next_attempt_at,
        "deadlineAt": item.deadline_at,
        "completedAt": item.completed_at,
        "createdAt": item.created_at,
        "updatedAt": item.updated_at
    })
}

fn order_json(item: shop::ShopOrderView) -> Value {
    json!({
        "id": item.id,
        "userId": item.user_id,
        "productId": item.product_id,
        "productKey": item.product_key,
        "productName": item.product_name,
        "productDescription": item.product_description,
        "productLocale": item.product_locale,
        "assetDefinitionId": item.asset_definition_id,
        "assetKey": item.asset_key,
        "ownershipModel": item.ownership_model,
        "quantity": item.quantity,
        "unitPriceRub": item.unit_price_rub,
        "totalPriceRub": item.total_price_rub,
        "grantedAmount": item.granted_amount,
        "grantedDurationSeconds": item.granted_duration_seconds,
        "maxOwnedAmount": item.max_owned_amount,
        "paymentProvider": item.payment_provider,
        "status": item.status,
        "failureProblem": item.failure_problem,
        "paymentExpiresAt": item.payment_expires_at,
        "paidAt": item.paid_at,
        "fulfilledAt": item.fulfilled_at,
        "payment": item.payment.map(payment_json),
        "receipt": item.receipt.map(receipt_json),
        "metadata": item.metadata,
        "createdAt": item.created_at,
        "updatedAt": item.updated_at
    })
}

fn webhook_ack_json(item: shop::ShopWebhookAckView) -> Value {
    json!({
        "ok": item.ok,
        "provider": item.provider,
        "event": item.event,
        "providerPaymentId": item.provider_payment_id,
        "orderId": item.order_id,
        "orderStatus": item.order_status
    })
}

fn map_shop_error(error: anyhow::Error) -> HttpError {
    let message = error.to_string();
    if message.starts_with("not_found: ") {
        HttpError::not_found(shop::strip_problem_prefix(message))
    } else if message.starts_with("conflict: ") {
        HttpError::new(
            axum::http::StatusCode::CONFLICT,
            shop::strip_problem_prefix(message),
        )
    } else if message.starts_with("unprocessable: ") {
        HttpError::new(
            axum::http::StatusCode::UNPROCESSABLE_ENTITY,
            shop::strip_problem_prefix(message),
        )
    } else if message.starts_with("bad_request: ") {
        HttpError::bad_request(shop::strip_problem_prefix(message))
    } else if message.starts_with("unavailable: ") {
        HttpError::new(
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            shop::strip_problem_prefix(message),
        )
    } else {
        HttpError::internal_error(message)
    }
}

fn parse_uuid(value: &str, message: &str) -> HttpResult<Uuid> {
    Uuid::parse_str(value).map_err(|_| HttpError::bad_request(message))
}
