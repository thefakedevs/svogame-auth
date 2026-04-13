use axum::Json;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::app::auth::{PrivilegedActor, get_user_from_headers, require_privileged_actor};
use crate::app::http::{HttpError, HttpResult};
use crate::app::state::AppStateExtractor;
use crate::services::gunskins::{
    GunskinAssetView, GunskinCollectionView, GunskinSelectionListItemView, SelectGunskinMutation,
    SelectedGunskinView,
};
use crate::services::ownership::types::{OperationContext, OwnershipActor, SkinRarity};

#[derive(Debug, Deserialize, ToSchema)]
pub struct SelectGunskinRequest {
    #[serde(rename = "assetKey")]
    pub asset_key: String,
    #[serde(rename = "reasonCode")]
    pub reason_code: Option<String>,
    #[serde(rename = "reasonText")]
    pub reason_text: Option<String>,
    pub metadata: Option<Value>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct GunskinAssetResponse {
    pub id: String,
    pub key: String,
    #[serde(rename = "displayName")]
    pub display_name: String,
    pub description: Option<String>,
    #[serde(rename = "weaponKey")]
    pub weapon_key: String,
    pub rarity: SkinRarity,
    #[serde(rename = "ownershipModel")]
    pub ownership_model: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SelectedGunskinResponse {
    #[serde(rename = "weaponKey")]
    pub weapon_key: String,
    #[serde(rename = "assetDefinitionId")]
    pub asset_definition_id: String,
    #[serde(rename = "assetKey")]
    pub asset_key: String,
    #[serde(rename = "displayName")]
    pub display_name: String,
    pub description: Option<String>,
    pub rarity: SkinRarity,
    #[serde(rename = "selectedAt")]
    pub selected_at: chrono::DateTime<chrono::Utc>,
    #[serde(rename = "updatedAt")]
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct GunskinCollectionResponse {
    #[serde(rename = "weaponKey")]
    pub weapon_key: String,
    pub selected: Option<SelectedGunskinResponse>,
    pub available: Vec<GunskinAssetResponse>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct GunskinSelectionListItemResponse {
    #[serde(rename = "weaponKey")]
    pub weapon_key: String,
    pub selected: SelectedGunskinResponse,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct WeaponKeyListResponse {
    pub items: Vec<String>,
}

#[utoipa::path(
    get,
    path = "/api/user/me/gunskins/selections",
    responses(
        (status = 200, description = "List current authenticated user's selected gunskins.", body = [GunskinSelectionListItemResponse]),
        (status = 401, description = "Missing bearer token."),
        (status = 403, description = "Forbidden.")
    ),
    security(("bearer_auth" = [])),
    tag = "gunskins"
)]
pub async fn list_my_gunskin_selections(
    State(state): AppStateExtractor,
    headers: HeaderMap,
) -> HttpResult<Json<Vec<GunskinSelectionListItemResponse>>> {
    let state = state.read().await;
    let user = get_user_from_headers(&headers, &state).await?;
    let items = crate::services::gunskins::list_user_selections(&state.db, user.id)
        .await
        .map_err(map_domain_error)?;
    Ok(Json(items.into_iter().map(selection_item_response).collect()))
}

#[utoipa::path(
    get,
    path = "/api/user/me/gunskins/{weapon_key}",
    params(
        ("weapon_key" = String, Path, description = "Weapon key, for example `tacz:akm`.")
    ),
    responses(
        (status = 200, description = "Current user's selected and available gunskins for one weapon.", body = GunskinCollectionResponse),
        (status = 401, description = "Missing bearer token."),
        (status = 403, description = "Forbidden."),
        (status = 400, description = "Invalid weapon key.")
    ),
    security(("bearer_auth" = [])),
    tag = "gunskins"
)]
pub async fn get_my_gunskin_collection(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Path(weapon_key): Path<String>,
) -> HttpResult<Json<GunskinCollectionResponse>> {
    let state = state.read().await;
    let user = get_user_from_headers(&headers, &state).await?;
    let collection = crate::services::gunskins::get_user_gunskin_collection(
        &state.db,
        user.id,
        &weapon_key,
    )
    .await
    .map_err(map_domain_error)?;
    Ok(Json(collection_response(collection)))
}

#[utoipa::path(
    put,
    path = "/api/user/me/gunskins/{weapon_key}/selected",
    params(
        ("weapon_key" = String, Path, description = "Weapon key, for example `tacz:akm`.")
    ),
    request_body = SelectGunskinRequest,
    responses(
        (status = 200, description = "Gunskin selected for the current user.", body = SelectedGunskinResponse),
        (status = 401, description = "Missing bearer token."),
        (status = 403, description = "Forbidden."),
        (status = 404, description = "Gunskin or user not found."),
        (status = 422, description = "User does not own the gunskin or it is not applicable to the weapon.")
    ),
    security(("bearer_auth" = [])),
    tag = "gunskins"
)]
pub async fn select_my_gunskin(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Path(weapon_key): Path<String>,
    Json(body): Json<SelectGunskinRequest>,
) -> HttpResult<Json<SelectedGunskinResponse>> {
    let state = state.read().await;
    let user = get_user_from_headers(&headers, &state).await?;
    let context = context_from_body(&body);
    let selected = crate::services::gunskins::select_gunskin(
        &state.db,
        SelectGunskinMutation {
            user_id: user.id,
            weapon_key,
            asset_key: body.asset_key,
            actor: OwnershipActor::user(user.id),
            context,
        },
    )
    .await
    .map_err(map_domain_error)?;
    Ok(Json(selected_response(selected)))
}

