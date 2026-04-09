use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::app::auth::{PrivilegedActor, get_user_from_headers, require_privileged_actor};
use crate::app::http::{HttpError, HttpResult};
use crate::app::state::AppStateExtractor;
use crate::services::audit::{
    ACTION_ADMIN_ASSET_CREATED, ACTION_ADMIN_ASSET_UPDATED,
    ACTION_ADMIN_INVENTORY_ENTITLEMENT_GRANTED, ACTION_ADMIN_INVENTORY_ENTITLEMENT_REVOKED,
    ACTION_ADMIN_INVENTORY_EXPIRABLE_EXPIRATION_SET, ACTION_ADMIN_INVENTORY_EXPIRABLE_PROLONGED,
    ACTION_ADMIN_INVENTORY_EXPIRABLE_REVOKED, ACTION_ADMIN_INVENTORY_STACKABLE_ADDED,
    ACTION_ADMIN_INVENTORY_STACKABLE_REMOVED, ACTION_ADMIN_INVENTORY_STACKABLE_SET,
    ACTION_ADMIN_WALLET_ADJUSTED, ACTION_ADMIN_WALLET_CREDITED, ACTION_ADMIN_WALLET_DEBITED,
    write_audit_log,
};
use crate::services::ownership::catalog::{
    AssetDefinitionQuery, CreateAssetDefinitionInput, UpdateAssetDefinitionInput,
};
use crate::services::ownership::inventory::{
    EntitlementMutation, ProlongExpirableMutation, SetExpirationMutation, StackableMutation,
    SubscriptionMutation, SubscriptionStatus,
};
use crate::services::ownership::types::{OperationContext, OwnershipActor};
use crate::services::ownership::wallet::WalletMutation;
use crate::services::ownership::{catalog, inventory, wallet};

#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct AssetListQuery {
    /// Free-text search across asset key and display name.
    pub q: Option<String>,
    #[serde(rename = "assetKind")]
    /// Filter by semantic asset kind.
    pub asset_kind: Option<String>,
    #[serde(rename = "ownershipModel")]
    /// Filter by ownership model: `stackable`, `entitlement`, `expirable`.
    pub ownership_model: Option<String>,
    #[serde(rename = "isCurrency")]
    /// Filter currency assets. Currency assets are served by wallet, not inventory.
    pub is_currency: Option<bool>,
    #[serde(rename = "isPublic")]
    /// Filter by public visibility flag.
    pub is_public: Option<bool>,
    #[serde(rename = "isUserPurchasable")]
    /// Filter by direct purchasability flag.
    pub is_user_purchasable: Option<bool>,
    #[serde(rename = "isActive")]
    /// Filter active/inactive catalog entries.
    pub is_active: Option<bool>,
    /// 1-based page number. Defaults to 1.
    pub page: Option<u64>,
    #[serde(rename = "perPage")]
    /// Page size. Defaults to 20 and is capped server-side.
    pub per_page: Option<u64>,
}

#[derive(Debug, Deserialize, Clone, ToSchema)]
pub struct MutationBody {
    /// Amount for stackable or wallet operations. Positive for add/remove/credit/debit. Non-negative for set/adjust.
    pub amount: Option<i64>,
    #[serde(rename = "durationSeconds")]
    /// Duration in seconds to prolong an expirable asset. Required for prolong endpoint.
    pub duration_seconds: Option<i64>,
    #[serde(rename = "expiresAt")]
    /// Absolute expiration timestamp for set-expiration endpoint.
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(rename = "reasonCode")]
    /// Structured machine-readable reason code used for analytics, support tooling, or downstream automation.
    pub reason_code: Option<String>,
    #[serde(rename = "reasonText")]
    /// Human-readable explanation shown in audit and investigation flows.
    pub reason_text: Option<String>,
    /// Arbitrary JSON metadata attached to inventory or wallet journal records.
    pub metadata: Option<Value>,
}

#[derive(Serialize, ToSchema)]
pub struct AssetResponse {
    pub id: String,
    pub key: String,
    #[schema(rename = "displayName")]
    pub display_name: String,
    pub description: Option<String>,
    #[schema(rename = "assetKind")]
    pub asset_kind: String,
    #[schema(rename = "ownershipModel")]
    pub ownership_model: String,
    #[schema(rename = "isCurrency")]
    pub is_currency: bool,
    #[schema(rename = "isUserPurchasable")]
    pub is_user_purchasable: bool,
    #[schema(rename = "isPublic")]
    pub is_public: bool,
    #[schema(rename = "isActive")]
    pub is_active: bool,
    pub metadata: Value,
    #[schema(rename = "createdAt")]
    pub created_at: chrono::DateTime<chrono::Utc>,
    #[schema(rename = "updatedAt")]
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Serialize, ToSchema)]
pub struct AssetListResponse {
    pub items: Vec<AssetResponse>,
    pub total: u64,
    pub page: u64,
    #[schema(rename = "perPage")]
    pub per_page: u64,
    #[schema(rename = "totalPages")]
    pub total_pages: u64,
}

#[derive(Serialize, ToSchema)]
pub struct StackableResponse {
    #[schema(rename = "assetKey")]
    pub asset_key: String,
    #[schema(rename = "assetDefinitionId")]
    pub asset_definition_id: String,
    pub amount: i64,
    #[schema(rename = "updatedAt")]
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Serialize, ToSchema)]
pub struct EntitlementResponse {
    #[schema(rename = "assetKey")]
    pub asset_key: String,
    #[schema(rename = "assetDefinitionId")]
    pub asset_definition_id: String,
    #[schema(rename = "grantedAt")]
    pub granted_at: chrono::DateTime<chrono::Utc>,
    #[schema(rename = "updatedAt")]
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Serialize, ToSchema)]
pub struct ExpirableResponse {
    #[schema(rename = "assetKey")]
    pub asset_key: String,
    #[schema(rename = "assetDefinitionId")]
    pub asset_definition_id: String,
    #[schema(rename = "expiresAt")]
    pub expires_at: chrono::DateTime<chrono::Utc>,
    #[schema(rename = "grantedAt")]
    pub granted_at: chrono::DateTime<chrono::Utc>,
    #[schema(rename = "lastExtendedAt")]
    pub last_extended_at: Option<chrono::DateTime<chrono::Utc>>,
    #[schema(rename = "updatedAt")]
    pub updated_at: chrono::DateTime<chrono::Utc>,
    #[schema(rename = "isActive")]
    pub is_active: bool,
}

#[derive(Serialize, ToSchema)]
pub struct InventoryResponse {
    #[schema(rename = "userId")]
    pub user_id: String,
    pub stackables: Vec<StackableResponse>,
    pub entitlements: Vec<EntitlementResponse>,
    pub expirables: Vec<ExpirableResponse>,
}

#[derive(Serialize, ToSchema)]
pub struct InventoryPresenceResponse {
    #[schema(rename = "assetKey")]
    pub asset_key: String,
    #[schema(rename = "ownershipModel")]
    pub ownership_model: String,
    pub exists: bool,
    pub amount: Option<i64>,
    #[schema(rename = "isActive")]
    pub is_active: Option<bool>,
    #[schema(rename = "expiresAt")]
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Serialize, ToSchema)]
pub struct InventoryOperationResponse {
    pub id: i64,
    #[schema(rename = "assetDefinitionId")]
    pub asset_definition_id: String,
    #[schema(rename = "assetKey")]
    pub asset_key: String,
    #[schema(rename = "ownershipModel")]
    pub ownership_model: String,
    #[schema(rename = "operationType")]
    pub operation_type: String,
    #[schema(rename = "actorKind")]
    pub actor_kind: String,
    #[schema(rename = "actorUserId")]
    pub actor_user_id: Option<String>,
    #[schema(rename = "actorServiceName")]
    pub actor_service_name: Option<String>,
    #[schema(rename = "deltaAmount")]
    pub delta_amount: Option<i64>,
    #[schema(rename = "newAmount")]
    pub new_amount: Option<i64>,
    #[schema(rename = "previousExpiresAt")]
    pub previous_expires_at: Option<chrono::DateTime<chrono::Utc>>,
    #[schema(rename = "newExpiresAt")]
    pub new_expires_at: Option<chrono::DateTime<chrono::Utc>>,
    #[schema(rename = "reasonCode")]
    pub reason_code: Option<String>,
    #[schema(rename = "reasonText")]
    pub reason_text: Option<String>,
    pub metadata: Value,
    #[schema(rename = "createdAt")]
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Serialize, ToSchema)]
pub struct WalletBalanceResponse {
    #[schema(rename = "userId")]
    pub user_id: String,
    #[schema(rename = "currencyAssetDefinitionId")]
    pub currency_asset_definition_id: String,
    #[schema(rename = "currencyKey")]
    pub currency_key: String,
    pub balance: i64,
    #[schema(rename = "updatedAt")]
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Serialize, ToSchema)]
pub struct WalletTransactionResponse {
    pub id: i64,
    #[schema(rename = "currencyAssetDefinitionId")]
    pub currency_asset_definition_id: String,
    #[schema(rename = "currencyKey")]
    pub currency_key: String,
    #[schema(rename = "operationType")]
    pub operation_type: String,
    #[schema(rename = "actorKind")]
    pub actor_kind: String,
    #[schema(rename = "actorUserId")]
    pub actor_user_id: Option<String>,
    #[schema(rename = "actorServiceName")]
    pub actor_service_name: Option<String>,
    pub delta: i64,
    #[schema(rename = "balanceAfter")]
    pub balance_after: i64,
    #[schema(rename = "reasonCode")]
    pub reason_code: Option<String>,
    #[schema(rename = "reasonText")]
    pub reason_text: Option<String>,
    pub metadata: Value,
    #[schema(rename = "createdAt")]
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Serialize, ToSchema)]
pub struct SubscriptionStatusResponse {
    #[schema(rename = "userId")]
    pub user_id: String,
    pub status: String,
}

