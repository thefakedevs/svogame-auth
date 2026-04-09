use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::app::auth::{
    PrivilegedActor, get_user_from_headers, require_human_superuser, require_privileged_actor,
};
use crate::app::http::{HttpError, HttpResult};
use crate::app::state::AppStateExtractor;
use crate::services::audit::{
    ACTION_ADMIN_LOOTBOX_CREATED, ACTION_ADMIN_LOOTBOX_DROP_CREATED,
    ACTION_ADMIN_LOOTBOX_DROP_DELETED, ACTION_ADMIN_LOOTBOX_DROP_UPDATED,
    ACTION_ADMIN_LOOTBOX_OPENED_FOR_USER, ACTION_ADMIN_LOOTBOX_UPDATED, ACTION_USER_LOOTBOX_OPENED,
    write_audit_log,
};
use crate::services::lootboxes::{
    self, CreateLootboxDefinitionInput, CreateLootboxDropInput, LootboxDefinitionView,
    LootboxDetailView, LootboxDropView, LootboxOpenHistoryView, LootboxOpenResultView,
    OpenFeedEntryView, OpenRewardView, OwnedLootboxView, UpdateLootboxDefinitionInput,
    UpdateLootboxDropInput,
};
use crate::services::ownership::types::OwnershipActor;

