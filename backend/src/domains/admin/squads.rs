use axum::body::Bytes;
use axum::extract::{Multipart, Path, Query, State};
use axum::Json;
use aws_sdk_s3::primitives::ByteStream;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, Set, TransactionTrait,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::app::auth::require_superuser;
use crate::app::http::{HttpError, HttpResult};
use crate::app::state::{AppState, AppStateExtractor};
use crate::entities::{
    Squad, SquadActiveModel, SquadColumn, SquadInvite, SquadInviteColumn, User, UserActiveModel,
    UserColumn,
};
use crate::services::audit::{
    write_audit_log, ACTION_ADMIN_SQUAD_DELETED, ACTION_ADMIN_SQUAD_IMAGE_DELETED,
    ACTION_ADMIN_SQUAD_IMAGE_UPDATED, ACTION_ADMIN_SQUAD_MEMBER_KICKED,
    ACTION_ADMIN_SQUAD_RESTRICTED, ACTION_ADMIN_SQUAD_UNRESTRICTED, ACTION_ADMIN_SQUAD_UPDATED,
};
use crate::services::squads::{process_squad_image, squad_image_key, validate_squad_name, SQUAD_MAX_MEMBERS};

#[derive(Deserialize, IntoParams, ToSchema)]
pub struct ListSquadsQuery {
    pub q: Option<String>,
    pub page: Option<u64>,
    #[serde(rename = "perPage")]
    pub per_page: Option<u64>,
}

#[derive(Deserialize, ToSchema)]
pub struct PatchAdminSquadRequest {
    pub name: Option<String>,
    #[serde(rename = "isRestricted")]
    pub is_restricted: Option<bool>,
    #[serde(rename = "restrictionReason")]
    pub restriction_reason: Option<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct ReasonRequest {
    pub reason: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct AdminSquadResponse {
    pub id: String,
    pub name: String,
    #[serde(rename = "leaderUserId")]
    pub leader_user_id: String,
    #[serde(rename = "memberCount")]
    pub member_count: u64,
    #[serde(rename = "maxMembers")]
    pub max_members: u64,
    #[serde(rename = "imageUrl")]
    pub image_url: Option<String>,
    #[serde(rename = "isRestricted")]
    pub is_restricted: bool,
    #[serde(rename = "restrictionReason")]
    pub restriction_reason: Option<String>,
    #[serde(rename = "createdAt")]
    pub created_at: chrono::DateTime<chrono::Utc>,
    #[serde(rename = "updatedAt")]
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Serialize, ToSchema)]
pub struct AdminSquadListResponse {
    pub items: Vec<AdminSquadResponse>,
    pub total: u64,
    pub page: u64,
    #[serde(rename = "perPage")]
    pub per_page: u64,
}

#[derive(Serialize, ToSchema)]
pub struct ActionResponse {
    pub status: &'static str,
}

#[utoipa::path(
    get,
    path = "/api/admin/squads",
    params(ListSquadsQuery),
    responses(
        (status = 200, description = "Admin list of squads with optional search by squad name, squad id, or leader id.", body = AdminSquadListResponse),
        (status = 401, description = "Missing bearer token."),
        (status = 403, description = "Superuser permissions required.")
    ),
    security(("bearer_auth" = [])),
    tag = "admin"
)]
pub async fn list_squads(
    State(state): AppStateExtractor,
    headers: axum::http::HeaderMap,
    Query(query): Query<ListSquadsQuery>,
) -> HttpResult<Json<AdminSquadListResponse>> {
    let state = state.read().await;
    require_superuser(&headers, &state).await?;

    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(20).clamp(1, 100);
    let mut squad_query = Squad::find().order_by_desc(SquadColumn::CreatedAt);

    if let Some(q) = query.q.as_ref().map(|it| it.trim()).filter(|it| !it.is_empty()) {
        let mut condition = Condition::any().add(SquadColumn::Name.contains(q));
        if let Ok(uuid) = Uuid::parse_str(q) {
            condition = condition
                .add(SquadColumn::Id.eq(uuid))
                .add(SquadColumn::LeaderUserId.eq(uuid));
        }
        squad_query = squad_query.filter(condition);
    }

    let paginator = squad_query.paginate(&state.db, per_page);
    let total = paginator
        .num_items()
        .await
        .map_err(|e| HttpError::internal_error(format!("Failed to count squads: {e}")))?;
    let squads = paginator
        .fetch_page(page.saturating_sub(1))
        .await
        .map_err(|e| HttpError::internal_error(format!("Failed to fetch squads: {e}")))?;

    let mut items = Vec::with_capacity(squads.len());
    for squad in squads {
        items.push(to_squad_response(&state, squad).await?);
    }

    Ok(Json(AdminSquadListResponse {
        items,
        total,
        page,
        per_page,
    }))
}