#[derive(Serialize, ToSchema)]
pub struct OkResponse {
    pub ok: bool,
}

#[utoipa::path(
    get,
    path = "/api/assets",
    params(AssetListQuery),
    responses(
        (status = 200, description = "List public asset definitions. Only assets with `is_public = true` and `is_active = true` are returned here.", body = AssetListResponse),
        (status = 400, description = "Invalid query filter, for example an unsupported asset kind or ownership model.")
    ),
    tag = "ownership"
)]
pub async fn list_public_assets(
    State(state): AppStateExtractor,
    Query(query): Query<AssetListQuery>,
) -> HttpResult<Json<Value>> {
    let state = state.read().await;
    let assets = catalog::list_asset_definitions(&state.db, map_asset_query(query), false)
        .await
        .map_err(map_domain_error)?;
    Ok(Json(asset_list_json(assets)))
}

#[utoipa::path(
    get,
    path = "/api/assets/{asset_id}",
    params(
        ("asset_id" = String, Path, description = "Asset UUID.")
    ),
    responses(
        (status = 200, description = "Get one public asset definition. Inactive or non-public assets are hidden behind 404.", body = AssetResponse),
        (status = 400, description = "Invalid asset ID."),
        (status = 404, description = "Asset not found or not publicly visible.")
    ),
    tag = "ownership"
)]
pub async fn get_public_asset(
    State(state): AppStateExtractor,
    Path(asset_id): Path<String>,
) -> HttpResult<Json<Value>> {
    let state = state.read().await;
    let asset_id = parse_uuid(&asset_id, "Invalid asset ID")?;
    let asset = catalog::get_asset_definition_by_id(&state.db, asset_id)
        .await
        .map_err(map_domain_error)?
        .filter(|asset| asset.is_public && asset.is_active)
        .ok_or_else(|| HttpError::not_found("Asset not found"))?;
    Ok(Json(asset_json(asset)))
}

#[utoipa::path(
    get,
    path = "/api/admin/assets",
    params(AssetListQuery),
    responses(
        (status = 200, description = "Admin list of asset definitions. Unlike the public catalog, this can include inactive and non-public assets.", body = AssetListResponse),
        (status = 401, description = "Missing bearer token."),
        (status = 403, description = "Superuser permissions required."),
        (status = 400, description = "Invalid query filter.")
    ),
    security(("bearer_auth" = [])),
    tag = "ownership-admin"
)]
pub async fn list_admin_assets(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Query(query): Query<AssetListQuery>,
) -> HttpResult<Json<Value>> {
    let state = state.read().await;
    require_privileged_actor(&headers, &state).await?;
    let assets = catalog::list_asset_definitions(&state.db, map_asset_query(query), true)
        .await
        .map_err(map_domain_error)?;
    Ok(Json(asset_list_json(assets)))
}

#[utoipa::path(
    get,
    path = "/api/admin/assets/{asset_id}",
    params(
        ("asset_id" = String, Path, description = "Asset UUID.")
    ),
    responses(
        (status = 200, description = "Admin-only full asset definition view.", body = AssetResponse),
        (status = 400, description = "Invalid asset ID."),
        (status = 401, description = "Missing bearer token."),
        (status = 403, description = "Superuser permissions required."),
        (status = 404, description = "Asset not found.")
    ),
    security(("bearer_auth" = [])),
    tag = "ownership-admin"
)]
pub async fn get_admin_asset(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Path(asset_id): Path<String>,
) -> HttpResult<Json<Value>> {
    let state = state.read().await;
    require_privileged_actor(&headers, &state).await?;
    let asset_id = parse_uuid(&asset_id, "Invalid asset ID")?;
    let asset = catalog::get_asset_definition_by_id(&state.db, asset_id)
        .await
        .map_err(map_domain_error)?
        .ok_or_else(|| HttpError::not_found("Asset not found"))?;
    Ok(Json(asset_json(asset)))
}

#[utoipa::path(
    post,
    path = "/api/admin/assets",
    request_body(
        content = CreateAssetDefinitionInput,
        description = "Create a new asset definition. `key` must be lowercase URL-safe (`[a-z0-9_-]+`). Currency assets must use `asset_kind = currency`, `is_currency = true`, and `ownership_model = stackable`."
    ),
    responses(
        (status = 200, description = "Asset definition created.", body = AssetResponse),
        (status = 400, description = "Validation error. Typical edge cases: duplicate key, invalid key format, empty display name, or currency/model mismatch."),
        (status = 401, description = "Missing bearer token."),
        (status = 403, description = "Superuser permissions required.")
    ),
    security(("bearer_auth" = [])),
    tag = "ownership-admin"
)]
pub async fn create_asset(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Json(body): Json<CreateAssetDefinitionInput>,
) -> HttpResult<Json<Value>> {
    let state = state.read().await;
    let actor = require_privileged_actor(&headers, &state).await?;
    let asset = catalog::create_asset_definition(&state.db, body)
        .await
        .map_err(map_domain_error)?;
    write_audit_log(
        &state.db,
        ACTION_ADMIN_ASSET_CREATED,
        actor.actor_user_id(),
        None,
        None,
        Some(with_actor_metadata(
            json!({ "assetId": asset.id, "assetKey": asset.key }),
            &actor,
        )),
    )
    .await
    .map_err(|error| HttpError::internal_error(format!("Failed to write audit log: {error}")))?;
    Ok(Json(asset_json(asset)))
}

#[utoipa::path(
    patch,
    path = "/api/admin/assets/{asset_id}",
    params(
        ("asset_id" = String, Path, description = "Asset UUID.")
    ),
    request_body(
        content = UpdateAssetDefinitionInput,
        description = "Patch mutable asset fields. This endpoint intentionally does not expose changes to the fundamental storage semantics such as ownership model."
    ),
    responses(
        (status = 200, description = "Asset definition updated.", body = AssetResponse),
        (status = 400, description = "Invalid asset ID or invalid patch payload."),
        (status = 401, description = "Missing bearer token."),
        (status = 403, description = "Superuser permissions required."),
        (status = 404, description = "Asset not found.")
    ),
    security(("bearer_auth" = [])),
    tag = "ownership-admin"
)]
pub async fn patch_asset(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Path(asset_id): Path<String>,
    Json(body): Json<UpdateAssetDefinitionInput>,
) -> HttpResult<Json<Value>> {
    let state = state.read().await;
    let actor = require_privileged_actor(&headers, &state).await?;
    let asset_id = parse_uuid(&asset_id, "Invalid asset ID")?;
    let asset = catalog::update_asset_definition(&state.db, asset_id, body)
        .await
        .map_err(map_domain_error)?;
    write_audit_log(
        &state.db,
        ACTION_ADMIN_ASSET_UPDATED,
        actor.actor_user_id(),
        None,
        None,
        Some(with_actor_metadata(
            json!({ "assetId": asset.id, "assetKey": asset.key }),
            &actor,
        )),
    )
    .await
    .map_err(|error| HttpError::internal_error(format!("Failed to write audit log: {error}")))?;
    Ok(Json(asset_json(asset)))
}

#[utoipa::path(
    get,
    path = "/api/user/me/inventory",
    responses(
        (status = 200, description = "Current user's aggregated inventory. Expired expirable assets are intentionally omitted and should be treated as non-existing in this view.", body = InventoryResponse),
        (status = 401, description = "Missing bearer token."),
        (status = 403, description = "Token invalid or user inactive.")
    ),
    security(("bearer_auth" = [])),
    tag = "ownership"
)]
pub async fn get_my_inventory(
    State(state): AppStateExtractor,
    headers: HeaderMap,
) -> HttpResult<Json<Value>> {
    let state = state.read().await;
    let user = get_user_from_headers(&headers, &state).await?;
    let inventory = inventory::get_inventory(&state.db, user.id)
        .await
        .map_err(map_domain_error)?;
    Ok(Json(inventory_json(inventory)))
}

