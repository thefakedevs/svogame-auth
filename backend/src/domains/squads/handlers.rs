use aws_sdk_s3::primitives::ByteStream;
use axum::Json;
use axum::body::Bytes;
use axum::extract::{Multipart, Path, State};
use axum::http::HeaderMap;
use axum::http::header;
use axum::response::{IntoResponse, Response};
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, ConnectionTrait, EntityTrait, PaginatorTrait,
    QueryFilter, QueryOrder, Set, TransactionTrait,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::app::auth::get_user_from_headers;
use crate::app::http::{HttpError, HttpResult};
use crate::app::state::{AppState, AppStateExtractor};
use crate::entities::{
    Squad, SquadActiveModel, SquadInvite, SquadInviteActiveModel, SquadInviteColumn,
    SquadInviteModel, SquadModel, User, UserActiveModel, UserColumn, UserModel,
};
use crate::services::audit::{
    ACTION_USER_SQUAD_CREATED, ACTION_USER_SQUAD_DISBANDED, ACTION_USER_SQUAD_IMAGE_DELETED,
    ACTION_USER_SQUAD_IMAGE_UPDATED, ACTION_USER_SQUAD_INVITE_CREATED, ACTION_USER_SQUAD_KICKED,
    ACTION_USER_SQUAD_LEFT, ACTION_USER_SQUAD_UPDATED, write_audit_log,
};
use crate::services::restrictions::{RestrictionKind, has_restriction};
use crate::services::squads::{
    SQUAD_INVITE_TTL_HOURS, SQUAD_MAX_MEMBERS, process_squad_image, squad_image_key,
    validate_squad_name,
};

#[derive(Deserialize, ToSchema)]
pub struct CreateSquadRequest {
    pub name: String,
}

#[derive(Deserialize, ToSchema)]
pub struct PatchSquadRequest {
    pub name: String,
}

#[derive(Deserialize, ToSchema)]
pub struct CreateInviteRequest {
    #[serde(rename = "userId")]
    pub user_id: String,
}

#[derive(Serialize, ToSchema)]
pub struct SquadMemberResponse {
    pub id: String,
    pub username: String,
    #[serde(rename = "avatarUrl")]
    pub avatar_url: Option<String>,
    #[serde(rename = "isLeader")]
    pub is_leader: bool,
    #[serde(rename = "isPendingInvite")]
    pub is_pending_invite: bool,
    #[serde(rename = "inviteId")]
    pub invite_id: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct SquadResponse {
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
pub struct SquadInviteResponse {
    pub id: String,
    #[serde(rename = "squadId")]
    pub squad_id: String,
    #[serde(rename = "squadName")]
    pub squad_name: String,
    #[serde(rename = "inviterUserId")]
    pub inviter_user_id: String,
    #[serde(rename = "inviterUsername")]
    pub inviter_username: String,
    #[serde(rename = "inviterAvatarUrl")]
    pub inviter_avatar_url: Option<String>,
    #[serde(rename = "invitedUserId")]
    pub invited_user_id: String,
    #[serde(rename = "invitedUsername")]
    pub invited_username: String,
    #[serde(rename = "expiresAt")]
    pub expires_at: chrono::DateTime<chrono::Utc>,
    #[serde(rename = "createdAt")]
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Serialize, ToSchema)]
pub struct SquadActionResponse {
    pub status: &'static str,
}

#[utoipa::path(
    post,
    path = "/api/squads",
    request_body(
        content = CreateSquadRequest,
        description = "Create a squad and make current user its leader. User must not already be in a squad and must not have a create-squad restriction."
    ),
    responses(
        (status = 200, description = "Squad created.", body = SquadResponse),
        (status = 400, description = "Invalid squad name."),
        (status = 401, description = "Missing bearer token."),
        (status = 403, description = "User already in squad or restricted from squad creation."),
        (status = 500, description = "Database or audit failure.")
    ),
    security(("bearer_auth" = [])),
    tag = "squads"
)]
pub async fn create_squad(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Json(body): Json<CreateSquadRequest>,
) -> HttpResult<Json<SquadResponse>> {
    let state = state.read().await;
    let user = get_user_from_headers(&headers, &state).await?;
    ensure_can_create_squad(&state, &user).await?;
    let name =
        validate_squad_name(&body.name).map_err(|e| HttpError::bad_request(e.to_string()))?;

    let tx = state
        .db
        .begin()
        .await
        .map_err(|e| HttpError::internal_error(format!("Failed to start transaction: {e}")))?;

    let squad = SquadActiveModel {
        id: ActiveValue::Set(Uuid::new_v4()),
        leader_user_id: ActiveValue::Set(user.id),
        name: ActiveValue::Set(name.clone()),
        image_key: ActiveValue::Set(None),
        image_content_type: ActiveValue::Set(None),
        is_restricted: ActiveValue::Set(false),
        restriction_reason: ActiveValue::Set(None),
        created_at: ActiveValue::Set(chrono::Utc::now()),
        updated_at: ActiveValue::Set(chrono::Utc::now()),
    }
    .insert(&tx)
    .await
    .map_err(|e| HttpError::internal_error(format!("Failed to create squad: {e}")))?;

    let mut user_active: UserActiveModel = user.clone().into();
    user_active.squad_id = Set(Some(squad.id));
    user_active
        .update(&tx)
        .await
        .map_err(|e| HttpError::internal_error(format!("Failed to attach squad to user: {e}")))?;

    write_audit_log(
        &tx,
        ACTION_USER_SQUAD_CREATED,
        Some(user.id),
        Some(user.id),
        None,
        Some(json!({ "squadId": squad.id, "name": squad.name })),
    )
    .await
    .map_err(|e| HttpError::internal_error(format!("Failed to write audit log: {e}")))?;

    tx.commit()
        .await
        .map_err(|e| HttpError::internal_error(format!("Failed to commit transaction: {e}")))?;

    Ok(Json(to_squad_response(squad, 1)))
}