#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct OpenLootboxQuery {
    #[serde(rename = "feedLength")]
    pub feed_length: Option<usize>,
    pub locale: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct LootboxDefinitionResponse {
    pub id: String,
    #[schema(rename = "assetDefinitionId")]
    pub asset_definition_id: String,
    #[schema(rename = "assetKey")]
    pub asset_key: String,
    #[schema(rename = "assetDisplayName")]
    pub asset_display_name: String,
    #[schema(rename = "assetDescription")]
    pub asset_description: Option<String>,
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

#[derive(Debug, Serialize, ToSchema)]
pub struct LootboxDropResponse {
    pub id: String,
    #[schema(rename = "rewardAssetDefinitionId")]
    pub reward_asset_definition_id: String,
    #[schema(rename = "rewardAssetKey")]
    pub reward_asset_key: String,
    #[schema(rename = "rewardAssetDisplayName")]
    pub reward_asset_display_name: String,
    #[schema(rename = "rewardOwnershipModel")]
    pub reward_ownership_model: String,
    pub amount: Option<i64>,
    #[schema(rename = "durationSeconds")]
    pub duration_seconds: Option<i64>,
    pub weight: i64,
    #[schema(rename = "totalWeight")]
    pub total_weight: i64,
    #[schema(rename = "titleI18n")]
    pub title_i18n: Value,
    #[schema(rename = "isActive")]
    pub is_active: bool,
    #[schema(rename = "sortOrder")]
    pub sort_order: i32,
    #[schema(rename = "createdAt")]
    pub created_at: chrono::DateTime<chrono::Utc>,
    #[schema(rename = "updatedAt")]
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct LootboxDetailResponse {
    pub definition: LootboxDefinitionResponse,
    pub drops: Vec<LootboxDropResponse>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct OwnedLootboxResponse {
    #[schema(rename = "lootboxId")]
    pub lootbox_id: String,
    #[schema(rename = "assetKey")]
    pub asset_key: String,
    #[schema(rename = "displayName")]
    pub display_name: String,
    pub amount: i64,
    #[schema(rename = "isOpenable")]
    pub is_openable: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct LootboxFeedEntryResponse {
    #[schema(rename = "assetKey")]
    pub asset_key: String,
    #[schema(rename = "displayName")]
    pub display_name: String,
    pub title: String,
    #[schema(rename = "ownershipModel")]
    pub ownership_model: String,
    pub amount: Option<i64>,
    #[schema(rename = "durationSeconds")]
    pub duration_seconds: Option<i64>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct LootboxRewardResponse {
    #[schema(rename = "assetKey")]
    pub asset_key: String,
    #[schema(rename = "displayName")]
    pub display_name: String,
    pub title: String,
    #[schema(rename = "ownershipModel")]
    pub ownership_model: String,
    pub amount: Option<i64>,
    #[schema(rename = "durationSeconds")]
    pub duration_seconds: Option<i64>,
    #[schema(rename = "expiresAt")]
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct LootboxOpenResultResponse {
    #[schema(rename = "operationId")]
    pub operation_id: i64,
    #[schema(rename = "lootboxAssetKey")]
    pub lootbox_asset_key: String,
    #[schema(rename = "openedAt")]
    pub opened_at: chrono::DateTime<chrono::Utc>,
    pub reward: LootboxRewardResponse,
    pub feed: Vec<LootboxFeedEntryResponse>,
    #[schema(rename = "winnerIndex")]
    pub winner_index: usize,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct LootboxOpenHistoryResponse {
    pub id: i64,
    #[schema(rename = "userId")]
    pub user_id: String,
    #[schema(rename = "lootboxAssetKey")]
    pub lootbox_asset_key: String,
    pub reward: LootboxRewardResponse,
    #[schema(rename = "actorKind")]
    pub actor_kind: String,
    #[schema(rename = "actorUserId")]
    pub actor_user_id: Option<String>,
    #[schema(rename = "actorServiceName")]
    pub actor_service_name: Option<String>,
    #[schema(rename = "feedLength")]
    pub feed_length: usize,
    #[schema(rename = "winnerIndex")]
    pub winner_index: usize,
    #[schema(rename = "openedAt")]
    pub opened_at: chrono::DateTime<chrono::Utc>,
}

#[utoipa::path(
    get,
    path = "/api/lootboxes",
    responses(
        (status = 200, description = "List publicly visible, active lootbox definitions. This is a catalog view only and does not depend on ownership.", body = [LootboxDefinitionResponse])
    ),
    tag = "lootboxes"
)]
pub async fn list_public_lootboxes(State(state): AppStateExtractor) -> HttpResult<Json<Value>> {
    let state = state.read().await;
    let items = lootboxes::list_lootboxes(&state.db, true)
        .await
        .map_err(map_domain_error)?;
    Ok(Json(Value::Array(
        items.into_iter().map(lootbox_definition_json).collect(),
    )))
}

#[utoipa::path(
    get,
    path = "/api/lootboxes/{asset_key}",
    params(
        ("asset_key" = String, Path, description = "Lootbox asset key in lowercase URL-safe format.")
    ),
    responses(
        (status = 200, description = "Get one public lootbox definition together with its configured drop entries. Amount and duration are exact constants per drop entry.", body = LootboxDetailResponse),
        (status = 400, description = "Invalid asset key."),
        (status = 404, description = "Lootbox not found, inactive, or not public.")
    ),
    tag = "lootboxes"
)]
pub async fn get_public_lootbox(
    State(state): AppStateExtractor,
    Path(asset_key): Path<String>,
) -> HttpResult<Json<Value>> {
    let state = state.read().await;
    let lootbox = lootboxes::get_public_lootbox_by_asset_key(&state.db, &asset_key)
        .await
        .map_err(map_domain_error)?
        .ok_or_else(|| HttpError::not_found("Lootbox not found"))?;
    Ok(Json(lootbox_detail_json(lootbox)))
}

#[utoipa::path(
    get,
    path = "/api/user/me/lootboxes",
    responses(
        (status = 200, description = "List lootbox assets currently owned by the authenticated user. Only holdings with amount > 0 are returned.", body = [OwnedLootboxResponse]),
        (status = 401, description = "Missing bearer token."),
        (status = 403, description = "Token invalid or user inactive.")
    ),
    security(("bearer_auth" = [])),
    tag = "lootboxes"
)]
pub async fn get_my_lootboxes(
    State(state): AppStateExtractor,
    headers: HeaderMap,
) -> HttpResult<Json<Value>> {
    let state = state.read().await;
    let user = get_user_from_headers(&headers, &state).await?;
    let items = lootboxes::list_owned_lootboxes(&state.db, user.id)
        .await
        .map_err(map_domain_error)?;
    Ok(Json(Value::Array(
        items.into_iter().map(owned_lootbox_json).collect(),
    )))
}

#[utoipa::path(
    get,
    path = "/api/user/me/lootboxes/open-history",
    responses(
        (status = 200, description = "Current user's lootbox opening history. The final reward and generated roulette metadata are recorded per open operation.", body = [LootboxOpenHistoryResponse]),
        (status = 401, description = "Missing bearer token."),
        (status = 403, description = "Token invalid or user inactive.")
    ),
    security(("bearer_auth" = [])),
    tag = "lootboxes"
)]
pub async fn get_my_lootbox_open_history(
    State(state): AppStateExtractor,
    headers: HeaderMap,
) -> HttpResult<Json<Value>> {
    let state = state.read().await;
    let user = get_user_from_headers(&headers, &state).await?;
    let items = lootboxes::get_open_history(&state.db, user.id)
        .await
        .map_err(map_domain_error)?;
    Ok(Json(Value::Array(
        items.into_iter().map(open_history_json).collect(),
    )))
}

