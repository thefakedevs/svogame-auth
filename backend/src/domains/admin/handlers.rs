use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, Condition, ConnectionTrait, DatabaseTransaction,
    EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set, TransactionTrait,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::app::auth::require_human_superuser;
use crate::app::http::{HttpError, HttpResult};
use crate::app::state::{AppState, AppStateExtractor};
use crate::entities::{
    NICKNAME_REGEX, Squad, SquadModel, User, UserActiveModel, UserColumn, UserModel,
    UserRestriction, UserRestrictionActiveModel, UserRestrictionColumn,
};
use crate::services::audit::{
    ACTION_ADMIN_USER_ACTIVATED, ACTION_ADMIN_USER_AUTH_EPOCH_RESET, ACTION_ADMIN_USER_DEACTIVATED,
    ACTION_ADMIN_USER_RESTRICTION_GRANTED, ACTION_ADMIN_USER_RESTRICTION_REVOKED,
    ACTION_ADMIN_USER_SKIN_DELETED, ACTION_ADMIN_USER_SUPERUSER_GRANTED,
    ACTION_ADMIN_USER_SUPERUSER_REVOKED, ACTION_ADMIN_USER_UPDATED, write_audit_log,
};
use crate::services::restrictions::{RestrictionKind, list_user_restrictions};
use crate::services::squads::SQUAD_MAX_MEMBERS;

const DEFAULT_PAGE: u64 = 1;
const DEFAULT_PER_PAGE: u64 = 20;
const MAX_PER_PAGE: u64 = 100;

#[derive(Serialize, ToSchema)]
pub struct AdminHealthResponse {
    pub status: &'static str,
}

#[derive(Serialize, ToSchema)]
pub struct AdminMeResponse {
    pub id: String,
    pub username: String,
    #[serde(rename = "isSuperuser")]
    pub is_superuser: bool,
}