#[utoipa::path(
    get,
    path = "/api/squads/{squad_id}",
    params(
        ("squad_id" = String, Path, description = "Squad UUID.")
    ),
    responses(
        (status = 200, description = "Get squad summary by id.", body = SquadResponse),
        (status = 400, description = "Invalid squad ID."),
        (status = 404, description = "Squad not found.")
    ),
    tag = "squads"
)]
pub async fn get_squad(
    State(state): AppStateExtractor,
    Path(squad_id): Path<String>,
) -> HttpResult<Json<SquadResponse>> {
    let state = state.read().await;
    let squad = get_squad_by_id(&state.db, &squad_id).await?;
    let member_count = squad_member_count(&state.db, squad.id).await?;
    Ok(Json(to_squad_response(squad, member_count)))
}

#[utoipa::path(
    get,
    path = "/api/squads/{squad_id}/members",
    params(
        ("squad_id" = String, Path, description = "Squad UUID.")
    ),
    responses(
        (status = 200, description = "List squad members and active pending invites in display order. Pending invite rows are marked with `isPendingInvite=true` and carry `inviteId`.", body = [SquadMemberResponse]),
        (status = 400, description = "Invalid squad ID."),
        (status = 404, description = "Squad not found.")
    ),
    tag = "squads"
)]
pub async fn get_squad_members(
    State(state): AppStateExtractor,
    Path(squad_id): Path<String>,
) -> HttpResult<Json<Vec<SquadMemberResponse>>> {
    let state = state.read().await;
    let squad = get_squad_by_id(&state.db, &squad_id).await?;
    let members = load_squad_members(&state.db, squad.id).await?;
    let pending_invites = load_squad_pending_invites(&state.db, squad.id).await?;

    let mut response: Vec<SquadMemberResponse> = members
        .into_iter()
        .map(|user| SquadMemberResponse {
            id: user.id.to_string(),
            username: user.username,
            avatar_url: user.avatar_url,
            is_leader: user.id == squad.leader_user_id,
            is_pending_invite: false,
            invite_id: None,
        })
        .collect();
    response.extend(
        pending_invites
            .into_iter()
            .map(|(invite, user)| SquadMemberResponse {
                id: user.id.to_string(),
                username: user.username,
                avatar_url: user.avatar_url,
                is_leader: false,
                is_pending_invite: true,
                invite_id: Some(invite.id.to_string()),
            }),
    );

    Ok(Json(response))
}

#[utoipa::path(
    patch,
    path = "/api/squads/{squad_id}",
    params(
        ("squad_id" = String, Path, description = "Squad UUID.")
    ),
    request_body(
        content = PatchSquadRequest,
        description = "Rename squad. Only current leader can do this. Restricted squads cannot be managed by members."
    ),
    responses(
        (status = 200, description = "Squad updated.", body = SquadResponse),
        (status = 400, description = "Invalid squad ID or invalid squad name."),
        (status = 401, description = "Missing bearer token."),
        (status = 403, description = "Only leader can update squad, or squad is restricted."),
        (status = 404, description = "Squad not found.")
    ),
    security(("bearer_auth" = [])),
    tag = "squads"
)]
pub async fn patch_squad(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Path(squad_id): Path<String>,
    Json(body): Json<PatchSquadRequest>,
) -> HttpResult<Json<SquadResponse>> {
    let state = state.read().await;
    let user = get_user_from_headers(&headers, &state).await?;
    let squad = require_squad_leader(&state, &user, &squad_id).await?;
    ensure_squad_not_restricted_for_member_actions(&squad)?;
    let name =
        validate_squad_name(&body.name).map_err(|e| HttpError::bad_request(e.to_string()))?;

    let mut active_squad: SquadActiveModel = squad.into();
    active_squad.name = Set(name.clone());
    active_squad.updated_at = Set(chrono::Utc::now());
    let updated_squad = active_squad
        .update(&state.db)
        .await
        .map_err(|e| HttpError::internal_error(format!("Failed to update squad: {e}")))?;

    write_audit_log(
        &state.db,
        ACTION_USER_SQUAD_UPDATED,
        Some(user.id),
        Some(user.id),
        None,
        Some(json!({ "squadId": updated_squad.id, "name": updated_squad.name })),
    )
    .await
    .map_err(|e| HttpError::internal_error(format!("Failed to write audit log: {e}")))?;

    let member_count = squad_member_count(&state.db, updated_squad.id).await?;
    Ok(Json(to_squad_response(updated_squad, member_count)))
}