#[utoipa::path(
    post,
    path = "/api/user/me/lootboxes/{asset_key}/open",
    params(
        ("asset_key" = String, Path, description = "Lootbox asset key in lowercase URL-safe format."),
        OpenLootboxQuery
    ),
    responses(
        (status = 200, description = "Open one lootbox from the current user's inventory. The server first selects the real reward by weights, then separately generates the roulette feed.", body = LootboxOpenResultResponse),
        (status = 400, description = "Invalid asset key, invalid feedLength, or misconfigured lootbox."),
        (status = 401, description = "Missing bearer token."),
        (status = 403, description = "Token invalid or user inactive."),
        (status = 404, description = "Lootbox definition not found."),
        (status = 422, description = "User does not own this lootbox or does not have enough quantity to consume one.")
    ),
    security(("bearer_auth" = [])),
    tag = "lootboxes"
)]
pub async fn open_my_lootbox(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Path(asset_key): Path<String>,
    Query(query): Query<OpenLootboxQuery>,
) -> HttpResult<Json<Value>> {
    let state = state.read().await;
    let user = get_user_from_headers(&headers, &state).await?;
    let result = lootboxes::open_lootbox(
        &state.db,
        user.id,
        &asset_key,
        query.feed_length,
        query.locale,
        OwnershipActor::user(user.id),
    )
    .await
    .map_err(map_domain_error)?;
    write_audit_log(
        &state.db,
        ACTION_USER_LOOTBOX_OPENED,
        Some(user.id),
        Some(user.id),
        None,
        Some(json!({
            "lootboxAssetKey": result.lootbox_asset_key,
            "operationId": result.operation_id,
            "rewardAssetKey": result.reward.asset_key,
        })),
    )
    .await
    .map_err(|error| HttpError::internal_error(format!("Failed to write audit log: {error}")))?;
    Ok(Json(open_result_json(result)))
}

#[utoipa::path(
    get,
    path = "/api/admin/lootboxes",
    responses(
        (status = 200, description = "Administrative list of all lootbox definitions, including inactive ones.", body = [LootboxDefinitionResponse]),
        (status = 401, description = "Missing bearer token."),
        (status = 403, description = "Human superuser required.")
    ),
    security(("bearer_auth" = [])),
    tag = "lootboxes-admin"
)]
pub async fn list_admin_lootboxes(
    State(state): AppStateExtractor,
    headers: HeaderMap,
) -> HttpResult<Json<Value>> {
    let state = state.read().await;
    require_human_superuser(&headers, &state).await?;
    let items = lootboxes::list_lootboxes(&state.db, false)
        .await
        .map_err(map_domain_error)?;
    Ok(Json(Value::Array(
        items.into_iter().map(lootbox_definition_json).collect(),
    )))
}

#[utoipa::path(
    post,
    path = "/api/admin/lootboxes",
    request_body(
        content = CreateLootboxDefinitionInput,
        description = "Create a lootbox definition for an existing asset. The asset must already be active, non-currency, have `asset_kind = lootbox`, and use `stackable` ownership."
    ),
    responses(
        (status = 200, description = "Lootbox definition created.", body = LootboxDetailResponse),
        (status = 400, description = "Invalid asset key, invalid target asset, or a definition already exists for that asset."),
        (status = 401, description = "Missing bearer token."),
        (status = 403, description = "Human superuser required.")
    ),
    security(("bearer_auth" = [])),
    tag = "lootboxes-admin"
)]
pub async fn create_lootbox(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Json(body): Json<CreateLootboxDefinitionInput>,
) -> HttpResult<Json<Value>> {
    let state = state.read().await;
    let actor = require_human_superuser(&headers, &state).await?;
    let lootbox = lootboxes::create_lootbox_definition(&state.db, body)
        .await
        .map_err(map_domain_error)?;
    write_audit_log(
        &state.db,
        ACTION_ADMIN_LOOTBOX_CREATED,
        Some(actor.id),
        None,
        None,
        Some(json!({
            "lootboxId": lootbox.definition.id,
            "assetKey": lootbox.definition.asset_key,
        })),
    )
    .await
    .map_err(|error| HttpError::internal_error(format!("Failed to write audit log: {error}")))?;
    Ok(Json(lootbox_detail_json(lootbox)))
}