#[derive(Serialize, ToSchema)]
pub struct AdminUserResponse {
    pub id: String,
    #[serde(rename = "discordId")]
    pub discord_id: String,
    pub username: String,
    #[serde(rename = "avatarUrl")]
    pub avatar_url: Option<String>,
    pub email: Option<String>,
    #[serde(rename = "authEpoch")]
    pub auth_epoch: i32,
    #[serde(rename = "isActive")]
    pub is_active: bool,
    #[serde(rename = "isSuperuser")]
    pub is_superuser: bool,
    #[serde(rename = "deactivationReason")]
    pub deactivation_reason: Option<String>,
    #[serde(rename = "lastLoginAt")]
    pub last_login_at: chrono::DateTime<chrono::Utc>,
    #[serde(rename = "createdAt")]
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl From<UserModel> for AdminUserResponse {
    fn from(user: UserModel) -> Self {
        Self {
            id: user.id.to_string(),
            discord_id: user.discord_id,
            username: user.username,
            avatar_url: user.avatar_url,
            email: user.email,
            auth_epoch: user.auth_epoch,
            is_active: user.is_active,
            is_superuser: user.is_superuser,
            deactivation_reason: user.deactivation_reason,
            last_login_at: user.last_login_at,
            created_at: user.created_at,
        }
    }
}

#[derive(Deserialize, IntoParams)]
pub struct ListUsersQuery {
    pub q: Option<String>,
    pub page: Option<u64>,
    #[serde(rename = "perPage")]
    pub per_page: Option<u64>,
}

#[derive(Serialize, ToSchema)]
pub struct AdminUsersListResponse {
    pub items: Vec<AdminUserResponse>,
    pub total: u64,
    pub page: u64,
    #[serde(rename = "perPage")]
    pub per_page: u64,
    #[serde(rename = "totalPages")]
    pub total_pages: u64,
}

#[derive(Deserialize, ToSchema)]
pub struct PatchAdminUserRequest {
    pub username: Option<String>,
    #[serde(rename = "avatarUrl")]
    pub avatar_url: Option<String>,
    pub email: Option<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct DeactivateAdminUserRequest {
    pub reason: String,
}

#[derive(Deserialize, ToSchema)]
pub struct RestrictionReasonRequest {
    pub reason: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct AdminUserRestrictionResponse {
    pub key: String,
    pub reason: Option<String>,
    #[serde(rename = "createdAt")]
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Serialize, ToSchema)]
pub struct AdminUserSkinActionResponse {
    pub status: &'static str,
}

#[utoipa::path(
    get,
    path = "/api/admin/health",
    responses(
        (status = 200, description = "Admin domain health", body = AdminHealthResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "admin"
)]
pub async fn health(
    State(state): AppStateExtractor,
    headers: HeaderMap,
) -> HttpResult<Json<AdminHealthResponse>> {
    let state = state.read().await;
    require_human_superuser(&headers, &state).await?;
    Ok(Json(AdminHealthResponse { status: "ok" }))
}

#[utoipa::path(
    get,
    path = "/api/admin/me",
    responses(
        (status = 200, description = "Current admin info", body = AdminMeResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "admin"
)]
pub async fn me(
    State(state): AppStateExtractor,
    headers: HeaderMap,
) -> HttpResult<Json<AdminMeResponse>> {
    let state = state.read().await;
    let user = require_human_superuser(&headers, &state).await?;

    Ok(Json(AdminMeResponse {
        id: user.id.to_string(),
        username: user.username,
        is_superuser: user.is_superuser,
    }))
}

#[utoipa::path(
    get,
    path = "/api/admin/users",
    params(ListUsersQuery),
    responses(
        (status = 200, description = "List users", body = AdminUsersListResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "admin"
)]
pub async fn list_users(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Query(query): Query<ListUsersQuery>,
) -> HttpResult<Json<AdminUsersListResponse>> {
    let state = state.read().await;
    require_human_superuser(&headers, &state).await?;

    let page = query.page.unwrap_or(DEFAULT_PAGE).max(1);
    let per_page = query
        .per_page
        .unwrap_or(DEFAULT_PER_PAGE)
        .clamp(1, MAX_PER_PAGE);
    let mut user_query = User::find().order_by_desc(UserColumn::CreatedAt);

    if let Some(q) = query
        .q
        .as_ref()
        .map(|it| it.trim())
        .filter(|it| !it.is_empty())
    {
        let mut condition = Condition::any().add(UserColumn::Username.contains(q));
        if let Ok(uuid) = Uuid::parse_str(q) {
            condition = condition.add(UserColumn::Id.eq(uuid));
        }
        user_query = user_query.filter(condition);
    }

    let paginator = user_query.paginate(&state.db, per_page);
    let total = paginator
        .num_items()
        .await
        .map_err(|e| HttpError::internal_error(format!("Failed to count users: {e}")))?;
    let users = paginator
        .fetch_page(page.saturating_sub(1))
        .await
        .map_err(|e| HttpError::internal_error(format!("Failed to fetch users: {e}")))?;
    let total_pages = if total == 0 {
        0
    } else {
        total.div_ceil(per_page)
    };

    Ok(Json(AdminUsersListResponse {
        items: users.into_iter().map(Into::into).collect(),
        total,
        page,
        per_page,
        total_pages,
    }))
}

#[utoipa::path(
    get,
    path = "/api/admin/users/{user_id}",
    params(
        ("user_id" = String, Path, description = "User UUID")
    ),
    responses(
        (status = 200, description = "Get user details", body = AdminUserResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "User not found")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "admin"
)]
pub async fn get_user(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Path(user_id): Path<String>,
) -> HttpResult<Json<AdminUserResponse>> {
    let state = state.read().await;
    require_human_superuser(&headers, &state).await?;
    let user = get_user_by_id(&state.db, &user_id).await?;
    Ok(Json(user.into()))
}

#[utoipa::path(
    get,
    path = "/api/admin/user/{user_id}/squad",
    params(
        ("user_id" = String, Path, description = "User UUID")
    ),
    responses(
        (status = 200, description = "Get the squad the target user currently belongs to, or `null` when user has no squad.", body = Option<crate::domains::admin::squads::AdminSquadResponse>),
        (status = 400, description = "Invalid user ID."),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "User not found")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "admin"
)]
pub async fn get_user_squad(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Path(user_id): Path<String>,
) -> HttpResult<Json<Option<crate::domains::admin::squads::AdminSquadResponse>>> {
    let state = state.read().await;
    require_human_superuser(&headers, &state).await?;
    let user = get_user_by_id(&state.db, &user_id).await?;

    let Some(squad_id) = user.squad_id else {
        return Ok(Json(None));
    };

    let squad = Squad::find_by_id(squad_id)
        .one(&state.db)
        .await
        .map_err(|e| HttpError::internal_error(format!("Failed to load squad: {e}")))?
        .ok_or_else(|| HttpError::not_found("Squad not found"))?;

    Ok(Json(Some(to_admin_squad_response(&state.db, squad).await?)))
}