#[utoipa::path(
    get,
    path = "/api/user/me/inventory/contains/{asset_key}",
    params(
        ("asset_key" = String, Path, description = "Asset key in lowercase URL-safe format.")
    ),
    responses(
        (status = 200, description = "Check typed presence of a specific asset in current user's inventory. For expired expirable holdings, `exists` is false.", body = InventoryPresenceResponse),
        (status = 400, description = "Invalid asset key."),
        (status = 401, description = "Missing bearer token."),
        (status = 403, description = "Token invalid or user inactive."),
        (status = 404, description = "Asset definition not found.")
    ),
    security(("bearer_auth" = [])),
    tag = "ownership"
)]
pub async fn check_my_inventory_presence(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Path(asset_key): Path<String>,
) -> HttpResult<Json<Value>> {
    let state = state.read().await;
    let user = get_user_from_headers(&headers, &state).await?;
    let presence = inventory::check_presence(&state.db, user.id, &asset_key)
        .await
        .map_err(map_domain_error)?;
    Ok(Json(inventory_presence_json(presence)))
}

#[utoipa::path(
    get,
    path = "/api/user/me/inventory/stackables",
    responses(
        (status = 200, description = "List non-currency stackable holdings for current user.", body = [StackableResponse]),
        (status = 401, description = "Missing bearer token."),
        (status = 403, description = "Token invalid or user inactive.")
    ),
    security(("bearer_auth" = [])),
    tag = "ownership"
)]
pub async fn get_my_stackables(
    State(state): AppStateExtractor,
    headers: HeaderMap,
) -> HttpResult<Json<Value>> {
    let state = state.read().await;
    let user = get_user_from_headers(&headers, &state).await?;
    let items = inventory::get_stackables(&state.db, user.id)
        .await
        .map_err(map_domain_error)?;
    Ok(Json(Value::Array(
        items.into_iter().map(stackable_json).collect(),
    )))
}

#[utoipa::path(
    get,
    path = "/api/user/me/inventory/entitlements",
    responses(
        (status = 200, description = "List current user's entitlement holdings. Presence of a row means ownership.", body = [EntitlementResponse]),
        (status = 401, description = "Missing bearer token."),
        (status = 403, description = "Token invalid or user inactive.")
    ),
    security(("bearer_auth" = [])),
    tag = "ownership"
)]
pub async fn get_my_entitlements(
    State(state): AppStateExtractor,
    headers: HeaderMap,
) -> HttpResult<Json<Value>> {
    let state = state.read().await;
    let user = get_user_from_headers(&headers, &state).await?;
    let items = inventory::get_entitlements(&state.db, user.id)
        .await
        .map_err(map_domain_error)?;
    Ok(Json(Value::Array(
        items.into_iter().map(entitlement_json).collect(),
    )))
}

#[utoipa::path(
    get,
    path = "/api/user/me/inventory/expirables/active",
    responses(
        (status = 200, description = "List only active expirable holdings for current user. Already expired holdings are hidden here.", body = [ExpirableResponse]),
        (status = 401, description = "Missing bearer token."),
        (status = 403, description = "Token invalid or user inactive.")
    ),
    security(("bearer_auth" = [])),
    tag = "ownership"
)]
pub async fn get_my_active_expirables(
    State(state): AppStateExtractor,
    headers: HeaderMap,
) -> HttpResult<Json<Value>> {
    let state = state.read().await;
    let user = get_user_from_headers(&headers, &state).await?;
    let items = inventory::get_active_expirables(&state.db, user.id)
        .await
        .map_err(map_domain_error)?;
    Ok(Json(Value::Array(
        items.into_iter().map(expirable_json).collect(),
    )))
}

#[utoipa::path(
    get,
    path = "/api/user/me/subscription/status",
    responses(
        (status = 200, description = "Get effective subscription tier for current user. Returns only `none`, `plus`, or `pro`. If the underlying state is inconsistent and both subscriptions are active, `pro` wins in the response.", body = SubscriptionStatusResponse),
        (status = 401, description = "Missing bearer token."),
        (status = 403, description = "Token invalid or user inactive.")
    ),
    security(("bearer_auth" = [])),
    tag = "ownership"
)]
pub async fn get_my_subscription_status(
    State(state): AppStateExtractor,
    headers: HeaderMap,
) -> HttpResult<Json<Value>> {
    let state = state.read().await;
    let user = get_user_from_headers(&headers, &state).await?;
    let status = inventory::get_subscription_status(&state.db, user.id)
        .await
        .map_err(map_domain_error)?;
    Ok(Json(subscription_status_json(status)))
}

#[utoipa::path(
    get,
    path = "/api/user/me/wallet",
    responses(
        (status = 200, description = "List all wallet balances for current user. Only currency assets live here.", body = [WalletBalanceResponse]),
        (status = 401, description = "Missing bearer token."),
        (status = 403, description = "Token invalid or user inactive.")
    ),
    security(("bearer_auth" = [])),
    tag = "wallet"
)]
pub async fn get_my_wallet(
    State(state): AppStateExtractor,
    headers: HeaderMap,
) -> HttpResult<Json<Value>> {
    let state = state.read().await;
    let user = get_user_from_headers(&headers, &state).await?;
    let wallet = wallet::get_wallet(&state.db, user.id)
        .await
        .map_err(map_domain_error)?;
    Ok(Json(Value::Array(
        wallet.into_iter().map(wallet_balance_json).collect(),
    )))
}

#[utoipa::path(
    get,
    path = "/api/user/me/wallet/default",
    responses(
        (status = 200, description = "Get current user's balance for the default currency (`coin_default`). Missing balance returns `0` if the system asset exists.", body = WalletBalanceResponse),
        (status = 401, description = "Missing bearer token."),
        (status = 403, description = "Token invalid or user inactive."),
        (status = 404, description = "Default currency asset definition not found.")
    ),
    security(("bearer_auth" = [])),
    tag = "wallet"
)]
pub async fn get_my_default_wallet_balance(
    State(state): AppStateExtractor,
    headers: HeaderMap,
) -> HttpResult<Json<Value>> {
    let state = state.read().await;
    let user = get_user_from_headers(&headers, &state).await?;
    let balance = wallet::get_default_wallet_balance(&state.db, user.id)
        .await
        .map_err(map_domain_error)?;
    Ok(Json(wallet_balance_json(balance)))
}

#[utoipa::path(
    get,
    path = "/api/user/me/wallet/{currency_key}",
    params(
        ("currency_key" = String, Path, description = "Currency asset key.")
    ),
    responses(
        (status = 200, description = "Get one wallet balance for current user. Missing balance returns `0` balance if the currency definition exists.", body = WalletBalanceResponse),
        (status = 400, description = "Invalid asset key or asset is not a currency."),
        (status = 401, description = "Missing bearer token."),
        (status = 403, description = "Token invalid or user inactive."),
        (status = 404, description = "Currency asset definition not found.")
    ),
    security(("bearer_auth" = [])),
    tag = "wallet"
)]
pub async fn get_my_wallet_balance(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Path(currency_key): Path<String>,
) -> HttpResult<Json<Value>> {
    let state = state.read().await;
    let user = get_user_from_headers(&headers, &state).await?;
    let balance = wallet::get_wallet_balance(&state.db, user.id, &currency_key)
        .await
        .map_err(map_domain_error)?;
    Ok(Json(wallet_balance_json(balance)))
}

#[utoipa::path(
    get,
    path = "/api/user/me/wallet/{currency_key}/transactions",
    params(
        ("currency_key" = String, Path, description = "Currency asset key.")
    ),
    responses(
        (status = 200, description = "Get append-only wallet transaction history for a specific currency.", body = [WalletTransactionResponse]),
        (status = 400, description = "Invalid asset key or asset is not a currency."),
        (status = 401, description = "Missing bearer token."),
        (status = 403, description = "Token invalid or user inactive."),
        (status = 404, description = "Currency asset definition not found.")
    ),
    security(("bearer_auth" = [])),
    tag = "wallet"
)]
pub async fn get_my_wallet_transactions(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Path(currency_key): Path<String>,
) -> HttpResult<Json<Value>> {
    let state = state.read().await;
    let user = get_user_from_headers(&headers, &state).await?;
    let txs = wallet::get_wallet_transactions(&state.db, user.id, &currency_key)
        .await
        .map_err(map_domain_error)?;
    Ok(Json(Value::Array(
        txs.into_iter().map(wallet_transaction_json).collect(),
    )))
}

#[utoipa::path(
    get,
    path = "/api/admin/users/{user_id}/inventory",
    params(
        ("user_id" = String, Path, description = "Target user UUID.")
    ),
    responses(
        (status = 200, description = "Admin view of a user's aggregated inventory. Expired expirable holdings are omitted here too.", body = InventoryResponse),
        (status = 400, description = "Invalid user ID."),
        (status = 401, description = "Missing bearer token."),
        (status = 403, description = "Superuser permissions required."),
        (status = 404, description = "User not found.")
    ),
    security(("bearer_auth" = [])),
    tag = "ownership-admin"
)]
pub async fn get_user_inventory(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Path(user_id): Path<String>,
) -> HttpResult<Json<Value>> {
    let state = state.read().await;
    require_privileged_actor(&headers, &state).await?;
    let inventory = inventory::get_inventory(&state.db, parse_uuid(&user_id, "Invalid user ID")?)
        .await
        .map_err(map_domain_error)?;
    Ok(Json(inventory_json(inventory)))
}