#[utoipa::path(
    get,
    path = "/api/admin/lootboxes/{lootbox_id}",
    params(
        ("lootbox_id" = String, Path, description = "Lootbox definition UUID.")
    ),
    responses(
        (status = 200, description = "Admin-only detailed lootbox definition view.", body = LootboxDetailResponse),
        (status = 400, description = "Invalid lootbox ID."),
        (status = 401, description = "Missing bearer token."),
        (status = 403, description = "Human superuser required."),
        (status = 404, description = "Lootbox not found.")
    ),
    security(("bearer_auth" = [])),
    tag = "lootboxes-admin"
)]
pub async fn get_admin_lootbox(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Path(lootbox_id): Path<String>,
) -> HttpResult<Json<Value>> {
    let state = state.read().await;
    require_human_superuser(&headers, &state).await?;
    let lootbox_id = parse_uuid(&lootbox_id, "Invalid lootbox ID")?;
    let lootbox = lootboxes::get_lootbox_by_id(&state.db, lootbox_id)
        .await
        .map_err(map_domain_error)?
        .ok_or_else(|| HttpError::not_found("Lootbox not found"))?;
    Ok(Json(lootbox_detail_json(lootbox)))
}

#[utoipa::path(
    patch,
    path = "/api/admin/lootboxes/{lootbox_id}",
    params(
        ("lootbox_id" = String, Path, description = "Lootbox definition UUID.")
    ),
    request_body(
        content = UpdateLootboxDefinitionInput,
        description = "Patch mutable lootbox configuration fields such as active flag and metadata."
    ),
    responses(
        (status = 200, description = "Lootbox definition updated.", body = LootboxDetailResponse),
        (status = 400, description = "Invalid lootbox ID or invalid patch payload."),
        (status = 401, description = "Missing bearer token."),
        (status = 403, description = "Human superuser required."),
        (status = 404, description = "Lootbox not found.")
    ),
    security(("bearer_auth" = [])),
    tag = "lootboxes-admin"
)]
pub async fn patch_lootbox(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Path(lootbox_id): Path<String>,
    Json(body): Json<UpdateLootboxDefinitionInput>,
) -> HttpResult<Json<Value>> {
    let state = state.read().await;
    let actor = require_human_superuser(&headers, &state).await?;
    let lootbox_id = parse_uuid(&lootbox_id, "Invalid lootbox ID")?;
    let lootbox = lootboxes::update_lootbox_definition(&state.db, lootbox_id, body)
        .await
        .map_err(map_domain_error)?;
    write_audit_log(
        &state.db,
        ACTION_ADMIN_LOOTBOX_UPDATED,
        Some(actor.id),
        None,
        None,
        Some(json!({
            "lootboxId": lootbox.definition.id,
            "assetKey": lootbox.definition.asset_key,
        })),
    )
    .await
    .map_err(|error| HttpError::internal_error(format!("Failed to write audit log: {error}")))?;
    Ok(Json(lootbox_detail_json(lootbox)))
}