#[utoipa::path(
    get,
    path = "/api/squads/{squad_id}/image",
    params(
        ("squad_id" = String, Path, description = "Squad UUID.")
    ),
    responses(
        (status = 200, description = "Get squad image as PNG when configured.", content_type = "image/png"),
        (status = 400, description = "Invalid squad ID."),
        (status = 404, description = "Squad not found or squad has no image.")
    ),
    tag = "squads"
)]
pub async fn get_squad_image(
    State(state): AppStateExtractor,
    Path(squad_id): Path<String>,
) -> HttpResult<Response> {
    let state = state.read().await;
    let squad = get_squad_by_id(&state.db, &squad_id).await?;
    let image_key = squad
        .image_key
        .ok_or_else(|| HttpError::not_found("Squad image not found"))?;

    let bytes = state
        .s3
        .get_object()
        .bucket(&state.config.s3.bucket)
        .key(&image_key)
        .send()
        .await
        .map_err(|e| HttpError::not_found(format!("Failed to load squad image: {e}")))?
        .body
        .collect()
        .await
        .map_err(|e| HttpError::internal_error(format!("Failed to read squad image body: {e}")))?
        .into_bytes();

    Ok(([(header::CONTENT_TYPE, "image/png")], bytes).into_response())
}

#[utoipa::path(
    post,
    path = "/api/squads/{squad_id}/image",
    params(
        ("squad_id" = String, Path, description = "Squad UUID.")
    ),
    request_body(
        content = String,
        content_type = "multipart/form-data",
        description = "Upload squad image as multipart form-data. Only the first image part is used. Restricted squads cannot be managed by members."
    ),
    responses(
        (status = 200, description = "Squad image uploaded and processed to PNG.", body = SquadResponse),
        (status = 400, description = "Invalid squad ID, malformed multipart body, unsupported content, or image validation failed."),
        (status = 401, description = "Missing bearer token."),
        (status = 403, description = "Only leader can upload image, or squad is restricted."),
        (status = 404, description = "Squad not found.")
    ),
    security(("bearer_auth" = [])),
    tag = "squads"
)]
pub async fn upload_squad_image(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Path(squad_id): Path<String>,
    mut multipart: Multipart,
) -> HttpResult<Json<SquadResponse>> {
    let state = state.read().await;
    let user = get_user_from_headers(&headers, &state).await?;
    let squad = require_squad_leader(&state, &user, &squad_id).await?;
    ensure_squad_not_restricted_for_member_actions(&squad)?;
    let data = read_first_image(&mut multipart).await?;
    let processed =
        process_squad_image(&data).map_err(|e| HttpError::bad_request(e.to_string()))?;
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
        ACTION_USER_SQUAD_IMAGE_UPDATED,
        Some(user.id),
        Some(user.id),
        None,
        Some(json!({ "squadId": updated_squad.id })),
    )
    .await
    .map_err(|e| HttpError::internal_error(format!("Failed to write audit log: {e}")))?;

    let member_count = squad_member_count(&state.db, updated_squad.id).await?;
    Ok(Json(to_squad_response(updated_squad, member_count)))
}

#[utoipa::path(
    delete,
    path = "/api/squads/{squad_id}/image",
    params(
        ("squad_id" = String, Path, description = "Squad UUID.")
    ),
    responses(
        (status = 200, description = "Squad image removed.", body = SquadResponse),
        (status = 400, description = "Invalid squad ID."),
        (status = 401, description = "Missing bearer token."),
        (status = 403, description = "Only leader can delete image, or squad is restricted."),
        (status = 404, description = "Squad not found.")
    ),
    security(("bearer_auth" = [])),
    tag = "squads"
)]
pub async fn delete_squad_image(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Path(squad_id): Path<String>,
) -> HttpResult<Json<SquadResponse>> {
    let state = state.read().await;
    let user = get_user_from_headers(&headers, &state).await?;
    let squad = require_squad_leader(&state, &user, &squad_id).await?;
    ensure_squad_not_restricted_for_member_actions(&squad)?;

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
        ACTION_USER_SQUAD_IMAGE_DELETED,
        Some(user.id),
        Some(user.id),
        None,
        Some(json!({ "squadId": updated_squad.id })),
    )
    .await
    .map_err(|e| HttpError::internal_error(format!("Failed to write audit log: {e}")))?;

    let member_count = squad_member_count(&state.db, updated_squad.id).await?;
    Ok(Json(to_squad_response(updated_squad, member_count)))
}