pub async fn check_user_inventory_presence(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Path((user_id, asset_key)): Path<(String, String)>,
) -> HttpResult<Json<Value>> {
    let state = state.read().await;
    require_privileged_actor(&headers, &state).await?;
    let presence = inventory::check_presence(
        &state.db,
        parse_uuid(&user_id, "Invalid user ID")?,
        &asset_key,
    )
    .await
    .map_err(map_domain_error)?;
    Ok(Json(inventory_presence_json(presence)))
}

pub async fn get_user_stackables(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Path(user_id): Path<String>,
) -> HttpResult<Json<Value>> {
    let state = state.read().await;
    require_privileged_actor(&headers, &state).await?;
    let items = inventory::get_stackables(&state.db, parse_uuid(&user_id, "Invalid user ID")?)
        .await
        .map_err(map_domain_error)?;
    Ok(Json(Value::Array(
        items.into_iter().map(stackable_json).collect(),
    )))
}

pub async fn get_user_entitlements(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Path(user_id): Path<String>,
) -> HttpResult<Json<Value>> {
    let state = state.read().await;
    require_privileged_actor(&headers, &state).await?;
    let items = inventory::get_entitlements(&state.db, parse_uuid(&user_id, "Invalid user ID")?)
        .await
        .map_err(map_domain_error)?;
    Ok(Json(Value::Array(
        items.into_iter().map(entitlement_json).collect(),
    )))
}

pub async fn get_user_active_expirables(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Path(user_id): Path<String>,
) -> HttpResult<Json<Value>> {
    let state = state.read().await;
    require_privileged_actor(&headers, &state).await?;
    let items =
        inventory::get_active_expirables(&state.db, parse_uuid(&user_id, "Invalid user ID")?)
            .await
            .map_err(map_domain_error)?;
    Ok(Json(Value::Array(
        items.into_iter().map(expirable_json).collect(),
    )))
}

#[utoipa::path(
    get,
    path = "/api/admin/users/{user_id}/inventory/history",
    params(
        ("user_id" = String, Path, description = "Target user UUID.")
    ),
    responses(
        (status = 200, description = "Admin audit view of non-currency inventory history. This is the authoritative journal for grants, removals, expirations, and administrative corrections.", body = [InventoryOperationResponse]),
        (status = 400, description = "Invalid user ID."),
        (status = 401, description = "Missing bearer token."),
        (status = 403, description = "Superuser permissions required."),
        (status = 404, description = "User not found.")
    ),
    security(("bearer_auth" = [])),
    tag = "ownership-admin"
)]
pub async fn get_user_inventory_history(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Path(user_id): Path<String>,
) -> HttpResult<Json<Value>> {
    let state = state.read().await;
    require_privileged_actor(&headers, &state).await?;
    let items =
        inventory::get_inventory_history(&state.db, parse_uuid(&user_id, "Invalid user ID")?)
            .await
            .map_err(map_domain_error)?;
    Ok(Json(Value::Array(
        items.into_iter().map(inventory_operation_json).collect(),
    )))
}

#[utoipa::path(
    post,
    path = "/api/admin/users/{user_id}/inventory/entitlements/{asset_key}/grant",
    params(
        ("user_id" = String, Path, description = "Target user UUID."),
        ("asset_key" = String, Path, description = "Entitlement asset key.")
    ),
    request_body(
        content = MutationBody,
        description = "Grant a binary entitlement. This operation is idempotent: granting the same entitlement again leaves the current state unchanged and does not duplicate ownership rows."
    ),
    responses(
        (status = 200, description = "Entitlement granted or already present.", body = EntitlementResponse),
        (status = 400, description = "Invalid user ID, invalid asset key, non-entitlement asset, inactive asset, or malformed payload."),
        (status = 401, description = "Missing bearer token."),
        (status = 403, description = "Superuser permissions required."),
        (status = 404, description = "User or asset not found.")
    ),
    security(("bearer_auth" = [])),
    tag = "ownership-admin"
)]
pub async fn grant_entitlement(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Path((user_id, asset_key)): Path<(String, String)>,
    Json(body): Json<MutationBody>,
) -> HttpResult<Json<Value>> {
    let state = state.read().await;
    let actor = require_privileged_actor(&headers, &state).await?;
    let ownership_actor = ownership_actor_from_privileged(&actor);
    let parsed_user_id = parse_uuid(&user_id, "Invalid user ID")?;
    let result = inventory::grant_entitlement(
        &state.db,
        EntitlementMutation {
            user_id: parsed_user_id,
            asset_key,
            actor: ownership_actor,
            context: context_from_body(&body),
        },
    )
    .await
    .map_err(map_domain_error)?;
    write_admin_audit(
        &state.db,
        ACTION_ADMIN_INVENTORY_ENTITLEMENT_GRANTED,
        &actor,
        result.asset_key.clone(),
        parsed_user_id,
        result.asset_definition_id,
    )
    .await?;
    Ok(Json(entitlement_json(result)))
}

#[utoipa::path(
    post,
    path = "/api/admin/users/{user_id}/inventory/entitlements/{asset_key}/revoke",
    params(
        ("user_id" = String, Path, description = "Target user UUID."),
        ("asset_key" = String, Path, description = "Entitlement asset key.")
    ),
    request_body(
        content = MutationBody,
        description = "Revoke a binary entitlement. Revocation is idempotent for admin UX: revoking an absent entitlement still succeeds."
    ),
    responses(
        (status = 200, description = "Entitlement revoked or already absent.", body = OkResponse),
        (status = 400, description = "Invalid user ID, invalid asset key, or non-entitlement asset."),
        (status = 401, description = "Missing bearer token."),
        (status = 403, description = "Superuser permissions required."),
        (status = 404, description = "User or asset not found.")
    ),
    security(("bearer_auth" = [])),
    tag = "ownership-admin"
)]
pub async fn revoke_entitlement(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Path((user_id, asset_key)): Path<(String, String)>,
    Json(body): Json<MutationBody>,
) -> HttpResult<Json<Value>> {
    let state = state.read().await;
    let actor = require_privileged_actor(&headers, &state).await?;
    let parsed_user_id = parse_uuid(&user_id, "Invalid user ID")?;
    inventory::revoke_entitlement(
        &state.db,
        EntitlementMutation {
            user_id: parsed_user_id,
            asset_key: asset_key.clone(),
            actor: ownership_actor_from_privileged(&actor),
            context: context_from_body(&body),
        },
    )
    .await
    .map_err(map_domain_error)?;
    write_audit_log(
        &state.db,
        ACTION_ADMIN_INVENTORY_ENTITLEMENT_REVOKED,
        actor.actor_user_id(),
        Some(parsed_user_id),
        body.reason_text.clone(),
        Some(with_actor_metadata(
            json!({ "assetKey": asset_key }),
            &actor,
        )),
    )
    .await
    .map_err(|error| HttpError::internal_error(format!("Failed to write audit log: {error}")))?;
    Ok(Json(json!({ "ok": true })))
}

#[utoipa::path(
    post,
    path = "/api/admin/users/{user_id}/inventory/stackables/{asset_key}/add",
    params(
        ("user_id" = String, Path, description = "Target user UUID."),
        ("asset_key" = String, Path, description = "Non-currency stackable asset key.")
    ),
    request_body(
        content = MutationBody,
        description = "Add quantity to a non-currency stackable holding. `amount` must be positive."
    ),
    responses(
        (status = 200, description = "Stackable amount increased.", body = StackableResponse),
        (status = 400, description = "Invalid input, invalid asset key, non-stackable asset, or currency asset used through inventory endpoint."),
        (status = 401, description = "Missing bearer token."),
        (status = 403, description = "Superuser permissions required."),
        (status = 404, description = "User or asset not found.")
    ),
    security(("bearer_auth" = [])),
    tag = "ownership-admin"
)]
pub async fn add_stackable(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Path((user_id, asset_key)): Path<(String, String)>,
    Json(body): Json<MutationBody>,
) -> HttpResult<Json<Value>> {
    let state = state.read().await;
    let actor = require_privileged_actor(&headers, &state).await?;
    let parsed_user_id = parse_uuid(&user_id, "Invalid user ID")?;
    let result = inventory::add_stackable(
        &state.db,
        StackableMutation {
            user_id: parsed_user_id,
            asset_key,
            amount: body
                .amount
                .ok_or_else(|| HttpError::bad_request("amount is required"))?,
            actor: ownership_actor_from_privileged(&actor),
            context: context_from_body(&body),
        },
    )
    .await
    .map_err(map_domain_error)?;
    write_admin_audit(
        &state.db,
        ACTION_ADMIN_INVENTORY_STACKABLE_ADDED,
        &actor,
        result.asset_key.clone(),
        parsed_user_id,
        result.asset_definition_id,
    )
    .await?;
    Ok(Json(stackable_json(result)))
}