#[utoipa::path(
    get,
    path = "/api/admin/squads/{squad_id}",
    params(
        ("squad_id" = String, Path, description = "Squad UUID.")
    ),
    responses(
        (status = 200, description = "Admin squad details.", body = AdminSquadResponse),
        (status = 400, description = "Invalid squad ID."),
        (status = 401, description = "Missing bearer token."),
        (status = 403, description = "Superuser permissions required."),
        (status = 404, description = "Squad not found.")
    ),
    security(("bearer_auth" = [])),
    tag = "admin"
)]
pub async fn get_squad(
    State(state): AppStateExtractor,
    headers: axum::http::HeaderMap,
    Path(squad_id): Path<String>,
) -> HttpResult<Json<AdminSquadResponse>> {
    let state = state.read().await;
    require_superuser(&headers, &state).await?;
    let squad = get_squad_model(&state, &squad_id).await?;
    Ok(Json(to_squad_response(&state, squad).await?))
}

#[utoipa::path(
    patch,
    path = "/api/admin/squads/{squad_id}",
    params(
        ("squad_id" = String, Path, description = "Squad UUID.")
    ),
    request_body = PatchAdminSquadRequest,
    responses(
        (status = 200, description = "Admin-updated squad fields.", body = AdminSquadResponse),
        (status = 400, description = "Invalid squad ID or invalid squad patch values."),
        (status = 401, description = "Missing bearer token."),
        (status = 403, description = "Superuser permissions required."),
        (status = 404, description = "Squad not found.")
    ),
    security(("bearer_auth" = [])),
    tag = "admin"
)]
pub async fn patch_squad(
    State(state): AppStateExtractor,
    headers: axum::http::HeaderMap,
    Path(squad_id): Path<String>,
    Json(body): Json<PatchAdminSquadRequest>,
) -> HttpResult<Json<AdminSquadResponse>> {
    let state = state.read().await;
    let admin = require_superuser(&headers, &state).await?;
    let squad = get_squad_model(&state, &squad_id).await?;
    let mut active_squad: SquadActiveModel = squad.into();

    if let Some(name) = body.name {
        active_squad.name = Set(validate_squad_name(&name).map_err(|e| HttpError::bad_request(e.to_string()))?);
    }
    if let Some(is_restricted) = body.is_restricted {
        active_squad.is_restricted = Set(is_restricted);
    }
    if body.restriction_reason.is_some() {
        active_squad.restriction_reason = Set(body.restriction_reason);
    }
    active_squad.updated_at = Set(chrono::Utc::now());

    let updated_squad = active_squad
        .update(&state.db)
        .await
        .map_err(|e| HttpError::internal_error(format!("Failed to update squad: {e}")))?;

    write_audit_log(
        &state.db,
        ACTION_ADMIN_SQUAD_UPDATED,
        Some(admin.id),
        Some(updated_squad.leader_user_id),
        None,
        Some(json!({ "squadId": updated_squad.id })),
    )
    .await
    .map_err(|e| HttpError::internal_error(format!("Failed to write audit log: {e}")))?;

    Ok(Json(to_squad_response(&state, updated_squad).await?))
}

#[utoipa::path(
    post,
    path = "/api/admin/squads/{squad_id}/restrict",
    params(
        ("squad_id" = String, Path, description = "Squad UUID.")
    ),
    request_body = ReasonRequest,
    responses(
        (status = 200, description = "Restrict squad from normal member management actions.", body = AdminSquadResponse),
        (status = 400, description = "Invalid squad ID."),
        (status = 401, description = "Missing bearer token."),
        (status = 403, description = "Superuser permissions required."),
        (status = 404, description = "Squad not found.")
    ),
    security(("bearer_auth" = [])),
    tag = "admin"
)]
pub async fn restrict_squad(
    State(state): AppStateExtractor,
    headers: axum::http::HeaderMap,
    Path(squad_id): Path<String>,
    Json(body): Json<ReasonRequest>,
) -> HttpResult<Json<AdminSquadResponse>> {
    let state = state.read().await;
    let admin = require_superuser(&headers, &state).await?;
    update_squad_restricted_flag(&state, admin.id, squad_id, true, body.reason, ACTION_ADMIN_SQUAD_RESTRICTED).await
}