#[utoipa::path(
    delete,
    path = "/api/squads/{squad_id}",
    params(
        ("squad_id" = String, Path, description = "Squad UUID.")
    ),
    responses(
        (status = 200, description = "Disband squad. Leader can always delete the squad, even when it is restricted.", body = SquadActionResponse),
        (status = 400, description = "Invalid squad ID."),
        (status = 401, description = "Missing bearer token."),
        (status = 403, description = "Only leader can delete squad."),
        (status = 404, description = "Squad not found.")
    ),
    security(("bearer_auth" = [])),
    tag = "squads"
)]
pub async fn delete_squad(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Path(squad_id): Path<String>,
) -> HttpResult<Json<SquadActionResponse>> {
    let state = state.read().await;
    let user = get_user_from_headers(&headers, &state).await?;
    let squad = require_squad_leader(&state, &user, &squad_id).await?;

    let tx = state
        .db
        .begin()
        .await
        .map_err(|e| HttpError::internal_error(format!("Failed to start transaction: {e}")))?;

    let members = load_squad_members(&tx, squad.id).await?;
    for member in members {
        let mut active_member: UserActiveModel = member.into();
        active_member.squad_id = Set(None);
        active_member.update(&tx).await.map_err(|e| {
            HttpError::internal_error(format!("Failed to detach squad member: {e}"))
        })?;
    }

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
        ACTION_USER_SQUAD_DISBANDED,
        Some(user.id),
        Some(user.id),
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

    Ok(Json(SquadActionResponse { status: "ok" }))
}

#[utoipa::path(
    post,
    path = "/api/squads/{squad_id}/leave",
    params(
        ("squad_id" = String, Path, description = "Squad UUID.")
    ),
    responses(
        (status = 200, description = "Leave squad. Leaders cannot leave and must delete the squad instead.", body = SquadActionResponse),
        (status = 400, description = "Invalid squad ID."),
        (status = 401, description = "Missing bearer token."),
        (status = 403, description = "User is not a squad member or is the leader."),
        (status = 404, description = "Squad not found.")
    ),
    security(("bearer_auth" = [])),
    tag = "squads"
)]
pub async fn leave_squad(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Path(squad_id): Path<String>,
) -> HttpResult<Json<SquadActionResponse>> {
    let state = state.read().await;
    let user = get_user_from_headers(&headers, &state).await?;
    let squad = get_squad_by_id(&state.db, &squad_id).await?;

    if user.squad_id != Some(squad.id) {
        return Err(HttpError::forbidden("User is not a member of this squad"));
    }
    if squad.leader_user_id == user.id {
        return Err(HttpError::forbidden(
            "Leader cannot leave the squad. Delete it instead",
        ));
    }

    let mut active_user: UserActiveModel = user.clone().into();
    active_user.squad_id = Set(None);
    active_user
        .update(&state.db)
        .await
        .map_err(|e| HttpError::internal_error(format!("Failed to leave squad: {e}")))?;

    write_audit_log(
        &state.db,
        ACTION_USER_SQUAD_LEFT,
        Some(user.id),
        Some(user.id),
        None,
        Some(json!({ "squadId": squad.id })),
    )
    .await
    .map_err(|e| HttpError::internal_error(format!("Failed to write audit log: {e}")))?;

    Ok(Json(SquadActionResponse { status: "ok" }))
}

#[utoipa::path(
    post,
    path = "/api/squads/{squad_id}/invites",
    params(
        ("squad_id" = String, Path, description = "Squad UUID.")
    ),
    request_body(
        content = CreateInviteRequest,
        description = "Create invite for another user. Only current leader can do this. Restricted squads cannot issue invites. Creating the same active invite twice returns the existing invite."
    ),
    responses(
        (status = 200, description = "Invite created or existing active invite returned.", body = SquadInviteResponse),
        (status = 400, description = "Invalid IDs, invited user already in squad, squad full, or invalid payload."),
        (status = 401, description = "Missing bearer token."),
        (status = 403, description = "Only leader can invite, or squad is restricted."),
        (status = 404, description = "Squad or invited user not found.")
    ),
    security(("bearer_auth" = [])),
    tag = "squads"
)]
pub async fn create_invite(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Path(squad_id): Path<String>,
    Json(body): Json<CreateInviteRequest>,
) -> HttpResult<Json<SquadInviteResponse>> {
    let state = state.read().await;
    let user = get_user_from_headers(&headers, &state).await?;
    let squad = require_squad_leader(&state, &user, &squad_id).await?;
    ensure_squad_not_restricted_for_member_actions(&squad)?;

    cleanup_expired_invites(&state.db, squad.id).await?;

    let invited_user_id = parse_uuid(&body.user_id, "Invalid invited user ID")?;
    let invited_user = User::find_by_id(invited_user_id)
        .one(&state.db)
        .await
        .map_err(|e| HttpError::internal_error(format!("Failed to load invited user: {e}")))?
        .ok_or_else(|| HttpError::not_found("Invited user not found"))?;

    if invited_user.squad_id.is_some() {
        return Err(HttpError::bad_request("Invited user is already in a squad"));
    }
    let member_count = squad_member_count(&state.db, squad.id).await?;
    let active_invites = SquadInvite::find()
        .filter(SquadInviteColumn::SquadId.eq(squad.id))
        .filter(SquadInviteColumn::ExpiresAt.gt(chrono::Utc::now()))
        .count(&state.db)
        .await
        .map_err(|e| HttpError::internal_error(format!("Failed to count invites: {e}")))?;
    if member_count + active_invites >= SQUAD_MAX_MEMBERS {
        return Err(HttpError::bad_request("Squad is full"));
    }

    if let Some(existing_invite) = SquadInvite::find()
        .filter(SquadInviteColumn::SquadId.eq(squad.id))
        .filter(SquadInviteColumn::InvitedUserId.eq(invited_user.id))
        .filter(SquadInviteColumn::ExpiresAt.gt(chrono::Utc::now()))
        .one(&state.db)
        .await
        .map_err(|e| HttpError::internal_error(format!("Failed to check existing invite: {e}")))?
    {
        return Ok(Json(to_invite_response(
            existing_invite,
            squad.name,
            user.username,
            user.avatar_url,
            invited_user.username,
        )));
    }

    let invite = SquadInviteActiveModel {
        id: ActiveValue::Set(Uuid::new_v4()),
        squad_id: ActiveValue::Set(squad.id),
        inviter_user_id: ActiveValue::Set(user.id),
        invited_user_id: ActiveValue::Set(invited_user.id),
        expires_at: ActiveValue::Set(
            chrono::Utc::now() + chrono::Duration::hours(SQUAD_INVITE_TTL_HOURS),
        ),
        created_at: ActiveValue::Set(chrono::Utc::now()),
    }
    .insert(&state.db)
    .await
    .map_err(|e| HttpError::internal_error(format!("Failed to create invite: {e}")))?;

    write_audit_log(
        &state.db,
        ACTION_USER_SQUAD_INVITE_CREATED,
        Some(user.id),
        Some(invited_user.id),
        None,
        Some(json!({ "squadId": squad.id, "inviteId": invite.id })),
    )
    .await
    .map_err(|e| HttpError::internal_error(format!("Failed to write audit log: {e}")))?;

    Ok(Json(to_invite_response(
        invite,
        squad.name,
        user.username,
        user.avatar_url,
        invited_user.username,
    )))
}