#[utoipa::path(
    post,
    path = "/api/admin/users/{user_id}/inventory/stackables/{asset_key}/remove",
    params(
        ("user_id" = String, Path, description = "Target user UUID."),
        ("asset_key" = String, Path, description = "Non-currency stackable asset key.")
    ),
    request_body(
        content = MutationBody,
        description = "Remove quantity from a non-currency stackable holding. `amount` must be positive and cannot exceed the current amount."
    ),
    responses(
        (status = 200, description = "Stackable amount decreased.", body = StackableResponse),
        (status = 400, description = "Invalid input, invalid asset key, or wrong asset type."),
        (status = 401, description = "Missing bearer token."),
        (status = 403, description = "Superuser permissions required."),
        (status = 404, description = "User or asset not found."),
        (status = 422, description = "Insufficient amount.")
    ),
    security(("bearer_auth" = [])),
    tag = "ownership-admin"
)]
pub async fn remove_stackable(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Path((user_id, asset_key)): Path<(String, String)>,
    Json(body): Json<MutationBody>,
) -> HttpResult<Json<Value>> {
    let state = state.read().await;
    let actor = require_privileged_actor(&headers, &state).await?;
    let parsed_user_id = parse_uuid(&user_id, "Invalid user ID")?;
    let result = inventory::remove_stackable(
        &state.db,
        StackableMutation {
            user_id: parsed_user_id,
            asset_key,
            amount: body
                .amount
                .ok_or_else(|| HttpError::bad_request("amount is required"))?,
            actor: ownership_actor_from_privileged(&actor),
            context: context_from_body(&body),
        },
    )
    .await
    .map_err(map_domain_error)?;
    write_admin_audit(
        &state.db,
        ACTION_ADMIN_INVENTORY_STACKABLE_REMOVED,
        &actor,
        result.asset_key.clone(),
        parsed_user_id,
        result.asset_definition_id,
    )
    .await?;
    Ok(Json(stackable_json(result)))
}

#[utoipa::path(
    put,
    path = "/api/admin/users/{user_id}/inventory/stackables/{asset_key}",
    params(
        ("user_id" = String, Path, description = "Target user UUID."),
        ("asset_key" = String, Path, description = "Non-currency stackable asset key.")
    ),
    request_body(
        content = MutationBody,
        description = "Set exact amount for a non-currency stackable holding. `amount = 0` effectively clears the current state row while preserving operation history."
    ),
    responses(
        (status = 200, description = "Stackable amount set.", body = StackableResponse),
        (status = 400, description = "Invalid input, negative amount, invalid asset key, or wrong asset type."),
        (status = 401, description = "Missing bearer token."),
        (status = 403, description = "Superuser permissions required."),
        (status = 404, description = "User or asset not found.")
    ),
    security(("bearer_auth" = [])),
    tag = "ownership-admin"
)]
pub async fn set_stackable(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Path((user_id, asset_key)): Path<(String, String)>,
    Json(body): Json<MutationBody>,
) -> HttpResult<Json<Value>> {
    let state = state.read().await;
    let actor = require_privileged_actor(&headers, &state).await?;
    let parsed_user_id = parse_uuid(&user_id, "Invalid user ID")?;
    let result = inventory::set_stackable(
        &state.db,
        StackableMutation {
            user_id: parsed_user_id,
            asset_key,
            amount: body
                .amount
                .ok_or_else(|| HttpError::bad_request("amount is required"))?,
            actor: ownership_actor_from_privileged(&actor),
            context: context_from_body(&body),
        },
    )
    .await
    .map_err(map_domain_error)?;
    write_admin_audit(
        &state.db,
        ACTION_ADMIN_INVENTORY_STACKABLE_SET,
        &actor,
        result.asset_key.clone(),
        parsed_user_id,
        result.asset_definition_id,
    )
    .await?;
    Ok(Json(stackable_json(result)))
}

#[utoipa::path(
    post,
    path = "/api/admin/users/{user_id}/inventory/expirables/{asset_key}/prolong",
    params(
        ("user_id" = String, Path, description = "Target user UUID."),
        ("asset_key" = String, Path, description = "Expirable asset key.")
    ),
    request_body(
        content = MutationBody,
        description = "Prolong or issue temporary ownership. If the holding is still active, extension is added on top of current `expiresAt`. If it is already expired, prolong starts from `now`, not from historical expiration."
    ),
    responses(
        (status = 200, description = "Expirable holding prolonged.", body = ExpirableResponse),
        (status = 400, description = "Invalid input, missing `durationSeconds`, invalid asset key, or wrong asset type."),
        (status = 401, description = "Missing bearer token."),
        (status = 403, description = "Superuser permissions required."),
        (status = 404, description = "User or asset not found.")
    ),
    security(("bearer_auth" = [])),
    tag = "ownership-admin"
)]
pub async fn prolong_expirable(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Path((user_id, asset_key)): Path<(String, String)>,
    Json(body): Json<MutationBody>,
) -> HttpResult<Json<Value>> {
    let state = state.read().await;
    let actor = require_privileged_actor(&headers, &state).await?;
    let parsed_user_id = parse_uuid(&user_id, "Invalid user ID")?;
    let result = inventory::prolong_expirable(
        &state.db,
        ProlongExpirableMutation {
            user_id: parsed_user_id,
            asset_key,
            duration_seconds: body
                .duration_seconds
                .ok_or_else(|| HttpError::bad_request("durationSeconds is required"))?,
            actor: ownership_actor_from_privileged(&actor),
            context: context_from_body(&body),
        },
    )
    .await
    .map_err(map_domain_error)?;
    write_admin_audit(
        &state.db,
        ACTION_ADMIN_INVENTORY_EXPIRABLE_PROLONGED,
        &actor,
        result.asset_key.clone(),
        parsed_user_id,
        result.asset_definition_id,
    )
    .await?;
    Ok(Json(expirable_json(result)))
}

#[utoipa::path(
    put,
    path = "/api/admin/users/{user_id}/inventory/expirables/{asset_key}/expiration",
    params(
        ("user_id" = String, Path, description = "Target user UUID."),
        ("asset_key" = String, Path, description = "Expirable asset key.")
    ),
    request_body(
        content = MutationBody,
        description = "Hard-set exact expiration timestamp. This may either extend or shorten access, including setting a past timestamp. Once expired, the holding is omitted from inventory queries."
    ),
    responses(
        (status = 200, description = "Expiration updated.", body = ExpirableResponse),
        (status = 400, description = "Invalid input, missing `expiresAt`, invalid asset key, or wrong asset type."),
        (status = 401, description = "Missing bearer token."),
        (status = 403, description = "Superuser permissions required."),
        (status = 404, description = "User or asset not found.")
    ),
    security(("bearer_auth" = [])),
    tag = "ownership-admin"
)]
pub async fn set_expiration(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Path((user_id, asset_key)): Path<(String, String)>,
    Json(body): Json<MutationBody>,
) -> HttpResult<Json<Value>> {
    let state = state.read().await;
    let actor = require_privileged_actor(&headers, &state).await?;
    let parsed_user_id = parse_uuid(&user_id, "Invalid user ID")?;
    let result = inventory::set_expiration(
        &state.db,
        SetExpirationMutation {
            user_id: parsed_user_id,
            asset_key,
            expires_at: body
                .expires_at
                .ok_or_else(|| HttpError::bad_request("expiresAt is required"))?,
            actor: ownership_actor_from_privileged(&actor),
            context: context_from_body(&body),
        },
    )
    .await
    .map_err(map_domain_error)?;
    write_admin_audit(
        &state.db,
        ACTION_ADMIN_INVENTORY_EXPIRABLE_EXPIRATION_SET,
        &actor,
        result.asset_key.clone(),
        parsed_user_id,
        result.asset_definition_id,
    )
    .await?;
    Ok(Json(expirable_json(result)))
}

#[utoipa::path(
    delete,
    path = "/api/admin/users/{user_id}/inventory/expirables/{asset_key}",
    params(
        ("user_id" = String, Path, description = "Target user UUID."),
        ("asset_key" = String, Path, description = "Expirable asset key.")
    ),
    request_body(
        content = MutationBody,
        description = "Explicitly revoke an expirable holding by removing the current row and recording an operation. This is different from `set expiration`, which may intentionally preserve an expired record."
    ),
    responses(
        (status = 200, description = "Expirable revoked or already absent.", body = OkResponse),
        (status = 400, description = "Invalid input, invalid asset key, or wrong asset type."),
        (status = 401, description = "Missing bearer token."),
        (status = 403, description = "Superuser permissions required."),
        (status = 404, description = "User or asset not found.")
    ),
    security(("bearer_auth" = [])),
    tag = "ownership-admin"
)]
pub async fn revoke_expirable(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Path((user_id, asset_key)): Path<(String, String)>,
    Json(body): Json<MutationBody>,
) -> HttpResult<Json<Value>> {
    let state = state.read().await;
    let actor = require_privileged_actor(&headers, &state).await?;
    let parsed_user_id = parse_uuid(&user_id, "Invalid user ID")?;
    inventory::revoke_expirable(
        &state.db,
        EntitlementMutation {
            user_id: parsed_user_id,
            asset_key: asset_key.clone(),
            actor: ownership_actor_from_privileged(&actor),
            context: context_from_body(&body),
        },
    )
    .await
    .map_err(map_domain_error)?;
    write_audit_log(
        &state.db,
        ACTION_ADMIN_INVENTORY_EXPIRABLE_REVOKED,
        actor.actor_user_id(),
        Some(parsed_user_id),
        body.reason_text.clone(),
        Some(with_actor_metadata(
            json!({ "assetKey": asset_key }),
            &actor,
        )),
    )
    .await
    .map_err(|error| HttpError::internal_error(format!("Failed to write audit log: {error}")))?;
    Ok(Json(json!({ "ok": true })))
}