#[utoipa::path(
    post,
    path = "/api/admin/squads/{squad_id}/unrestrict",
    params(
        ("squad_id" = String, Path, description = "Squad UUID.")
    ),
    request_body = ReasonRequest,
    responses(
        (status = 200, description = "Remove squad restriction.", body = AdminSquadResponse),
        (status = 400, description = "Invalid squad ID."),
        (status = 401, description = "Missing bearer token."),
        (status = 403, description = "Superuser permissions required."),
        (status = 404, description = "Squad not found.")
    ),
    security(("bearer_auth" = [])),
    tag = "admin"
)]
pub async fn unrestrict_squad(
    State(state): AppStateExtractor,
    headers: axum::http::HeaderMap,
    Path(squad_id): Path<String>,
    Json(body): Json<ReasonRequest>,
) -> HttpResult<Json<AdminSquadResponse>> {
    let state = state.read().await;
    let admin = require_superuser(&headers, &state).await?;
    update_squad_restricted_flag(&state, admin.id, squad_id, false, body.reason, ACTION_ADMIN_SQUAD_UNRESTRICTED).await
}

#[utoipa::path(
    delete,
    path = "/api/admin/squads/{squad_id}",
    params(
        ("squad_id" = String, Path, description = "Squad UUID.")
    ),
    responses(
        (status = 200, description = "Force-delete squad and detach all users from it.", body = ActionResponse),
        (status = 400, description = "Invalid squad ID."),
        (status = 401, description = "Missing bearer token."),
        (status = 403, description = "Superuser permissions required."),
        (status = 404, description = "Squad not found.")
    ),
    security(("bearer_auth" = [])),
    tag = "admin"
)]
pub async fn delete_squad(
    State(state): AppStateExtractor,
    headers: axum::http::HeaderMap,
    Path(squad_id): Path<String>,
) -> HttpResult<Json<ActionResponse>> {
    let state = state.read().await;
    let admin = require_superuser(&headers, &state).await?;
    let squad = get_squad_model(&state, &squad_id).await?;

    let tx = state
        .db
        .begin()
        .await
        .map_err(|e| HttpError::internal_error(format!("Failed to start transaction: {e}")))?;

    User::update_many()
        .col_expr(UserColumn::SquadId, sea_orm::sea_query::Expr::value(Option::<Uuid>::None))
        .filter(UserColumn::SquadId.eq(squad.id))
        .exec(&tx)
        .await
        .map_err(|e| HttpError::internal_error(format!("Failed to clear squad members: {e}")))?;

    SquadInvite::delete_many()
        .filter(SquadInviteColumn::SquadId.eq(squad.id))
        .exec(&tx)
        .await
        .map_err(|e| HttpError::internal_error(format!("Failed to delete squad invites: {e}")))?;

    let image_key = squad.image_key.clone();
    let active_squad: SquadActiveModel = squad.clone().into();
    active_squad
        .delete(&tx)
        .await
        .map_err(|e| HttpError::internal_error(format!("Failed to delete squad: {e}")))?;

    write_audit_log(
        &tx,
        ACTION_ADMIN_SQUAD_DELETED,
        Some(admin.id),
        Some(squad.leader_user_id),
        None,
        Some(json!({ "squadId": squad.id })),
    )
    .await
    .map_err(|e| HttpError::internal_error(format!("Failed to write audit log: {e}")))?;

    tx.commit()
        .await
        .map_err(|e| HttpError::internal_error(format!("Failed to commit transaction: {e}")))?;

    if let Some(key) = image_key {
        delete_image_object(&state, &key).await?;
    }

    Ok(Json(ActionResponse { status: "ok" }))
}

#[utoipa::path(
    post,
    path = "/api/admin/squads/{squad_id}/members/{user_id}/kick",
    params(
        ("squad_id" = String, Path, description = "Squad UUID."),
        ("user_id" = String, Path, description = "Target user UUID.")
    ),
    responses(
        (status = 200, description = "Admin removed a member from squad.", body = ActionResponse),
        (status = 400, description = "Invalid ids, leader target, or target user not in squad."),
        (status = 401, description = "Missing bearer token."),
        (status = 403, description = "Superuser permissions required."),
        (status = 404, description = "Squad or user not found.")
    ),
    security(("bearer_auth" = [])),
    tag = "admin"
)]
pub async fn kick_member(
    State(state): AppStateExtractor,
    headers: axum::http::HeaderMap,
    Path((squad_id, user_id)): Path<(String, String)>,
) -> HttpResult<Json<ActionResponse>> {
    let state = state.read().await;
    let admin = require_superuser(&headers, &state).await?;
    let squad = get_squad_model(&state, &squad_id).await?;
    let target_user_id = parse_uuid(&user_id, "Invalid target user ID")?;

    if target_user_id == squad.leader_user_id {
        return Err(HttpError::bad_request("Leader cannot be kicked from squad"));
    }

    let target_user = User::find_by_id(target_user_id)
        .one(&state.db)
        .await
        .map_err(|e| HttpError::internal_error(format!("Failed to load user: {e}")))?
        .ok_or_else(|| HttpError::not_found("User not found"))?;
    if target_user.squad_id != Some(squad.id) {
        return Err(HttpError::bad_request("User is not a member of this squad"));
    }

    let mut active_user: UserActiveModel = target_user.clone().into();
    active_user.squad_id = Set(None);
    active_user
        .update(&state.db)
        .await
        .map_err(|e| HttpError::internal_error(format!("Failed to kick member: {e}")))?;

    write_audit_log(
        &state.db,
        ACTION_ADMIN_SQUAD_MEMBER_KICKED,
        Some(admin.id),
        Some(target_user.id),
        None,
        Some(json!({ "squadId": squad.id })),
    )
    .await
    .map_err(|e| HttpError::internal_error(format!("Failed to write audit log: {e}")))?;

    Ok(Json(ActionResponse { status: "ok" }))
}