#[utoipa::path(
    get,
    path = "/api/user/me/squad-invites",
    responses(
        (status = 200, description = "List current user's active squad invites. Expired invites are filtered out.", body = [SquadInviteResponse]),
        (status = 401, description = "Missing bearer token."),
        (status = 403, description = "Token invalid or user inactive.")
    ),
    security(("bearer_auth" = [])),
    tag = "squads"
)]
pub async fn list_my_invites(
    State(state): AppStateExtractor,
    headers: HeaderMap,
) -> HttpResult<Json<Vec<SquadInviteResponse>>> {
    let state = state.read().await;
    let user = get_user_from_headers(&headers, &state).await?;

    let invites = SquadInvite::find()
        .filter(SquadInviteColumn::InvitedUserId.eq(user.id))
        .filter(SquadInviteColumn::ExpiresAt.gt(chrono::Utc::now()))
        .order_by_desc(SquadInviteColumn::CreatedAt)
        .all(&state.db)
        .await
        .map_err(|e| HttpError::internal_error(format!("Failed to list invites: {e}")))?;

    let squad_ids: Vec<Uuid> = invites.iter().map(|invite| invite.squad_id).collect();
    let squads = Squad::find()
        .filter(crate::entities::SquadColumn::Id.is_in(squad_ids))
        .all(&state.db)
        .await
        .map_err(|e| {
            HttpError::internal_error(format!("Failed to load squads for invites: {e}"))
        })?;
    let squads_by_id: HashMap<Uuid, SquadModel> =
        squads.into_iter().map(|squad| (squad.id, squad)).collect();

    let inviter_ids: Vec<Uuid> = invites
        .iter()
        .map(|invite| invite.inviter_user_id)
        .collect();
    let inviters = User::find()
        .filter(UserColumn::Id.is_in(inviter_ids))
        .all(&state.db)
        .await
        .map_err(|e| {
            HttpError::internal_error(format!("Failed to load inviters for invites: {e}"))
        })?;
    let inviters_by_id: HashMap<Uuid, UserModel> = inviters
        .into_iter()
        .map(|inviter| (inviter.id, inviter))
        .collect();

    let mut response = Vec::with_capacity(invites.len());
    for invite in invites {
        if let (Some(squad), Some(inviter)) = (
            squads_by_id.get(&invite.squad_id),
            inviters_by_id.get(&invite.inviter_user_id),
        ) {
            response.push(to_invite_response(
                invite,
                squad.name.clone(),
                inviter.username.clone(),
                inviter.avatar_url.clone(),
                user.username.clone(),
            ));
        }
    }

    Ok(Json(response))
}