#[utoipa::path(
    delete,
    path = "/api/user/me/gunskins/{weapon_key}/selected",
    params(
        ("weapon_key" = String, Path, description = "Weapon key, for example `tacz:akm`.")
    ),
    responses(
        (status = 200, description = "Gunskin selection reset to default.", body = crate::domains::ownership::handlers::OkResponse),
        (status = 401, description = "Missing bearer token."),
        (status = 403, description = "Forbidden.")
    ),
    security(("bearer_auth" = [])),
    tag = "gunskins"
)]
pub async fn reset_my_gunskin(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Path(weapon_key): Path<String>,
) -> HttpResult<Json<Value>> {
    let state = state.read().await;
    let user = get_user_from_headers(&headers, &state).await?;
    crate::services::gunskins::reset_gunskin_selection(&state.db, user.id, &weapon_key)
        .await
        .map_err(map_domain_error)?;
    Ok(Json(json!({ "ok": true })))
}

#[utoipa::path(
    get,
    path = "/api/system/users/{user_id}/gunskins/{weapon_key}/selected",
    params(
        ("user_id" = String, Path, description = "Target user UUID."),
        ("weapon_key" = String, Path, description = "Weapon key, for example `tacz:akm`.")
    ),
    responses(
        (status = 200, description = "Selected gunskin for a user and weapon.", body = Option<SelectedGunskinResponse>),
        (status = 401, description = "Missing bearer token."),
        (status = 403, description = "Superuser or service token required."),
        (status = 404, description = "User not found.")
    ),
    security(("bearer_auth" = [])),
    tag = "gunskins-system"
)]
pub async fn get_user_selected_gunskin(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Path((user_id, weapon_key)): Path<(String, String)>,
) -> HttpResult<Json<Option<SelectedGunskinResponse>>> {
    let state = state.read().await;
    require_privileged_actor(&headers, &state).await?;
    let user_id = parse_uuid(&user_id)?;
    let selected = crate::services::gunskins::get_selected_gunskin(&state.db, user_id, &weapon_key)
        .await
        .map_err(map_domain_error)?;
    Ok(Json(selected.map(selected_response)))
}

#[utoipa::path(
    get,
    path = "/api/system/users/{user_id}/gunskins/{weapon_key}/available",
    params(
        ("user_id" = String, Path, description = "Target user UUID."),
        ("weapon_key" = String, Path, description = "Weapon key, for example `tacz:akm`.")
    ),
    responses(
        (status = 200, description = "Available gunskins for a user and weapon.", body = [GunskinAssetResponse]),
        (status = 401, description = "Missing bearer token."),
        (status = 403, description = "Superuser or service token required."),
        (status = 404, description = "User not found.")
    ),
    security(("bearer_auth" = [])),
    tag = "gunskins-system"
)]
pub async fn list_user_available_gunskins(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Path((user_id, weapon_key)): Path<(String, String)>,
) -> HttpResult<Json<Vec<GunskinAssetResponse>>> {
    let state = state.read().await;
    require_privileged_actor(&headers, &state).await?;
    let user_id = parse_uuid(&user_id)?;
    let items = crate::services::gunskins::list_available_gunskins_for_user(
        &state.db,
        user_id,
        &weapon_key,
    )
    .await
    .map_err(map_domain_error)?;
    Ok(Json(items.into_iter().map(asset_response).collect()))
}

#[utoipa::path(
    put,
    path = "/api/system/users/{user_id}/gunskins/{weapon_key}/selected",
    params(
        ("user_id" = String, Path, description = "Target user UUID."),
        ("weapon_key" = String, Path, description = "Weapon key, for example `tacz:akm`.")
    ),
    request_body = SelectGunskinRequest,
    responses(
        (status = 200, description = "Gunskin selected for the user.", body = SelectedGunskinResponse),
        (status = 401, description = "Missing bearer token."),
        (status = 403, description = "Superuser or service token required."),
        (status = 404, description = "Gunskin or user not found."),
        (status = 422, description = "User does not own the gunskin or it is not applicable to the weapon.")
    ),
    security(("bearer_auth" = [])),
    tag = "gunskins-system"
)]
pub async fn select_user_gunskin(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Path((user_id, weapon_key)): Path<(String, String)>,
    Json(body): Json<SelectGunskinRequest>,
) -> HttpResult<Json<SelectedGunskinResponse>> {
    let state = state.read().await;
    let actor = require_privileged_actor(&headers, &state).await?;
    let user_id = parse_uuid(&user_id)?;
    let context = context_from_body(&body);
    let selected = crate::services::gunskins::select_gunskin(
        &state.db,
        SelectGunskinMutation {
            user_id,
            weapon_key,
            asset_key: body.asset_key,
            actor: ownership_actor_from_privileged(&actor),
            context,
        },
    )
    .await
    .map_err(map_domain_error)?;
    Ok(Json(selected_response(selected)))
}