#[utoipa::path(
    get,
    path = "/api/admin/users/{user_id}/subscription/status",
    params(
        ("user_id" = String, Path, description = "Target user UUID.")
    ),
    responses(
        (status = 200, description = "Get effective subscription tier for a target user. Returns only `none`, `plus`, or `pro`. If raw state contains both active subscriptions, the result is still `pro`.", body = SubscriptionStatusResponse),
        (status = 400, description = "Invalid user ID."),
        (status = 401, description = "Missing bearer token."),
        (status = 403, description = "Superuser permissions required."),
        (status = 404, description = "User not found.")
    ),
    security(("bearer_auth" = [])),
    tag = "ownership-admin"
)]
pub async fn get_user_subscription_status(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Path(user_id): Path<String>,
) -> HttpResult<Json<Value>> {
    let state = state.read().await;
    require_privileged_actor(&headers, &state).await?;
    let status =
        inventory::get_subscription_status(&state.db, parse_uuid(&user_id, "Invalid user ID")?)
            .await
            .map_err(map_domain_error)?;
    Ok(Json(subscription_status_json(status)))
}

#[utoipa::path(
    post,
    path = "/api/admin/users/{user_id}/subscriptions/plus/prolong",
    params(
        ("user_id" = String, Path, description = "Target user UUID.")
    ),
    request_body(
        content = MutationBody,
        description = "Prolong or issue the default Plus subscription. `durationSeconds` must be positive. This convenience endpoint enforces the tier model and refuses to activate Plus while Pro is active."
    ),
    responses(
        (status = 200, description = "Plus subscription prolonged.", body = ExpirableResponse),
        (status = 400, description = "Invalid input, missing `durationSeconds`, or Pro is already active."),
        (status = 401, description = "Missing bearer token."),
        (status = 403, description = "Superuser permissions required."),
        (status = 404, description = "User not found.")
    ),
    security(("bearer_auth" = [])),
    tag = "ownership-admin"
)]
pub async fn prolong_plus_subscription(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Path(user_id): Path<String>,
    Json(body): Json<MutationBody>,
) -> HttpResult<Json<Value>> {
    let state = state.read().await;
    let actor = require_privileged_actor(&headers, &state).await?;
    let parsed_user_id = parse_uuid(&user_id, "Invalid user ID")?;
    let result = inventory::prolong_plus_subscription(
        &state.db,
        SubscriptionMutation {
            user_id: parsed_user_id,
            duration_seconds: body
                .duration_seconds
                .ok_or_else(|| HttpError::bad_request("durationSeconds is required"))?,
            actor: ownership_actor_from_privileged(&actor),
            context: context_from_body(&body),
        },
    )
    .await
    .map_err(map_domain_error)?;
    write_admin_audit(
        &state.db,
        ACTION_ADMIN_INVENTORY_EXPIRABLE_PROLONGED,
        &actor,
        result.asset_key.clone(),
        parsed_user_id,
        result.asset_definition_id,
    )
    .await?;
    Ok(Json(expirable_json(result)))
}

#[utoipa::path(
    post,
    path = "/api/admin/users/{user_id}/subscriptions/pro/prolong",
    params(
        ("user_id" = String, Path, description = "Target user UUID.")
    ),
    request_body(
        content = MutationBody,
        description = "Prolong or issue the default Pro subscription. `durationSeconds` must be positive. Unlike the low-level raw inventory endpoint, this convenience endpoint refuses to activate Pro while Plus is still active."
    ),
    responses(
        (status = 200, description = "Pro subscription prolonged.", body = ExpirableResponse),
        (status = 400, description = "Invalid input, missing `durationSeconds`, or Plus is still active."),
        (status = 401, description = "Missing bearer token."),
        (status = 403, description = "Superuser permissions required."),
        (status = 404, description = "User not found.")
    ),
    security(("bearer_auth" = [])),
    tag = "ownership-admin"
)]
pub async fn prolong_pro_subscription(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Path(user_id): Path<String>,
    Json(body): Json<MutationBody>,
) -> HttpResult<Json<Value>> {
    let state = state.read().await;
    let actor = require_privileged_actor(&headers, &state).await?;
    let parsed_user_id = parse_uuid(&user_id, "Invalid user ID")?;
    let result = inventory::prolong_pro_subscription(
        &state.db,
        SubscriptionMutation {
            user_id: parsed_user_id,
            duration_seconds: body
                .duration_seconds
                .ok_or_else(|| HttpError::bad_request("durationSeconds is required"))?,
            actor: ownership_actor_from_privileged(&actor),
            context: context_from_body(&body),
        },
    )
    .await
    .map_err(map_domain_error)?;
    write_admin_audit(
        &state.db,
        ACTION_ADMIN_INVENTORY_EXPIRABLE_PROLONGED,
        &actor,
        result.asset_key.clone(),
        parsed_user_id,
        result.asset_definition_id,
    )
    .await?;
    Ok(Json(expirable_json(result)))
}

pub async fn get_user_wallet(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Path(user_id): Path<String>,
) -> HttpResult<Json<Value>> {
    let state = state.read().await;
    require_privileged_actor(&headers, &state).await?;
    let items = wallet::get_wallet(&state.db, parse_uuid(&user_id, "Invalid user ID")?)
        .await
        .map_err(map_domain_error)?;
    Ok(Json(Value::Array(
        items.into_iter().map(wallet_balance_json).collect(),
    )))
}

#[utoipa::path(
    get,
    path = "/api/admin/users/{user_id}/wallet/default",
    params(
        ("user_id" = String, Path, description = "Target user UUID.")
    ),
    responses(
        (status = 200, description = "Get a target user's balance for the default currency (`coin_default`). Missing balance returns `0` if the system asset exists.", body = WalletBalanceResponse),
        (status = 400, description = "Invalid user ID."),
        (status = 401, description = "Missing bearer token."),
        (status = 403, description = "Superuser permissions required."),
        (status = 404, description = "User or default currency asset not found.")
    ),
    security(("bearer_auth" = [])),
    tag = "wallet-admin"
)]
pub async fn get_user_default_wallet_balance(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Path(user_id): Path<String>,
) -> HttpResult<Json<Value>> {
    let state = state.read().await;
    require_privileged_actor(&headers, &state).await?;
    let balance =
        wallet::get_default_wallet_balance(&state.db, parse_uuid(&user_id, "Invalid user ID")?)
            .await
            .map_err(map_domain_error)?;
    Ok(Json(wallet_balance_json(balance)))
}

#[utoipa::path(
    post,
    path = "/api/admin/users/{user_id}/wallet/default/credit",
    params(
        ("user_id" = String, Path, description = "Target user UUID.")
    ),
    request_body(
        content = MutationBody,
        description = "Credit the default currency (`coin_default`). This is a convenience alias over the regular wallet credit flow."
    ),
    responses(
        (status = 200, description = "Default currency credited.", body = WalletBalanceResponse),
        (status = 400, description = "Invalid input, missing `amount`, non-positive amount, or invalid user ID."),
        (status = 401, description = "Missing bearer token."),
        (status = 403, description = "Superuser permissions required."),
        (status = 404, description = "User or default currency asset not found.")
    ),
    security(("bearer_auth" = [])),
    tag = "wallet-admin"
)]
pub async fn credit_default_wallet(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Path(user_id): Path<String>,
    Json(body): Json<MutationBody>,
) -> HttpResult<Json<Value>> {
    let state = state.read().await;
    let actor = require_privileged_actor(&headers, &state).await?;
    let parsed_user_id = parse_uuid(&user_id, "Invalid user ID")?;
    let result = wallet::credit_default_wallet(
        &state.db,
        WalletMutation {
            user_id: parsed_user_id,
            currency_key: String::new(),
            amount: body
                .amount
                .ok_or_else(|| HttpError::bad_request("amount is required"))?,
            actor: ownership_actor_from_privileged(&actor),
            context: context_from_body(&body),
        },
    )
    .await
    .map_err(map_domain_error)?;
    write_admin_audit(
        &state.db,
        ACTION_ADMIN_WALLET_CREDITED,
        &actor,
        result.currency_key.clone(),
        parsed_user_id,
        result.currency_asset_definition_id,
    )
    .await?;
    Ok(Json(wallet_balance_json(result)))
}