#[utoipa::path(
    post,
    path = "/api/squad-invites/{invite_id}/accept",
    params(
        ("invite_id" = String, Path, description = "Invite UUID.")
    ),
    responses(
        (status = 200, description = "Invite accepted and user joins squad.", body = SquadActionResponse),
        (status = 400, description = "Invalid invite ID, invite expired, user already in squad, or squad full."),
        (status = 401, description = "Missing bearer token."),
        (status = 403, description = "Invite belongs to another user, target squad restricted, or user has join-squad restriction."),
        (status = 404, description = "Invite or squad not found.")
    ),
    security(("bearer_auth" = [])),
    tag = "squads"
)]
pub async fn accept_invite(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Path(invite_id): Path<String>,
) -> HttpResult<Json<SquadActionResponse>> {
    let state = state.read().await;
    let user = get_user_from_headers(&headers, &state).await?;
    let invite_id = parse_uuid(&invite_id, "Invalid invite ID")?;

    if has_restriction(&state.db, user.id, RestrictionKind::JoinSquad)
        .await
        .map_err(|e| HttpError::internal_error(format!("Failed to check restrictions: {e}")))?
    {
        return Err(HttpError::forbidden("User cannot join squads"));
    }
    if user.squad_id.is_some() {
        return Err(HttpError::bad_request("User is already in a squad"));
    }

    let tx = state
        .db
        .begin()
        .await
        .map_err(|e| HttpError::internal_error(format!("Failed to start transaction: {e}")))?;

    let invite = SquadInvite::find_by_id(invite_id)
        .one(&tx)
        .await
        .map_err(|e| HttpError::internal_error(format!("Failed to load invite: {e}")))?
        .ok_or_else(|| HttpError::not_found("Invite not found"))?;

    if invite.invited_user_id != user.id {
        return Err(HttpError::forbidden(
            "Invite does not belong to the current user",
        ));
    }
    if invite.expires_at <= chrono::Utc::now() {
        return Err(HttpError::bad_request("Invite has expired"));
    }

    let squad = Squad::find_by_id(invite.squad_id)
        .one(&tx)
        .await
        .map_err(|e| HttpError::internal_error(format!("Failed to load squad: {e}")))?
        .ok_or_else(|| HttpError::not_found("Squad not found"))?;
    if squad.is_restricted {
        return Err(HttpError::forbidden("Cannot join a restricted squad"));
    }

    let member_count = squad_member_count(&tx, squad.id).await?;
    if member_count >= SQUAD_MAX_MEMBERS {
        return Err(HttpError::bad_request("Squad is full"));
    }

    let mut user_active: UserActiveModel = user.clone().into();
    user_active.squad_id = Set(Some(squad.id));
    user_active
        .update(&tx)
        .await
        .map_err(|e| HttpError::internal_error(format!("Failed to join squad: {e}")))?;

    SquadInvite::delete_many()
        .filter(SquadInviteColumn::InvitedUserId.eq(user.id))
        .exec(&tx)
        .await
        .map_err(|e| HttpError::internal_error(format!("Failed to clear user invites: {e}")))?;

    write_audit_log(
        &tx,
        crate::services::audit::ACTION_USER_SQUAD_INVITE_ACCEPTED,
        Some(user.id),
        Some(user.id),
        None,
        Some(json!({ "squadId": squad.id, "inviteId": invite.id })),
    )
    .await
    .map_err(|e| HttpError::internal_error(format!("Failed to write audit log: {e}")))?;

    tx.commit()
        .await
        .map_err(|e| HttpError::internal_error(format!("Failed to commit transaction: {e}")))?;

    Ok(Json(SquadActionResponse { status: "ok" }))
}

#[utoipa::path(
    post,
    path = "/api/squad-invites/{invite_id}/decline",
    params(
        ("invite_id" = String, Path, description = "Invite UUID.")
    ),
    responses(
        (status = 200, description = "Invite declined and removed.", body = SquadActionResponse),
        (status = 400, description = "Invalid invite ID."),
        (status = 401, description = "Missing bearer token."),
        (status = 403, description = "Invite belongs to another user."),
        (status = 404, description = "Invite not found.")
    ),
    security(("bearer_auth" = [])),
    tag = "squads"
)]
pub async fn decline_invite(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Path(invite_id): Path<String>,
) -> HttpResult<Json<SquadActionResponse>> {
    let state = state.read().await;
    let user = get_user_from_headers(&headers, &state).await?;
    let invite = get_owned_invite(&state.db, &invite_id, user.id).await?;
    let active_invite: SquadInviteActiveModel = invite.clone().into();
    active_invite
        .delete(&state.db)
        .await
        .map_err(|e| HttpError::internal_error(format!("Failed to decline invite: {e}")))?;

    write_audit_log(
        &state.db,
        crate::services::audit::ACTION_USER_SQUAD_INVITE_DECLINED,
        Some(user.id),
        Some(user.id),
        None,
        Some(json!({ "inviteId": invite.id, "squadId": invite.squad_id })),
    )
    .await
    .map_err(|e| HttpError::internal_error(format!("Failed to write audit log: {e}")))?;

    Ok(Json(SquadActionResponse { status: "ok" }))
}

#[utoipa::path(
    delete,
    path = "/api/squad-invites/{invite_id}",
    params(
        ("invite_id" = String, Path, description = "Invite UUID.")
    ),
    responses(
        (status = 200, description = "Leader revoked invite.", body = SquadActionResponse),
        (status = 400, description = "Invalid invite ID."),
        (status = 401, description = "Missing bearer token."),
        (status = 403, description = "Only leader can revoke invite, or squad is restricted."),
        (status = 404, description = "Invite or squad not found.")
    ),
    security(("bearer_auth" = [])),
    tag = "squads"
)]
pub async fn revoke_invite(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Path(invite_id): Path<String>,
) -> HttpResult<Json<SquadActionResponse>> {
    let state = state.read().await;
    let user = get_user_from_headers(&headers, &state).await?;
    let invite_id = parse_uuid(&invite_id, "Invalid invite ID")?;
    let invite = SquadInvite::find_by_id(invite_id)
        .one(&state.db)
        .await
        .map_err(|e| HttpError::internal_error(format!("Failed to load invite: {e}")))?
        .ok_or_else(|| HttpError::not_found("Invite not found"))?;
    let squad = require_squad_leader(&state, &user, &invite.squad_id.to_string()).await?;
    ensure_squad_not_restricted_for_member_actions(&squad)?;

    let active_invite: SquadInviteActiveModel = invite.clone().into();
    active_invite
        .delete(&state.db)
        .await
        .map_err(|e| HttpError::internal_error(format!("Failed to revoke invite: {e}")))?;

    write_audit_log(
        &state.db,
        crate::services::audit::ACTION_USER_SQUAD_INVITE_REVOKED,
        Some(user.id),
        Some(invite.invited_user_id),
        None,
        Some(json!({ "inviteId": invite.id, "squadId": invite.squad_id })),
    )
    .await
    .map_err(|e| HttpError::internal_error(format!("Failed to write audit log: {e}")))?;

    Ok(Json(SquadActionResponse { status: "ok" }))
}