#[utoipa::path(
    delete,
    path = "/api/system/users/{user_id}/gunskins/{weapon_key}/selected",
    params(
        ("user_id" = String, Path, description = "Target user UUID."),
        ("weapon_key" = String, Path, description = "Weapon key, for example `tacz:akm`.")
    ),
    responses(
        (status = 200, description = "Gunskin selection reset to default.", body = crate::domains::ownership::handlers::OkResponse),
        (status = 401, description = "Missing bearer token."),
        (status = 403, description = "Superuser or service token required.")
    ),
    security(("bearer_auth" = [])),
    tag = "gunskins-system"
)]
pub async fn reset_user_gunskin(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Path((user_id, weapon_key)): Path<(String, String)>,
) -> HttpResult<Json<Value>> {
    let state = state.read().await;
    require_privileged_actor(&headers, &state).await?;
    let user_id = parse_uuid(&user_id)?;
    crate::services::gunskins::reset_gunskin_selection(&state.db, user_id, &weapon_key)
        .await
        .map_err(map_domain_error)?;
    Ok(Json(json!({ "ok": true })))
}

#[utoipa::path(
    get,
    path = "/api/admin/gunskins/keys",
    responses(
        (status = 200, description = "List all weapon keys currently referenced by active gunskins.", body = WeaponKeyListResponse),
        (status = 401, description = "Missing bearer token."),
        (status = 403, description = "Superuser or service token required.")
    ),
    security(("bearer_auth" = [])),
    tag = "gunskins-admin"
)]
pub async fn list_admin_weapon_keys(
    State(state): AppStateExtractor,
    headers: HeaderMap,
) -> HttpResult<Json<WeaponKeyListResponse>> {
    let state = state.read().await;
    require_privileged_actor(&headers, &state).await?;
    let items = crate::services::gunskins::list_weapon_keys(&state.db)
        .await
        .map_err(map_domain_error)?;
    Ok(Json(WeaponKeyListResponse { items }))
}

fn parse_uuid(value: &str) -> HttpResult<Uuid> {
    Uuid::parse_str(value).map_err(|_| HttpError::bad_request("Invalid user ID"))
}

fn context_from_body(body: &SelectGunskinRequest) -> OperationContext {
    OperationContext {
        reason_code: body.reason_code.clone(),
        reason_text: body.reason_text.clone(),
        metadata: body.metadata.clone().unwrap_or_else(|| json!({})),
    }
}

fn ownership_actor_from_privileged(actor: &PrivilegedActor) -> OwnershipActor {
    match actor {
        PrivilegedActor::User(user) => OwnershipActor::admin(user.id),
        PrivilegedActor::Service(service) => OwnershipActor::system(service.system_name.clone()),
    }
}

fn map_domain_error(error: anyhow::Error) -> HttpError {
    let message = error.to_string();
    if message.contains("not found") || message.contains("Not found") {
        HttpError::not_found(message)
    } else if message.contains("does not own")
        || message.contains("not applicable")
        || message.contains("missing rarity")
        || message.contains("missing weapon key")
    {
        HttpError::new(StatusCode::UNPROCESSABLE_ENTITY, message)
    } else {
        HttpError::bad_request(message)
    }
}

fn asset_response(view: GunskinAssetView) -> GunskinAssetResponse {
    GunskinAssetResponse {
        id: view.id.to_string(),
        key: view.key,
        display_name: view.display_name,
        description: view.description,
        weapon_key: view.weapon_key,
        rarity: view.rarity,
        ownership_model: view.ownership_model.as_str().to_string(),
    }
}

fn selected_response(view: SelectedGunskinView) -> SelectedGunskinResponse {
    SelectedGunskinResponse {
        weapon_key: view.weapon_key,
        asset_definition_id: view.asset_definition_id.to_string(),
        asset_key: view.asset_key,
        display_name: view.display_name,
        description: view.description,
        rarity: view.rarity,
        selected_at: view.selected_at,
        updated_at: view.updated_at,
    }
}

fn collection_response(view: GunskinCollectionView) -> GunskinCollectionResponse {
    GunskinCollectionResponse {
        weapon_key: view.weapon_key,
        selected: view.selected.map(selected_response),
        available: view.available.into_iter().map(asset_response).collect(),
    }
}

fn selection_item_response(view: GunskinSelectionListItemView) -> GunskinSelectionListItemResponse {
    GunskinSelectionListItemResponse {
        weapon_key: view.weapon_key,
        selected: selected_response(view.selected),
    }
}