#[utoipa::path(
    post,
    path = "/api/admin/users/{user_id}/wallet/default/debit",
    params(
        ("user_id" = String, Path, description = "Target user UUID.")
    ),
    request_body(
        content = MutationBody,
        description = "Debit the default currency (`coin_default`). Balance cannot become negative."
    ),
    responses(
        (status = 200, description = "Default currency debited.", body = WalletBalanceResponse),
        (status = 400, description = "Invalid input, missing `amount`, non-positive amount, or invalid user ID."),
        (status = 401, description = "Missing bearer token."),
        (status = 403, description = "Superuser permissions required."),
        (status = 404, description = "User or default currency asset not found."),
        (status = 422, description = "Insufficient wallet balance.")
    ),
    security(("bearer_auth" = [])),
    tag = "wallet-admin"
)]
pub async fn debit_default_wallet(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Path(user_id): Path<String>,
    Json(body): Json<MutationBody>,
) -> HttpResult<Json<Value>> {
    let state = state.read().await;
    let actor = require_privileged_actor(&headers, &state).await?;
    let parsed_user_id = parse_uuid(&user_id, "Invalid user ID")?;
    let result = wallet::debit_default_wallet(
        &state.db,
        WalletMutation {
            user_id: parsed_user_id,
            currency_key: String::new(),
            amount: body
                .amount
                .ok_or_else(|| HttpError::bad_request("amount is required"))?,
            actor: ownership_actor_from_privileged(&actor),
            context: context_from_body(&body),
        },
    )
    .await
    .map_err(map_domain_error)?;
    write_admin_audit(
        &state.db,
        ACTION_ADMIN_WALLET_DEBITED,
        &actor,
        result.currency_key.clone(),
        parsed_user_id,
        result.currency_asset_definition_id,
    )
    .await?;
    Ok(Json(wallet_balance_json(result)))
}

#[utoipa::path(
    put,
    path = "/api/admin/users/{user_id}/wallet/default",
    params(
        ("user_id" = String, Path, description = "Target user UUID.")
    ),
    request_body(
        content = MutationBody,
        description = "Set the exact default currency balance (`coin_default`). `amount` must be non-negative. This is a convenience alias over the regular wallet adjustment flow."
    ),
    responses(
        (status = 200, description = "Default currency balance adjusted.", body = WalletBalanceResponse),
        (status = 400, description = "Invalid input, missing `amount`, negative target balance, or invalid user ID."),
        (status = 401, description = "Missing bearer token."),
        (status = 403, description = "Superuser permissions required."),
        (status = 404, description = "User or default currency asset not found.")
    ),
    security(("bearer_auth" = [])),
    tag = "wallet-admin"
)]
pub async fn adjust_default_wallet_balance(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Path(user_id): Path<String>,
    Json(body): Json<MutationBody>,
) -> HttpResult<Json<Value>> {
    let state = state.read().await;
    let actor = require_privileged_actor(&headers, &state).await?;
    let parsed_user_id = parse_uuid(&user_id, "Invalid user ID")?;
    let result = wallet::adjust_default_wallet_balance(
        &state.db,
        WalletMutation {
            user_id: parsed_user_id,
            currency_key: String::new(),
            amount: body
                .amount
                .ok_or_else(|| HttpError::bad_request("amount is required"))?,
            actor: ownership_actor_from_privileged(&actor),
            context: context_from_body(&body),
        },
    )
    .await
    .map_err(map_domain_error)?;
    write_admin_audit(
        &state.db,
        ACTION_ADMIN_WALLET_ADJUSTED,
        &actor,
        result.currency_key.clone(),
        parsed_user_id,
        result.currency_asset_definition_id,
    )
    .await?;
    Ok(Json(wallet_balance_json(result)))
}

pub async fn get_user_wallet_balance(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Path((user_id, currency_key)): Path<(String, String)>,
) -> HttpResult<Json<Value>> {
    let state = state.read().await;
    require_privileged_actor(&headers, &state).await?;
    let balance = wallet::get_wallet_balance(
        &state.db,
        parse_uuid(&user_id, "Invalid user ID")?,
        &currency_key,
    )
    .await
    .map_err(map_domain_error)?;
    Ok(Json(wallet_balance_json(balance)))
}

#[utoipa::path(
    get,
    path = "/api/admin/users/{user_id}/wallet/{currency_key}/transactions",
    params(
        ("user_id" = String, Path, description = "Target user UUID."),
        ("currency_key" = String, Path, description = "Currency asset key.")
    ),
    responses(
        (status = 200, description = "Admin view of append-only wallet ledger for one currency.", body = [WalletTransactionResponse]),
        (status = 400, description = "Invalid user ID, invalid asset key, or asset is not a currency."),
        (status = 401, description = "Missing bearer token."),
        (status = 403, description = "Superuser permissions required."),
        (status = 404, description = "User or asset not found.")
    ),
    security(("bearer_auth" = [])),
    tag = "wallet-admin"
)]
pub async fn get_user_wallet_transactions(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Path((user_id, currency_key)): Path<(String, String)>,
) -> HttpResult<Json<Value>> {
    let state = state.read().await;
    require_privileged_actor(&headers, &state).await?;
    let items = wallet::get_wallet_transactions(
        &state.db,
        parse_uuid(&user_id, "Invalid user ID")?,
        &currency_key,
    )
    .await
    .map_err(map_domain_error)?;
    Ok(Json(Value::Array(
        items.into_iter().map(wallet_transaction_json).collect(),
    )))
}

#[utoipa::path(
    post,
    path = "/api/admin/users/{user_id}/wallet/{currency_key}/credit",
    params(
        ("user_id" = String, Path, description = "Target user UUID."),
        ("currency_key" = String, Path, description = "Currency asset key.")
    ),
    request_body(
        content = MutationBody,
        description = "Credit currency to wallet. This always appends a wallet transaction and updates current balance atomically."
    ),
    responses(
        (status = 200, description = "Currency credited.", body = WalletBalanceResponse),
        (status = 400, description = "Invalid input, invalid asset key, or asset is not a currency."),
        (status = 401, description = "Missing bearer token."),
        (status = 403, description = "Superuser permissions required."),
        (status = 404, description = "User or asset not found.")
    ),
    security(("bearer_auth" = [])),
    tag = "wallet-admin"
)]
pub async fn credit_wallet(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Path((user_id, currency_key)): Path<(String, String)>,
    Json(body): Json<MutationBody>,
) -> HttpResult<Json<Value>> {
    let state = state.read().await;
    let actor = require_privileged_actor(&headers, &state).await?;
    let parsed_user_id = parse_uuid(&user_id, "Invalid user ID")?;
    let result = wallet::credit(
        &state.db,
        WalletMutation {
            user_id: parsed_user_id,
            currency_key,
            amount: body
                .amount
                .ok_or_else(|| HttpError::bad_request("amount is required"))?,
            actor: ownership_actor_from_privileged(&actor),
            context: context_from_body(&body),
        },
    )
    .await
    .map_err(map_domain_error)?;
    write_admin_audit(
        &state.db,
        ACTION_ADMIN_WALLET_CREDITED,
        &actor,
        result.currency_key.clone(),
        parsed_user_id,
        result.currency_asset_definition_id,
    )
    .await?;
    Ok(Json(wallet_balance_json(result)))
}

#[utoipa::path(
    post,
    path = "/api/admin/users/{user_id}/wallet/{currency_key}/debit",
    params(
        ("user_id" = String, Path, description = "Target user UUID."),
        ("currency_key" = String, Path, description = "Currency asset key.")
    ),
    request_body(
        content = MutationBody,
        description = "Debit currency from wallet. Balance cannot become negative; insufficient funds return 422."
    ),
    responses(
        (status = 200, description = "Currency debited.", body = WalletBalanceResponse),
        (status = 400, description = "Invalid input, invalid asset key, or asset is not a currency."),
        (status = 401, description = "Missing bearer token."),
        (status = 403, description = "Superuser permissions required."),
        (status = 404, description = "User or asset not found."),
        (status = 422, description = "Insufficient wallet balance.")
    ),
    security(("bearer_auth" = [])),
    tag = "wallet-admin"
)]
pub async fn debit_wallet(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Path((user_id, currency_key)): Path<(String, String)>,
    Json(body): Json<MutationBody>,
) -> HttpResult<Json<Value>> {
    let state = state.read().await;
    let actor = require_privileged_actor(&headers, &state).await?;
    let parsed_user_id = parse_uuid(&user_id, "Invalid user ID")?;
    let result = wallet::debit(
        &state.db,
        WalletMutation {
            user_id: parsed_user_id,
            currency_key,
            amount: body
                .amount
                .ok_or_else(|| HttpError::bad_request("amount is required"))?,
            actor: ownership_actor_from_privileged(&actor),
            context: context_from_body(&body),
        },
    )
    .await
    .map_err(map_domain_error)?;
    write_admin_audit(
        &state.db,
        ACTION_ADMIN_WALLET_DEBITED,
        &actor,
        result.currency_key.clone(),
        parsed_user_id,
        result.currency_asset_definition_id,
    )
    .await?;
    Ok(Json(wallet_balance_json(result)))
}