#[utoipa::path(
    patch,
    path = "/api/admin/users/{user_id}",
    request_body = PatchAdminUserRequest,
    params(
        ("user_id" = String, Path, description = "User UUID")
    ),
    responses(
        (status = 200, description = "Patch user", body = AdminUserResponse),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "User not found")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "admin"
)]
pub async fn patch_user(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Path(user_id): Path<String>,
    Json(body): Json<PatchAdminUserRequest>,
) -> HttpResult<Json<AdminUserResponse>> {
    let state = state.read().await;
    let admin = require_human_superuser(&headers, &state).await?;
    let user_id = parse_user_id(&user_id)?;

    let tx = state
        .db
        .begin()
        .await
        .map_err(|e| HttpError::internal_error(format!("Failed to start transaction: {e}")))?;

    let user = get_user_by_uuid(&tx, user_id).await?;
    let mut active_user: UserActiveModel = user.clone().into();
    let mut changed_fields = Vec::new();

    if let Some(username) = body.username {
        validate_username(&tx, user.id, &username).await?;
        if user.username != username {
            active_user.username = Set(username);
            changed_fields.push("username");
        }
    }

    if let Some(email) = body.email {
        if user.email.as_deref() != Some(email.as_str()) {
            active_user.email = Set(Some(email));
            changed_fields.push("email");
        }
    }

    if let Some(avatar_url) = body.avatar_url {
        if user.avatar_url.as_deref() != Some(avatar_url.as_str()) {
            active_user.avatar_url = Set(Some(avatar_url));
            changed_fields.push("avatarUrl");
        }
    }

    let updated_user = if changed_fields.is_empty() {
        user
    } else {
        let updated_user = sea_orm::ActiveModelTrait::update(active_user, &tx)
            .await
            .map_err(|e| HttpError::internal_error(format!("Failed to update user: {e}")))?;

        write_audit_log(
            &tx,
            ACTION_ADMIN_USER_UPDATED,
            Some(admin.id),
            Some(updated_user.id),
            None,
            Some(json!({ "fields": changed_fields })),
        )
        .await
        .map_err(|e| HttpError::internal_error(format!("Failed to write audit log: {e}")))?;

        updated_user
    };

    tx.commit()
        .await
        .map_err(|e| HttpError::internal_error(format!("Failed to commit transaction: {e}")))?;

    Ok(Json(updated_user.into()))
}