#[utoipa::path(
    post,
    path = "/api/admin/squads/{squad_id}/image",
    params(
        ("squad_id" = String, Path, description = "Squad UUID.")
    ),
    request_body(
        content_type = "multipart/form-data",
        description = "First image part is taken, validated as an image, normalized, and uploaded as the squad avatar."
    ),
    responses(
        (status = 200, description = "Admin uploaded or replaced the squad image.", body = AdminSquadResponse),
        (status = 400, description = "Invalid squad ID, invalid multipart body, missing image, or uploaded file is not an image."),
        (status = 401, description = "Missing bearer token."),
        (status = 403, description = "Superuser permissions required."),
        (status = 404, description = "Squad not found.")
    ),
    security(("bearer_auth" = [])),
    tag = "admin"
)]
pub async fn upload_squad_image(
    State(state): AppStateExtractor,
    headers: axum::http::HeaderMap,
    Path(squad_id): Path<String>,
    mut multipart: Multipart,
) -> HttpResult<Json<AdminSquadResponse>> {
    let state = state.read().await;
    let admin = require_superuser(&headers, &state).await?;
    let squad = get_squad_model(&state, &squad_id).await?;
    let data = read_first_image(&mut multipart).await?;
    let processed = process_squad_image(&data).map_err(|e| HttpError::bad_request(e.to_string()))?;
    let key = squad_image_key(squad.id);

    state
        .s3
        .put_object()
        .bucket(&state.config.s3.bucket)
        .key(&key)
        .content_type("image/png")
        .body(ByteStream::from(processed))
        .send()
        .await
        .map_err(|e| HttpError::internal_error(format!("Failed to upload squad image: {e}")))?;

    let mut active_squad: SquadActiveModel = squad.into();
    active_squad.image_key = Set(Some(key.clone()));
    active_squad.image_content_type = Set(Some("image/png".to_string()));
    active_squad.updated_at = Set(chrono::Utc::now());
    let updated_squad = active_squad
        .update(&state.db)
        .await
        .map_err(|e| HttpError::internal_error(format!("Failed to update squad image: {e}")))?;

    write_audit_log(
        &state.db,
        ACTION_ADMIN_SQUAD_IMAGE_UPDATED,
        Some(admin.id),
        Some(updated_squad.leader_user_id),
        None,
        Some(json!({ "squadId": updated_squad.id })),
    )
    .await
    .map_err(|e| HttpError::internal_error(format!("Failed to write audit log: {e}")))?;

    Ok(Json(to_squad_response(&state, updated_squad).await?))
}