#[utoipa::path(
    post,
    path = "/api/admin/lootboxes/{lootbox_id}/drops",
    params(
        ("lootbox_id" = String, Path, description = "Lootbox definition UUID.")
    ),
    request_body(
        content = CreateLootboxDropInput,
        description = "Add one concrete drop entry. Different quantities or durations must be modeled as separate drop entries with their own weights."
    ),
    responses(
        (status = 200, description = "Drop entry created.", body = LootboxDetailResponse),
        (status = 400, description = "Invalid lootbox ID, invalid reward asset, invalid amount or duration, or non-positive weight."),
        (status = 401, description = "Missing bearer token."),
        (status = 403, description = "Human superuser required."),
        (status = 404, description = "Lootbox not found.")
    ),
    security(("bearer_auth" = [])),
    tag = "lootboxes-admin"
)]
pub async fn create_lootbox_drop(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Path(lootbox_id): Path<String>,
    Json(body): Json<CreateLootboxDropInput>,
) -> HttpResult<Json<Value>> {
    let state = state.read().await;
    let actor = require_human_superuser(&headers, &state).await?;
    let lootbox_id = parse_uuid(&lootbox_id, "Invalid lootbox ID")?;
    let lootbox = lootboxes::create_lootbox_drop(&state.db, lootbox_id, body)
        .await
        .map_err(map_domain_error)?;
    write_audit_log(
        &state.db,
        ACTION_ADMIN_LOOTBOX_DROP_CREATED,
        Some(actor.id),
        None,
        None,
        Some(json!({
            "lootboxId": lootbox.definition.id,
            "assetKey": lootbox.definition.asset_key,
        })),
    )
    .await
    .map_err(|error| HttpError::internal_error(format!("Failed to write audit log: {error}")))?;
    Ok(Json(lootbox_detail_json(lootbox)))
}

#[utoipa::path(
    patch,
    path = "/api/admin/lootboxes/{lootbox_id}/drops/{drop_id}",
    params(
        ("lootbox_id" = String, Path, description = "Lootbox definition UUID."),
        ("drop_id" = String, Path, description = "Drop definition UUID.")
    ),
    request_body(
        content = UpdateLootboxDropInput,
        description = "Patch one drop entry. Reward asset identity is immutable; create a new drop entry if a different reward asset is needed."
    ),
    responses(
        (status = 200, description = "Drop entry updated.", body = LootboxDetailResponse),
        (status = 400, description = "Invalid identifiers or invalid updated payload."),
        (status = 401, description = "Missing bearer token."),
        (status = 403, description = "Human superuser required."),
        (status = 404, description = "Lootbox or drop not found.")
    ),
    security(("bearer_auth" = [])),
    tag = "lootboxes-admin"
)]
pub async fn patch_lootbox_drop(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Path((lootbox_id, drop_id)): Path<(String, String)>,
    Json(body): Json<UpdateLootboxDropInput>,
) -> HttpResult<Json<Value>> {
    let state = state.read().await;
    let actor = require_human_superuser(&headers, &state).await?;
    let lootbox_id = parse_uuid(&lootbox_id, "Invalid lootbox ID")?;
    let drop_id = parse_uuid(&drop_id, "Invalid drop ID")?;
    let lootbox = lootboxes::update_lootbox_drop(&state.db, lootbox_id, drop_id, body)
        .await
        .map_err(map_domain_error)?;
    write_audit_log(
        &state.db,
        ACTION_ADMIN_LOOTBOX_DROP_UPDATED,
        Some(actor.id),
        None,
        None,
        Some(json!({
            "lootboxId": lootbox.definition.id,
            "dropId": drop_id,
            "assetKey": lootbox.definition.asset_key,
        })),
    )
    .await
    .map_err(|error| HttpError::internal_error(format!("Failed to write audit log: {error}")))?;
    Ok(Json(lootbox_detail_json(lootbox)))
}

#[utoipa::path(
    delete,
    path = "/api/admin/lootboxes/{lootbox_id}/drops/{drop_id}",
    params(
        ("lootbox_id" = String, Path, description = "Lootbox definition UUID."),
        ("drop_id" = String, Path, description = "Drop definition UUID.")
    ),
    responses(
        (status = 200, description = "Drop entry deleted.", body = Value),
        (status = 400, description = "Invalid identifiers."),
        (status = 401, description = "Missing bearer token."),
        (status = 403, description = "Human superuser required."),
        (status = 404, description = "Lootbox or drop not found.")
    ),
    security(("bearer_auth" = [])),
    tag = "lootboxes-admin"
)]
pub async fn delete_lootbox_drop(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Path((lootbox_id, drop_id)): Path<(String, String)>,
) -> HttpResult<Json<Value>> {
    let state = state.read().await;
    let actor = require_human_superuser(&headers, &state).await?;
    let lootbox_id = parse_uuid(&lootbox_id, "Invalid lootbox ID")?;
    let drop_id = parse_uuid(&drop_id, "Invalid drop ID")?;
    lootboxes::delete_lootbox_drop(&state.db, lootbox_id, drop_id)
        .await
        .map_err(map_domain_error)?;
    write_audit_log(
        &state.db,
        ACTION_ADMIN_LOOTBOX_DROP_DELETED,
        Some(actor.id),
        None,
        None,
        Some(json!({
            "lootboxId": lootbox_id,
            "dropId": drop_id,
        })),
    )
    .await
    .map_err(|error| HttpError::internal_error(format!("Failed to write audit log: {error}")))?;
    Ok(Json(json!({ "ok": true })))
}