#[utoipa::path(
    delete,
    path = "/api/admin/user/{user_id}/skin",
    params(
        ("user_id" = String, Path, description = "User UUID")
    ),
    responses(
        (status = 200, description = "Delete user's custom skin from object storage. Repeated delete is effectively idempotent.", body = AdminUserSkinActionResponse),
        (status = 400, description = "Invalid user ID."),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "User not found")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "admin"
)]
pub async fn delete_user_skin(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Path(user_id): Path<String>,
) -> HttpResult<Json<AdminUserSkinActionResponse>> {
    let state = state.read().await;
    let admin = require_human_superuser(&headers, &state).await?;
    let user = get_user_by_id(&state.db, &user_id).await?;
    let skin_key = format!("user_skins/{}.png", user.id.as_hyphenated());

    state
        .s3
        .delete_object()
        .bucket(&state.config.s3.bucket)
        .key(&skin_key)
        .send()
        .await
        .map_err(|e| HttpError::internal_error(format!("Failed to delete user skin: {e}")))?;

    write_audit_log(
        &state.db,
        ACTION_ADMIN_USER_SKIN_DELETED,
        Some(admin.id),
        Some(user.id),
        None,
        Some(json!({ "skinKey": skin_key })),
    )
    .await
    .map_err(|e| HttpError::internal_error(format!("Failed to write audit log: {e}")))?;

    Ok(Json(AdminUserSkinActionResponse { status: "ok" }))
}

#[utoipa::path(
    post,
    path = "/api/admin/users/{user_id}/deactivate",
    request_body = DeactivateAdminUserRequest,
    params(
        ("user_id" = String, Path, description = "User UUID")
    ),
    responses(
        (status = 200, description = "Deactivate user", body = AdminUserResponse),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "User not found")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "admin"
)]
pub async fn deactivate_user(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Path(user_id): Path<String>,
    Json(body): Json<DeactivateAdminUserRequest>,
) -> HttpResult<Json<AdminUserResponse>> {
    let reason = body.reason.trim().to_string();
    if reason.is_empty() {
        return Err(HttpError::bad_request("Reason is required"));
    }

    let state = state.read().await;
    let admin = require_human_superuser(&headers, &state).await?;
    let updated_user = update_user_with_audit(
        &state,
        admin.id,
        &user_id,
        ACTION_ADMIN_USER_DEACTIVATED,
        Some(reason.clone()),
        Some(json!({ "isActive": false })),
        move |active_user| {
            active_user.is_active = Set(false);
            active_user.deactivation_reason = Set(Some(reason));
        },
    )
    .await?;

    Ok(Json(updated_user.into()))
}

#[utoipa::path(
    post,
    path = "/api/admin/users/{user_id}/activate",
    params(
        ("user_id" = String, Path, description = "User UUID")
    ),
    responses(
        (status = 200, description = "Activate user", body = AdminUserResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "User not found")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "admin"
)]
pub async fn activate_user(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Path(user_id): Path<String>,
) -> HttpResult<Json<AdminUserResponse>> {
    let state = state.read().await;
    let admin = require_human_superuser(&headers, &state).await?;
    let updated_user = update_user_with_audit(
        &state,
        admin.id,
        &user_id,
        ACTION_ADMIN_USER_ACTIVATED,
        None,
        Some(json!({ "isActive": true })),
        |active_user| {
            active_user.is_active = Set(true);
            active_user.deactivation_reason = Set(None);
        },
    )
    .await?;

    Ok(Json(updated_user.into()))
}

#[utoipa::path(
    post,
    path = "/api/admin/users/{user_id}/reset-auth-epoch",
    params(
        ("user_id" = String, Path, description = "User UUID")
    ),
    responses(
        (status = 200, description = "Reset auth epoch", body = AdminUserResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "User not found")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "admin"
)]
pub async fn reset_auth_epoch(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Path(user_id): Path<String>,
) -> HttpResult<Json<AdminUserResponse>> {
    let state = state.read().await;
    let admin = require_human_superuser(&headers, &state).await?;
    let updated_user = update_user_with_audit(
        &state,
        admin.id,
        &user_id,
        ACTION_ADMIN_USER_AUTH_EPOCH_RESET,
        None,
        None,
        |active_user| {
            let next_auth_epoch = match &active_user.auth_epoch {
                ActiveValue::Set(value) => *value + 1,
                _ => 1,
            };
            active_user.auth_epoch = Set(next_auth_epoch);
        },
    )
    .await?;

    Ok(Json(updated_user.into()))
}