#[utoipa::path(
    put,
    path = "/api/admin/users/{user_id}/wallet/{currency_key}",
    params(
        ("user_id" = String, Path, description = "Target user UUID."),
        ("currency_key" = String, Path, description = "Currency asset key.")
    ),
    request_body(
        content = MutationBody,
        description = "Set exact wallet balance through administrative adjustment. `amount` must be non-negative. The ledger stores this as a single `adjustment` transaction with delta computed as `target - current`."
    ),
    responses(
        (status = 200, description = "Balance adjusted.", body = WalletBalanceResponse),
        (status = 400, description = "Invalid input, invalid asset key, negative target balance, or asset is not a currency."),
        (status = 401, description = "Missing bearer token."),
        (status = 403, description = "Superuser permissions required."),
        (status = 404, description = "User or asset not found.")
    ),
    security(("bearer_auth" = [])),
    tag = "wallet-admin"
)]
pub async fn adjust_wallet_balance(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Path((user_id, currency_key)): Path<(String, String)>,
    Json(body): Json<MutationBody>,
) -> HttpResult<Json<Value>> {
    let state = state.read().await;
    let actor = require_privileged_actor(&headers, &state).await?;
    let parsed_user_id = parse_uuid(&user_id, "Invalid user ID")?;
    let result = wallet::adjust_balance(
        &state.db,
        WalletMutation {
            user_id: parsed_user_id,
            currency_key,
            amount: body
                .amount
                .ok_or_else(|| HttpError::bad_request("amount is required"))?,
            actor: ownership_actor_from_privileged(&actor),
            context: context_from_body(&body),
        },
    )
    .await
    .map_err(map_domain_error)?;
    write_admin_audit(
        &state.db,
        ACTION_ADMIN_WALLET_ADJUSTED,
        &actor,
        result.currency_key.clone(),
        parsed_user_id,
        result.currency_asset_definition_id,
    )
    .await?;
    Ok(Json(wallet_balance_json(result)))
}

async fn write_admin_audit(
    db: &sea_orm::DatabaseConnection,
    action: &str,
    actor: &PrivilegedActor,
    asset_key: String,
    target_user_id: Uuid,
    asset_definition_id: Uuid,
) -> HttpResult<()> {
    write_audit_log(
        db,
        action,
        actor.actor_user_id(),
        Some(target_user_id),
        None,
        Some(with_actor_metadata(
            json!({ "assetKey": asset_key, "assetDefinitionId": asset_definition_id }),
            actor,
        )),
    )
    .await
    .map_err(|error| HttpError::internal_error(format!("Failed to write audit log: {error}")))?;
    Ok(())
}

fn ownership_actor_from_privileged(actor: &PrivilegedActor) -> OwnershipActor {
    match actor {
        PrivilegedActor::User(user) => OwnershipActor::admin(user.id),
        PrivilegedActor::Service(service) => OwnershipActor::system(service.system_name.clone()),
    }
}

fn with_actor_metadata(metadata: Value, actor: &PrivilegedActor) -> Value {
    let mut metadata = match metadata {
        Value::Object(map) => map,
        other => {
            let mut map = serde_json::Map::new();
            map.insert("value".to_string(), other);
            map
        }
    };

    if let Some(service_name) = actor.actor_service_name() {
        metadata.insert(
            "actorServiceName".to_string(),
            Value::String(service_name.to_string()),
        );
    }

    Value::Object(metadata)
}

fn parse_uuid(value: &str, message: &str) -> HttpResult<Uuid> {
    Uuid::parse_str(value).map_err(|_| HttpError::bad_request(message))
}

fn context_from_body(body: &MutationBody) -> OperationContext {
    OperationContext {
        reason_code: body.reason_code.clone(),
        reason_text: body.reason_text.clone(),
        metadata: body.metadata.clone().unwrap_or_else(|| json!({})),
    }
}

fn map_asset_query(query: AssetListQuery) -> AssetDefinitionQuery {
    AssetDefinitionQuery {
        q: query.q,
        asset_kind: query.asset_kind,
        ownership_model: query.ownership_model,
        is_currency: query.is_currency,
        is_public: query.is_public,
        is_user_purchasable: query.is_user_purchasable,
        is_active: query.is_active,
        page: query.page,
        per_page: query.per_page,
    }
}

fn map_domain_error(error: anyhow::Error) -> HttpError {
    let message = error.to_string();
    if message.contains("not found") || message.contains("Not found") {
        HttpError::not_found(message)
    } else if message.contains("Insufficient") {
        HttpError::new(axum::http::StatusCode::UNPROCESSABLE_ENTITY, message)
    } else {
        HttpError::bad_request(message)
    }
}

fn asset_json(asset: catalog::AssetDefinitionView) -> Value {
    json!({
        "id": asset.id,
        "key": asset.key,
        "displayName": asset.display_name,
        "description": asset.description,
        "assetKind": asset.asset_kind,
        "ownershipModel": asset.ownership_model,
        "isCurrency": asset.is_currency,
        "isUserPurchasable": asset.is_user_purchasable,
        "isPublic": asset.is_public,
        "isActive": asset.is_active,
        "metadata": asset.metadata,
        "createdAt": asset.created_at,
        "updatedAt": asset.updated_at
    })
}

fn asset_list_json(result: catalog::AssetDefinitionListResult) -> Value {
    json!({
        "items": result.items.into_iter().map(asset_json).collect::<Vec<_>>(),
        "total": result.total,
        "page": result.page,
        "perPage": result.per_page,
        "totalPages": result.total_pages
    })
}

fn inventory_json(inventory: inventory::InventoryView) -> Value {
    json!({
        "userId": inventory.user_id,
        "stackables": inventory.stackables.into_iter().map(stackable_json).collect::<Vec<_>>(),
        "entitlements": inventory.entitlements.into_iter().map(entitlement_json).collect::<Vec<_>>(),
        "expirables": inventory.expirables.into_iter().map(expirable_json).collect::<Vec<_>>()
    })
}

fn inventory_presence_json(presence: inventory::InventoryPresenceView) -> Value {
    json!({
        "assetKey": presence.asset_key,
        "ownershipModel": presence.ownership_model,
        "exists": presence.exists,
        "amount": presence.amount,
        "isActive": presence.is_active,
        "expiresAt": presence.expires_at
    })
}

fn stackable_json(item: inventory::StackableView) -> Value {
    json!({
        "assetKey": item.asset_key,
        "assetDefinitionId": item.asset_definition_id,
        "amount": item.amount,
        "updatedAt": item.updated_at
    })
}

fn entitlement_json(item: inventory::EntitlementView) -> Value {
    json!({
        "assetKey": item.asset_key,
        "assetDefinitionId": item.asset_definition_id,
        "grantedAt": item.granted_at,
        "updatedAt": item.updated_at
    })
}

fn expirable_json(item: inventory::ExpirableView) -> Value {
    json!({
        "assetKey": item.asset_key,
        "assetDefinitionId": item.asset_definition_id,
        "expiresAt": item.expires_at,
        "grantedAt": item.granted_at,
        "lastExtendedAt": item.last_extended_at,
        "updatedAt": item.updated_at,
        "isActive": item.is_active
    })
}

fn inventory_operation_json(item: inventory::InventoryOperationView) -> Value {
    json!({
        "id": item.id,
        "assetDefinitionId": item.asset_definition_id,
        "assetKey": item.asset_key,
        "ownershipModel": item.ownership_model,
        "operationType": item.operation_type,
        "actorKind": item.actor_kind,
        "actorUserId": item.actor_user_id,
        "actorServiceName": item.actor_service_name,
        "deltaAmount": item.delta_amount,
        "newAmount": item.new_amount,
        "previousExpiresAt": item.previous_expires_at,
        "newExpiresAt": item.new_expires_at,
        "reasonCode": item.reason_code,
        "reasonText": item.reason_text,
        "metadata": item.metadata,
        "createdAt": item.created_at
    })
}

fn wallet_balance_json(item: wallet::WalletBalanceView) -> Value {
    json!({
        "userId": item.user_id,
        "currencyAssetDefinitionId": item.currency_asset_definition_id,
        "currencyKey": item.currency_key,
        "balance": item.balance,
        "updatedAt": item.updated_at
    })
}

fn wallet_transaction_json(item: wallet::WalletTransactionView) -> Value {
    json!({
        "id": item.id,
        "currencyAssetDefinitionId": item.currency_asset_definition_id,
        "currencyKey": item.currency_key,
        "operationType": item.operation_type,
        "actorKind": item.actor_kind,
        "actorUserId": item.actor_user_id,
        "actorServiceName": item.actor_service_name,
        "delta": item.delta,
        "balanceAfter": item.balance_after,
        "reasonCode": item.reason_code,
        "reasonText": item.reason_text,
        "metadata": item.metadata,
        "createdAt": item.created_at
    })
}

fn subscription_status_json(item: inventory::SubscriptionStatusView) -> Value {
    json!({
        "userId": item.user_id,
        "status": match item.status {
            SubscriptionStatus::None => "none",
            SubscriptionStatus::Plus => "plus",
            SubscriptionStatus::Pro => "pro",
        }
    })
}