#[utoipa::path(
    post,
    path = "/api/squads/{squad_id}/members/{user_id}/kick",
    params(
        ("squad_id" = String, Path, description = "Squad UUID."),
        ("user_id" = String, Path, description = "Target user UUID.")
    ),
    responses(
        (status = 200, description = "Leader kicked a member.", body = SquadActionResponse),
        (status = 400, description = "Invalid IDs or target user is not a squad member."),
        (status = 401, description = "Missing bearer token."),
        (status = 403, description = "Only leader can kick members, squad is restricted, or leader tries to kick themselves."),
        (status = 404, description = "Squad or user not found.")
    ),
    security(("bearer_auth" = [])),
    tag = "squads"
)]
pub async fn kick_member(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Path((squad_id, user_id)): Path<(String, String)>,
) -> HttpResult<Json<SquadActionResponse>> {
    let state = state.read().await;
    let leader = get_user_from_headers(&headers, &state).await?;
    let squad = require_squad_leader(&state, &leader, &squad_id).await?;
    ensure_squad_not_restricted_for_member_actions(&squad)?;
    let target_user_id = parse_uuid(&user_id, "Invalid target user ID")?;

    if target_user_id == leader.id {
        return Err(HttpError::bad_request("Leader cannot kick themselves"));
    }

    let target_user = User::find_by_id(target_user_id)
        .one(&state.db)
        .await
        .map_err(|e| HttpError::internal_error(format!("Failed to load squad member: {e}")))?
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
        ACTION_USER_SQUAD_KICKED,
        Some(leader.id),
        Some(target_user.id),
        None,
        Some(json!({ "squadId": squad.id })),
    )
    .await
    .map_err(|e| HttpError::internal_error(format!("Failed to write audit log: {e}")))?;

    Ok(Json(SquadActionResponse { status: "ok" }))
}

#[utoipa::path(
    get,
    path = "/api/user/me/squad",
    responses(
        (status = 200, description = "Get current user's squad or `null` when user is not in any squad.", body = Option<SquadResponse>),
        (status = 401, description = "Missing bearer token."),
        (status = 403, description = "Token invalid or user inactive."),
        (status = 404, description = "Referenced squad record missing.")
    ),
    security(("bearer_auth" = [])),
    tag = "squads"
)]
pub async fn get_my_squad(
    State(state): AppStateExtractor,
    headers: HeaderMap,
) -> HttpResult<Json<Option<SquadResponse>>> {
    let state = state.read().await;
    let user = get_user_from_headers(&headers, &state).await?;

    let Some(squad_id) = user.squad_id else {
        return Ok(Json(None));
    };

    let squad = Squad::find_by_id(squad_id)
        .one(&state.db)
        .await
        .map_err(|e| HttpError::internal_error(format!("Failed to load squad: {e}")))?
        .ok_or_else(|| HttpError::not_found("Squad not found"))?;
    let member_count = squad_member_count(&state.db, squad.id).await?;

    Ok(Json(Some(to_squad_response(squad, member_count))))
}

fn to_squad_response(squad: SquadModel, member_count: u64) -> SquadResponse {
    let squad_id = squad.id;
    SquadResponse {
        id: squad_id.to_string(),
        name: squad.name,
        leader_user_id: squad.leader_user_id.to_string(),
        member_count,
        max_members: SQUAD_MAX_MEMBERS,
        image_url: squad
            .image_key
            .map(|_| format!("/api/squads/{}/image", squad_id)),
        is_restricted: squad.is_restricted,
        restriction_reason: squad.restriction_reason,
        created_at: squad.created_at,
        updated_at: squad.updated_at,
    }
}

fn to_invite_response(
    invite: SquadInviteModel,
    squad_name: String,
    inviter_username: String,
    inviter_avatar_url: Option<String>,
    invited_username: String,
) -> SquadInviteResponse {
    SquadInviteResponse {
        id: invite.id.to_string(),
        squad_id: invite.squad_id.to_string(),
        squad_name,
        inviter_user_id: invite.inviter_user_id.to_string(),
        inviter_username,
        inviter_avatar_url,
        invited_user_id: invite.invited_user_id.to_string(),
        invited_username,
        expires_at: invite.expires_at,
        created_at: invite.created_at,
    }
}