#[utoipa::path(
    post,
    path = "/api/admin/users/{user_id}/grant-superuser",
    params(
        ("user_id" = String, Path, description = "User UUID")
    ),
    responses(
        (status = 200, description = "Grant superuser", body = AdminUserResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "User not found")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "admin"
)]
pub async fn grant_superuser(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Path(user_id): Path<String>,
) -> HttpResult<Json<AdminUserResponse>> {
    let state = state.read().await;
    let admin = require_human_superuser(&headers, &state).await?;
    let updated_user = update_user_with_audit(
        &state,
        admin.id,
        &user_id,
        ACTION_ADMIN_USER_SUPERUSER_GRANTED,
        None,
        Some(json!({ "isSuperuser": true })),
        |active_user| {
            active_user.is_superuser = Set(true);
        },
    )
    .await?;

    Ok(Json(updated_user.into()))
}

#[utoipa::path(
    post,
    path = "/api/admin/users/{user_id}/revoke-superuser",
    params(
        ("user_id" = String, Path, description = "User UUID")
    ),
    responses(
        (status = 200, description = "Revoke superuser", body = AdminUserResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "User not found")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "admin"
)]
pub async fn revoke_superuser(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Path(user_id): Path<String>,
) -> HttpResult<Json<AdminUserResponse>> {
    let state = state.read().await;
    let admin = require_human_superuser(&headers, &state).await?;
    let updated_user = update_user_with_audit(
        &state,
        admin.id,
        &user_id,
        ACTION_ADMIN_USER_SUPERUSER_REVOKED,
        None,
        Some(json!({ "isSuperuser": false })),
        |active_user| {
            active_user.is_superuser = Set(false);
        },
    )
    .await?;

    Ok(Json(updated_user.into()))
}

async fn update_user_with_audit<F>(
    state: &AppState,
    actor_user_id: Uuid,
    user_id: &str,
    action: &str,
    reason: Option<String>,
    metadata: Option<Value>,
    mutate: F,
) -> HttpResult<UserModel>
where
    F: FnOnce(&mut UserActiveModel),
{
    let user_id = parse_user_id(user_id)?;
    let tx = state
        .db
        .begin()
        .await
        .map_err(|e| HttpError::internal_error(format!("Failed to start transaction: {e}")))?;

    let user = get_user_by_uuid(&tx, user_id).await?;
    let mut active_user: UserActiveModel = user.into();
    mutate(&mut active_user);

    let updated_user = sea_orm::ActiveModelTrait::update(active_user, &tx)
        .await
        .map_err(|e| HttpError::internal_error(format!("Failed to update user: {e}")))?;

    write_audit_log(
        &tx,
        action,
        Some(actor_user_id),
        Some(updated_user.id),
        reason,
        metadata,
    )
    .await
    .map_err(|e| HttpError::internal_error(format!("Failed to write audit log: {e}")))?;

    tx.commit()
        .await
        .map_err(|e| HttpError::internal_error(format!("Failed to commit transaction: {e}")))?;

    Ok(updated_user)
}

fn parse_user_id(user_id: &str) -> HttpResult<Uuid> {
    Uuid::parse_str(user_id).map_err(|_| HttpError::bad_request("Invalid user ID"))
}

async fn get_user_by_id(db: &impl ConnectionTrait, user_id: &str) -> HttpResult<UserModel> {
    get_user_by_uuid(db, parse_user_id(user_id)?).await
}