#[utoipa::path(
    delete,
    path = "/api/admin/squads/{squad_id}/image",
    params(
        ("squad_id" = String, Path, description = "Squad UUID.")
    ),
    responses(
        (status = 200, description = "Admin removed the squad image. Repeated delete is effectively idempotent.", body = AdminSquadResponse),
        (status = 400, description = "Invalid squad ID."),
        (status = 401, description = "Missing bearer token."),
        (status = 403, description = "Superuser permissions required."),
        (status = 404, description = "Squad not found.")
    ),
    security(("bearer_auth" = [])),
    tag = "admin"
)]
pub async fn delete_squad_image(
    State(state): AppStateExtractor,
    headers: axum::http::HeaderMap,
    Path(squad_id): Path<String>,
) -> HttpResult<Json<AdminSquadResponse>> {
    let state = state.read().await;
    let admin = require_superuser(&headers, &state).await?;
    let squad = get_squad_model(&state, &squad_id).await?;
    if let Some(key) = squad.image_key.clone() {
        delete_image_object(&state, &key).await?;
    }

    let mut active_squad: SquadActiveModel = squad.into();
    active_squad.image_key = Set(None);
    active_squad.image_content_type = Set(None);
    active_squad.updated_at = Set(chrono::Utc::now());
    let updated_squad = active_squad
        .update(&state.db)
        .await
        .map_err(|e| HttpError::internal_error(format!("Failed to delete squad image: {e}")))?;

    write_audit_log(
        &state.db,
        ACTION_ADMIN_SQUAD_IMAGE_DELETED,
        Some(admin.id),
        Some(updated_squad.leader_user_id),
        None,
        Some(json!({ "squadId": updated_squad.id })),
    )
    .await
    .map_err(|e| HttpError::internal_error(format!("Failed to write audit log: {e}")))?;

    Ok(Json(to_squad_response(&state, updated_squad).await?))
}

async fn update_squad_restricted_flag(
    state: &AppState,
    admin_user_id: Uuid,
    squad_id: String,
    is_restricted: bool,
    reason: Option<String>,
    action: &str,
) -> HttpResult<Json<AdminSquadResponse>> {
    let squad = get_squad_model(state, &squad_id).await?;
    let mut active_squad: SquadActiveModel = squad.into();
    active_squad.is_restricted = Set(is_restricted);
    active_squad.restriction_reason = Set(reason.clone());
    active_squad.updated_at = Set(chrono::Utc::now());

    let updated_squad = active_squad
        .update(&state.db)
        .await
        .map_err(|e| HttpError::internal_error(format!("Failed to update squad restriction: {e}")))?;

    write_audit_log(
        &state.db,
        action,
        Some(admin_user_id),
        Some(updated_squad.leader_user_id),
        reason,
        Some(json!({ "squadId": updated_squad.id, "isRestricted": updated_squad.is_restricted })),
    )
    .await
    .map_err(|e| HttpError::internal_error(format!("Failed to write audit log: {e}")))?;

    Ok(Json(to_squad_response(&state, updated_squad).await?))
}

async fn get_squad_model(state: &AppState, squad_id: &str) -> HttpResult<crate::entities::SquadModel> {
    let squad_id = parse_uuid(squad_id, "Invalid squad ID")?;
    Squad::find_by_id(squad_id)
        .one(&state.db)
        .await
        .map_err(|e| HttpError::internal_error(format!("Failed to load squad: {e}")))?
        .ok_or_else(|| HttpError::not_found("Squad not found"))
}

async fn to_squad_response(
    state: &AppState,
    squad: crate::entities::SquadModel,
) -> HttpResult<AdminSquadResponse> {
    let member_count = User::find()
        .filter(UserColumn::SquadId.eq(squad.id))
        .count(&state.db)
        .await
        .map_err(|e| HttpError::internal_error(format!("Failed to count squad members: {e}")))?;

    let squad_id = squad.id;
    Ok(AdminSquadResponse {
        id: squad_id.to_string(),
        name: squad.name,
        leader_user_id: squad.leader_user_id.to_string(),
        member_count,
        max_members: SQUAD_MAX_MEMBERS,
        image_url: squad.image_key.map(|_| format!("/api/squads/{}/image", squad_id)),
        is_restricted: squad.is_restricted,
        restriction_reason: squad.restriction_reason,
        created_at: squad.created_at,
        updated_at: squad.updated_at,
    })
}

fn parse_uuid(input: &str, message: &str) -> HttpResult<Uuid> {
    Uuid::parse_str(input).map_err(|_| HttpError::bad_request(message))
}

async fn read_first_image(multipart: &mut Multipart) -> HttpResult<Bytes> {
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| HttpError::bad_request(format!("Invalid multipart body: {e}")))? {
        let content_type = field
            .content_type()
            .ok_or_else(|| HttpError::bad_request("Missing content type"))?;
        if !content_type.starts_with("image/") {
            return Err(HttpError::bad_request("Uploaded file must be an image"));
        }
        let data = field
            .bytes()
            .await
            .map_err(|e| HttpError::bad_request(format!("Failed to read uploaded image: {e}")))?;
        return Ok(data);
    }

    Err(HttpError::bad_request("Image file is required"))
}

async fn delete_image_object(state: &AppState, key: &str) -> HttpResult<()> {
    state
        .s3
        .delete_object()
        .bucket(&state.config.s3.bucket)
        .key(key)
        .send()
        .await
        .map_err(|e| HttpError::internal_error(format!("Failed to delete image object: {e}")))?;
    Ok(())
}