async fn ensure_can_create_squad(state: &AppState, user: &UserModel) -> HttpResult<()> {
    if user.squad_id.is_some() {
        return Err(HttpError::forbidden("User is already in a squad"));
    }
    if has_restriction(&state.db, user.id, RestrictionKind::CreateSquad)
        .await
        .map_err(|e| HttpError::internal_error(format!("Failed to check restrictions: {e}")))?
    {
        return Err(HttpError::forbidden("User cannot create squads"));
    }
    Ok(())
}

async fn require_squad_leader(
    state: &AppState,
    user: &UserModel,
    squad_id: &str,
) -> HttpResult<SquadModel> {
    let squad = get_squad_by_id(&state.db, squad_id).await?;
    if squad.leader_user_id != user.id || user.squad_id != Some(squad.id) {
        return Err(HttpError::forbidden(
            "Only squad leader can perform this action",
        ));
    }
    Ok(squad)
}

fn ensure_squad_not_restricted_for_member_actions(squad: &SquadModel) -> HttpResult<()> {
    if squad.is_restricted {
        return Err(HttpError::forbidden(
            "Restricted squads cannot be managed by members. Only leave or delete is allowed",
        ));
    }
    Ok(())
}

async fn get_squad_by_id(db: &impl ConnectionTrait, squad_id: &str) -> HttpResult<SquadModel> {
    let squad_uuid = parse_uuid(squad_id, "Invalid squad ID")?;
    Squad::find_by_id(squad_uuid)
        .one(db)
        .await
        .map_err(|e| HttpError::internal_error(format!("Failed to load squad: {e}")))?
        .ok_or_else(|| HttpError::not_found("Squad not found"))
}

async fn squad_member_count(db: &impl ConnectionTrait, squad_id: Uuid) -> HttpResult<u64> {
    User::find()
        .filter(UserColumn::SquadId.eq(squad_id))
        .count(db)
        .await
        .map_err(|e| HttpError::internal_error(format!("Failed to count squad members: {e}")))
}

async fn load_squad_members(
    db: &impl ConnectionTrait,
    squad_id: Uuid,
) -> HttpResult<Vec<UserModel>> {
    User::find()
        .filter(UserColumn::SquadId.eq(squad_id))
        .order_by_asc(UserColumn::CreatedAt)
        .all(db)
        .await
        .map_err(|e| HttpError::internal_error(format!("Failed to load squad members: {e}")))
}

async fn load_squad_pending_invites(
    db: &impl ConnectionTrait,
    squad_id: Uuid,
) -> HttpResult<Vec<(SquadInviteModel, UserModel)>> {
    let invites = SquadInvite::find()
        .filter(SquadInviteColumn::SquadId.eq(squad_id))
        .filter(SquadInviteColumn::ExpiresAt.gt(chrono::Utc::now()))
        .order_by_asc(SquadInviteColumn::CreatedAt)
        .all(db)
        .await
        .map_err(|e| HttpError::internal_error(format!("Failed to load squad invites: {e}")))?;

    if invites.is_empty() {
        return Ok(Vec::new());
    }

    let invited_user_ids: Vec<Uuid> = invites
        .iter()
        .map(|invite| invite.invited_user_id)
        .collect();
    let invited_users = User::find()
        .filter(UserColumn::Id.is_in(invited_user_ids))
        .all(db)
        .await
        .map_err(|e| HttpError::internal_error(format!("Failed to load invited users: {e}")))?;
    let invited_users_by_id: HashMap<Uuid, UserModel> = invited_users
        .into_iter()
        .map(|user| (user.id, user))
        .collect();

    let mut result = Vec::with_capacity(invites.len());
    for invite in invites {
        if let Some(user) = invited_users_by_id.get(&invite.invited_user_id) {
            if user.squad_id.is_none() {
                result.push((invite, user.clone()));
            }
        }
    }

    Ok(result)
}

async fn cleanup_expired_invites(db: &impl ConnectionTrait, squad_id: Uuid) -> HttpResult<()> {
    SquadInvite::delete_many()
        .filter(SquadInviteColumn::SquadId.eq(squad_id))
        .filter(SquadInviteColumn::ExpiresAt.lte(chrono::Utc::now()))
        .exec(db)
        .await
        .map_err(|e| {
            HttpError::internal_error(format!("Failed to cleanup expired invites: {e}"))
        })?;
    Ok(())
}

async fn get_owned_invite(
    db: &impl ConnectionTrait,
    invite_id: &str,
    user_id: Uuid,
) -> HttpResult<SquadInviteModel> {
    let invite_uuid = parse_uuid(invite_id, "Invalid invite ID")?;
    let invite = SquadInvite::find_by_id(invite_uuid)
        .one(db)
        .await
        .map_err(|e| HttpError::internal_error(format!("Failed to load invite: {e}")))?
        .ok_or_else(|| HttpError::not_found("Invite not found"))?;
    if invite.invited_user_id != user_id {
        return Err(HttpError::forbidden(
            "Invite does not belong to current user",
        ));
    }
    Ok(invite)
}

fn parse_uuid(input: &str, message: &str) -> HttpResult<Uuid> {
    Uuid::parse_str(input).map_err(|_| HttpError::bad_request(message))
}

async fn read_first_image(multipart: &mut Multipart) -> HttpResult<Bytes> {
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| HttpError::bad_request(format!("Invalid multipart body: {e}")))?
    {
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