async fn get_user_by_uuid(db: &impl ConnectionTrait, user_id: Uuid) -> HttpResult<UserModel> {
    User::find_by_id(user_id)
        .one(db)
        .await
        .map_err(|e| HttpError::internal_error(format!("Database error: {e}")))?
        .ok_or_else(|| HttpError::not_found("User not found"))
}

async fn to_admin_squad_response(
    db: &impl ConnectionTrait,
    squad: SquadModel,
) -> HttpResult<crate::domains::admin::squads::AdminSquadResponse> {
    let member_count = User::find()
        .filter(UserColumn::SquadId.eq(squad.id))
        .count(db)
        .await
        .map_err(|e| HttpError::internal_error(format!("Failed to count squad members: {e}")))?;

    let squad_id = squad.id;
    Ok(crate::domains::admin::squads::AdminSquadResponse {
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
    })
}

async fn validate_username(
    db: &DatabaseTransaction,
    current_user_id: Uuid,
    username: &str,
) -> HttpResult<()> {
    if username.trim().is_empty() {
        return Err(HttpError::bad_request("Username cannot be empty"));
    }

    let nickname_regex = regex::Regex::new(NICKNAME_REGEX).unwrap();
    if !nickname_regex.is_match(username) {
        return Err(HttpError::bad_request(
            "Username must be 3-16 characters long and contain only letters, numbers, and underscores",
        ));
    }

    let existing_user = User::find()
        .filter(UserColumn::Username.eq(username))
        .one(db)
        .await
        .map_err(|e| HttpError::internal_error(format!("Failed to validate username: {e}")))?;

    if let Some(existing_user) = existing_user {
        if existing_user.id != current_user_id {
            return Err(HttpError::bad_request("Username is already taken"));
        }
    }

    Ok(())
}

#[utoipa::path(
    get,
    path = "/api/admin/users/{user_id}/restrictions",
    params(
        ("user_id" = String, Path, description = "User UUID")
    ),
    responses(
        (status = 200, description = "List active restrictions for a user.", body = [AdminUserRestrictionResponse]),
        (status = 400, description = "Invalid user ID."),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    ),
    security(("bearer_auth" = [])),
    tag = "admin"
)]
pub async fn get_user_restrictions(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Path(user_id): Path<String>,
) -> HttpResult<Json<Vec<AdminUserRestrictionResponse>>> {
    let state = state.read().await;
    require_human_superuser(&headers, &state).await?;
    let user_id = parse_user_id(&user_id)?;

    let restrictions = list_user_restrictions(&state.db, user_id)
        .await
        .map_err(|e| HttpError::internal_error(format!("Failed to load restrictions: {e}")))?;

    Ok(Json(
        restrictions
            .into_iter()
            .map(|restriction| AdminUserRestrictionResponse {
                key: restriction.restriction_key,
                reason: restriction.reason,
                created_at: restriction.created_at,
            })
            .collect(),
    ))
}