#[utoipa::path(
    post,
    path = "/api/admin/users/{user_id}/lootboxes/{asset_key}/open",
    params(
        ("user_id" = String, Path, description = "Target user UUID."),
        ("asset_key" = String, Path, description = "Lootbox asset key in lowercase URL-safe format."),
        OpenLootboxQuery
    ),
    responses(
        (status = 200, description = "Open one lootbox on behalf of the target player. This endpoint is intentionally available to both human superusers and authenticated service tokens.", body = LootboxOpenResultResponse),
        (status = 400, description = "Invalid user ID, invalid asset key, invalid feedLength, or misconfigured lootbox."),
        (status = 401, description = "Missing bearer token."),
        (status = 403, description = "Human superuser or service token required."),
        (status = 404, description = "User or lootbox not found."),
        (status = 422, description = "Target user does not own this lootbox or does not have enough quantity to consume one.")
    ),
    security(("bearer_auth" = [])),
    tag = "lootboxes-admin"
)]
pub async fn open_user_lootbox(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Path((user_id, asset_key)): Path<(String, String)>,
    Query(query): Query<OpenLootboxQuery>,
) -> HttpResult<Json<Value>> {
    let state = state.read().await;
    let actor = require_privileged_actor(&headers, &state).await?;
    let target_user_id = parse_uuid(&user_id, "Invalid user ID")?;
    let result = lootboxes::open_lootbox(
        &state.db,
        target_user_id,
        &asset_key,
        query.feed_length,
        query.locale,
        ownership_actor_from_privileged(&actor),
    )
    .await
    .map_err(map_domain_error)?;
    write_audit_log(
        &state.db,
        ACTION_ADMIN_LOOTBOX_OPENED_FOR_USER,
        actor.actor_user_id(),
        Some(target_user_id),
        None,
        Some(with_actor_metadata(
            json!({
                "lootboxAssetKey": result.lootbox_asset_key,
                "operationId": result.operation_id,
                "rewardAssetKey": result.reward.asset_key,
            }),
            &actor,
        )),
    )
    .await
    .map_err(|error| HttpError::internal_error(format!("Failed to write audit log: {error}")))?;
    Ok(Json(open_result_json(result)))
}

#[utoipa::path(
    get,
    path = "/api/admin/users/{user_id}/lootboxes/open-history",
    params(
        ("user_id" = String, Path, description = "Target user UUID.")
    ),
    responses(
        (status = 200, description = "Administrative view of one player's lootbox opening history.", body = [LootboxOpenHistoryResponse]),
        (status = 400, description = "Invalid user ID."),
        (status = 401, description = "Missing bearer token."),
        (status = 403, description = "Human superuser required."),
        (status = 404, description = "User not found.")
    ),
    security(("bearer_auth" = [])),
    tag = "lootboxes-admin"
)]
pub async fn get_user_lootbox_open_history(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Path(user_id): Path<String>,
) -> HttpResult<Json<Value>> {
    let state = state.read().await;
    require_human_superuser(&headers, &state).await?;
    let user_id = parse_uuid(&user_id, "Invalid user ID")?;
    let items = lootboxes::get_open_history(&state.db, user_id)
        .await
        .map_err(map_domain_error)?;
    Ok(Json(Value::Array(
        items.into_iter().map(open_history_json).collect(),
    )))
}

