use axum::extract::{Query, State};
use axum::http::HeaderMap;
use axum::Json;
use sea_orm::sea_query::{Expr, Func};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect, Set};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::app::auth::get_user_from_headers;
use crate::app::http::{HttpError, HttpResult};
use crate::app::state::AppStateExtractor;
use crate::entities::{User, UserColumn, UserModel, NICKNAME_REGEX};
use crate::services::restrictions::list_user_restrictions;

const DEFAULT_SEARCH_LIMIT: u64 = 10;
const MAX_SEARCH_LIMIT: u64 = 25;
const MIN_SEARCH_QUERY_LEN: usize = 3;

#[derive(Serialize, ToSchema)]
pub struct UserResponse {
    pub id: String,
    #[serde(rename = "discordId")]
    pub discord_id: String,
    pub username: String,
    #[serde(rename = "avatarUrl")]
    pub avatar_url: Option<String>,
    pub email: Option<String>,
    #[serde(rename = "isActive")]
    pub is_active: bool,
    #[serde(rename = "isSuperuser")]
    pub is_superuser: bool,
    #[serde(rename = "lastLoginAt")]
    pub last_login_at: chrono::DateTime<chrono::Utc>,
    #[serde(rename = "createdAt")]
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl From<UserModel> for UserResponse {
    fn from(user: UserModel) -> Self {
        Self {
            id: user.id.to_string(),
            discord_id: user.discord_id,
            username: user.username,
            avatar_url: user.avatar_url,
            email: user.email,
            is_active: user.is_active,
            is_superuser: user.is_superuser,
            last_login_at: user.last_login_at,
            created_at: user.created_at,
        }
    }
}

#[derive(Serialize, ToSchema)]
pub struct UserRestrictionResponse {
    pub key: String,
    pub reason: Option<String>,
    #[serde(rename = "createdAt")]
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Deserialize, ToSchema)]
pub struct UpdateNicknameRequest {
    pub nickname: String,
}

#[derive(Deserialize, IntoParams)]
pub struct SearchUsersQuery {
    pub q: String,
    pub limit: Option<u64>,
}

#[derive(Serialize, ToSchema)]
pub struct UserSearchItemResponse {
    pub id: String,
    pub username: String,
    #[serde(rename = "avatarUrl")]
    pub avatar_url: Option<String>,
}

#[utoipa::path(
    get,
    path = "/api/user/me",
    responses(
        (status = 200, description = "Get current user info", body = UserResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "users"
)]
pub async fn get_me(
    State(state): AppStateExtractor,
    headers: HeaderMap,
) -> HttpResult<Json<UserResponse>> {
    let state = state.read().await;
    let user = get_user_from_headers(&headers, &state).await?;
    Ok(Json(user.into()))
}

#[utoipa::path(
    post,
    path = "/api/user/me/nickname",
    request_body = UpdateNicknameRequest,
    responses(
        (status = 200, description = "Update user nickname", body = UserResponse),
        (status = 400, description = "Bad Request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "users"
)]
pub async fn update_nickname(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Json(body): Json<UpdateNicknameRequest>,
) -> HttpResult<Json<UserResponse>> {
    let state_guard = state.read().await;
    let user = get_user_from_headers(&headers, &state_guard).await?;

    if body.nickname.trim().is_empty() {
        return Err(HttpError::bad_request("Nickname cannot be empty"));
    }

    let nickname_regex = regex::Regex::new(NICKNAME_REGEX).unwrap();
    if !nickname_regex.is_match(&body.nickname) {
        return Err(HttpError::bad_request("Nickname must be 2-16 characters long and contain only letters, numbers, and underscores"));
    }

    let existing_user = User::find()
        .filter(crate::entities::UserColumn::Username.eq(&body.nickname))
        .one(&state_guard.db)
        .await
        .map_err(|e| HttpError::internal_error(format!("Database error: {}", e)))?;

    if let Some(existing) = existing_user {
        if existing.id != user.id {
            return Err(HttpError::bad_request("Nickname is already taken"));
        }
    }

    if let Some(gml_config) = &state_guard.config.gamervii_compat {
        crate::services::gml::remove_player_from_gml(gml_config, &user.id.to_string())
            .await
            .map_err(|e| HttpError::internal_error(format!("Failed to sync with GML: {}", e)))?;
    }

    let mut active_user: crate::entities::UserActiveModel = user.into();
    active_user.username = Set(body.nickname);

    let updated_user = active_user.update(&state_guard.db).await
        .map_err(|e| HttpError::internal_error(format!("Failed to update user: {}", e)))?;

    Ok(Json(updated_user.into()))
}

#[utoipa::path(
    get,
    path = "/api/users/search",
    params(SearchUsersQuery),
    responses(
        (status = 200, description = "Search users by username for autocomplete. Query must contain at least 3 characters. Returns at most `limit` matches, default 10.", body = [UserSearchItemResponse]),
        (status = 400, description = "Search query is too short or invalid."),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "users"
)]
pub async fn search_users(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Query(query): Query<SearchUsersQuery>,
) -> HttpResult<Json<Vec<UserSearchItemResponse>>> {
    let state = state.read().await;
    get_user_from_headers(&headers, &state).await?;

    let trimmed_query = query.q.trim();
    if trimmed_query.chars().count() < MIN_SEARCH_QUERY_LEN {
        return Err(HttpError::bad_request("Search query must contain at least 3 characters"));
    }

    let limit = query.limit.unwrap_or(DEFAULT_SEARCH_LIMIT).clamp(1, MAX_SEARCH_LIMIT);
    let lowered_query = trimmed_query.to_lowercase();

    let users = User::find()
        .filter(UserColumn::IsActive.eq(true))
        .filter(Expr::expr(Func::lower(Expr::col(UserColumn::Username))).like(format!("%{lowered_query}%")))
        .order_by_asc(UserColumn::Username)
        .limit(limit)
        .all(&state.db)
        .await
        .map_err(|e| HttpError::internal_error(format!("Failed to search users: {e}")))?;

    Ok(Json(
        users.into_iter()
            .filter(|user| user.username.to_lowercase().contains(&lowered_query))
            .map(|user| UserSearchItemResponse {
                id: user.id.to_string(),
                username: user.username,
                avatar_url: user.avatar_url,
            })
            .collect(),
    ))
}

#[utoipa::path(
    get,
    path = "/api/user/me/restrictions",
    responses(
        (status = 200, description = "List active restrictions currently applied to the authenticated user.", body = [UserRestrictionResponse]),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "users"
)]
pub async fn get_my_restrictions(
    State(state): AppStateExtractor,
    headers: HeaderMap,
) -> HttpResult<Json<Vec<UserRestrictionResponse>>> {
    let state = state.read().await;
    let user = get_user_from_headers(&headers, &state).await?;
    let restrictions = list_user_restrictions(&state.db, user.id)
        .await
        .map_err(|e| HttpError::internal_error(format!("Failed to load restrictions: {e}")))?;

    Ok(Json(
        restrictions
            .into_iter()
            .map(|restriction| UserRestrictionResponse {
                key: restriction.restriction_key,
                reason: restriction.reason,
                created_at: restriction.created_at,
            })
            .collect(),
    ))
}