#[utoipa::path(
    post,
    path = "/api/admin/users/{user_id}/restrictions/{restriction_key}",
    params(
        ("user_id" = String, Path, description = "User UUID"),
        ("restriction_key" = String, Path, description = "Restriction key")
    ),
    request_body = RestrictionReasonRequest,
    responses(
        (status = 200, description = "Grant restriction to a user. Repeated grant is effectively idempotent.", body = [AdminUserRestrictionResponse]),
        (status = 400, description = "Invalid user ID or restriction key."),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    ),
    security(("bearer_auth" = [])),
    tag = "admin"
)]
pub async fn grant_user_restriction(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Path((user_id, restriction_key)): Path<(String, String)>,
    Json(body): Json<RestrictionReasonRequest>,
) -> HttpResult<Json<Vec<AdminUserRestrictionResponse>>> {
    let state = state.read().await;
    let admin = require_human_superuser(&headers, &state).await?;
    let user_id = parse_user_id(&user_id)?;
    let restriction = restriction_key
        .parse::<RestrictionKind>()
        .map_err(|e| HttpError::bad_request(e.to_string()))?;

    let existing = UserRestriction::find()
        .filter(UserRestrictionColumn::UserId.eq(user_id))
        .filter(UserRestrictionColumn::RestrictionKey.eq(restriction.as_str()))
        .one(&state.db)
        .await
        .map_err(|e| HttpError::internal_error(format!("Failed to check restriction: {e}")))?;

    if existing.is_none() {
        let active_model = UserRestrictionActiveModel {
            id: ActiveValue::NotSet,
            user_id: Set(user_id),
            restriction_key: Set(restriction.as_str().to_string()),
            reason: Set(body.reason.clone()),
            created_at: Set(chrono::Utc::now()),
        };
        active_model
            .insert(&state.db)
            .await
            .map_err(|e| HttpError::internal_error(format!("Failed to grant restriction: {e}")))?;

        write_audit_log(
            &state.db,
            ACTION_ADMIN_USER_RESTRICTION_GRANTED,
            Some(admin.id),
            Some(user_id),
            body.reason,
            Some(json!({ "restrictionKey": restriction.as_str() })),
        )
        .await
        .map_err(|e| HttpError::internal_error(format!("Failed to write audit log: {e}")))?;
    }

    let restrictions = list_user_restrictions(&state.db, user_id)
        .await
        .map_err(|e| HttpError::internal_error(format!("Failed to load restrictions: {e}")))?;

    Ok(Json(
        restrictions
            .into_iter()
            .map(|restriction| AdminUserRestrictionResponse {
                key: restriction.restriction_key,
                reason: restriction.reason,
                created_at: restriction.created_at,
            })
            .collect(),
    ))
}

#[utoipa::path(
    delete,
    path = "/api/admin/users/{user_id}/restrictions/{restriction_key}",
    params(
        ("user_id" = String, Path, description = "User UUID"),
        ("restriction_key" = String, Path, description = "Restriction key")
    ),
    request_body = RestrictionReasonRequest,
    responses(
        (status = 200, description = "Revoke restriction from a user.", body = [AdminUserRestrictionResponse]),
        (status = 400, description = "Invalid user ID or restriction key."),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    ),
    security(("bearer_auth" = [])),
    tag = "admin"
)]
pub async fn revoke_user_restriction(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Path((user_id, restriction_key)): Path<(String, String)>,
    Json(body): Json<RestrictionReasonRequest>,
) -> HttpResult<Json<Vec<AdminUserRestrictionResponse>>> {
    let state = state.read().await;
    let admin = require_human_superuser(&headers, &state).await?;
    let user_id = parse_user_id(&user_id)?;
    let restriction = restriction_key
        .parse::<RestrictionKind>()
        .map_err(|e| HttpError::bad_request(e.to_string()))?;

    UserRestriction::delete_many()
        .filter(UserRestrictionColumn::UserId.eq(user_id))
        .filter(UserRestrictionColumn::RestrictionKey.eq(restriction.as_str()))
        .exec(&state.db)
        .await
        .map_err(|e| HttpError::internal_error(format!("Failed to revoke restriction: {e}")))?;

    write_audit_log(
        &state.db,
        ACTION_ADMIN_USER_RESTRICTION_REVOKED,
        Some(admin.id),
        Some(user_id),
        body.reason,
        Some(json!({ "restrictionKey": restriction.as_str() })),
    )
    .await
    .map_err(|e| HttpError::internal_error(format!("Failed to write audit log: {e}")))?;

    let restrictions = list_user_restrictions(&state.db, user_id)
        .await
        .map_err(|e| HttpError::internal_error(format!("Failed to load restrictions: {e}")))?;

    Ok(Json(
        restrictions
            .into_iter()
            .map(|restriction| AdminUserRestrictionResponse {
                key: restriction.restriction_key,
                reason: restriction.reason,
                created_at: restriction.created_at,
            })
            .collect(),
    ))
}