#[utoipa::path(
    get,
    path = "/api/admin/lootboxes/open-history",
    responses(
        (status = 200, description = "Global administrative view of lootbox opening history across all users.", body = [LootboxOpenHistoryResponse]),
        (status = 401, description = "Missing bearer token."),
        (status = 403, description = "Human superuser required.")
    ),
    security(("bearer_auth" = [])),
    tag = "lootboxes-admin"
)]
pub async fn get_all_lootbox_open_history(
    State(state): AppStateExtractor,
    headers: HeaderMap,
) -> HttpResult<Json<Value>> {
    let state = state.read().await;
    require_human_superuser(&headers, &state).await?;
    let items = lootboxes::get_all_open_history(&state.db)
        .await
        .map_err(map_domain_error)?;
    Ok(Json(Value::Array(
        items.into_iter().map(open_history_json).collect(),
    )))
}

fn lootbox_definition_json(item: LootboxDefinitionView) -> Value {
    json!({
        "id": item.id,
        "assetDefinitionId": item.asset_definition_id,
        "assetKey": item.asset_key,
        "assetDisplayName": item.asset_display_name,
        "assetDescription": item.asset_description,
        "isPublic": item.is_public,
        "isActive": item.is_active,
        "metadata": item.metadata,
        "createdAt": item.created_at,
        "updatedAt": item.updated_at
    })
}

fn lootbox_drop_json(item: LootboxDropView) -> Value {
    json!({
        "id": item.id,
        "rewardAssetDefinitionId": item.reward_asset_definition_id,
        "rewardAssetKey": item.reward_asset_key,
        "rewardAssetDisplayName": item.reward_asset_display_name,
        "rewardOwnershipModel": item.reward_ownership_model.as_str(),
        "amount": item.stackable_amount,
        "durationSeconds": item.expirable_duration_seconds,
        "weight": item.weight,
        "totalWeight": item.total_weight,
        "titleI18n": item.title_i18n,
        "isActive": item.is_active,
        "sortOrder": item.sort_order,
        "createdAt": item.created_at,
        "updatedAt": item.updated_at
    })
}

fn lootbox_detail_json(item: LootboxDetailView) -> Value {
    json!({
        "definition": lootbox_definition_json(item.definition),
        "drops": item.drops.into_iter().map(lootbox_drop_json).collect::<Vec<_>>()
    })
}

fn owned_lootbox_json(item: OwnedLootboxView) -> Value {
    json!({
        "lootboxId": item.lootbox_id,
        "assetKey": item.asset_key,
        "displayName": item.display_name,
        "amount": item.amount,
        "isOpenable": item.is_openable
    })
}

fn feed_entry_json(item: OpenFeedEntryView) -> Value {
    json!({
        "assetKey": item.asset_key,
        "displayName": item.display_name,
        "title": item.title,
        "ownershipModel": item.ownership_model,
        "amount": item.amount,
        "durationSeconds": item.duration_seconds
    })
}

fn reward_json(item: OpenRewardView) -> Value {
    json!({
        "assetKey": item.asset_key,
        "displayName": item.display_name,
        "title": item.title,
        "ownershipModel": item.ownership_model,
        "amount": item.amount,
        "durationSeconds": item.duration_seconds,
        "expiresAt": item.expires_at
    })
}

fn open_result_json(item: LootboxOpenResultView) -> Value {
    json!({
        "operationId": item.operation_id,
        "lootboxAssetKey": item.lootbox_asset_key,
        "openedAt": item.opened_at,
        "reward": reward_json(item.reward),
        "feed": item.feed.into_iter().map(feed_entry_json).collect::<Vec<_>>(),
        "winnerIndex": item.winner_index
    })
}

fn open_history_json(item: LootboxOpenHistoryView) -> Value {
    json!({
        "id": item.id,
        "userId": item.user_id,
        "lootboxAssetKey": item.lootbox_asset_key,
        "reward": reward_json(item.reward),
        "actorKind": item.actor_kind,
        "actorUserId": item.actor_user_id,
        "actorServiceName": item.actor_service_name,
        "feedLength": item.feed_length,
        "winnerIndex": item.winner_index,
        "openedAt": item.opened_at
    })
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

fn map_domain_error(error: anyhow::Error) -> HttpError {
    let message = error.to_string();
    if message.contains("not found") || message.contains("Not found") {
        HttpError::not_found(message)
    } else if message.contains("Insufficient") {
        HttpError::new(StatusCode::UNPROCESSABLE_ENTITY, message)
    } else {
        HttpError::bad_request(message)
    }
}
