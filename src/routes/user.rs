use axum::extract::State;
use axum::http::HeaderMap;
use axum::Json;
use sea_orm::{ActiveModelTrait, EntityTrait, Set};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use utoipa::ToSchema;

use crate::entities::{User, UserModel};
use crate::misc::{HttpError, HttpResult};
use crate::routes::AxumAppState;
use crate::services::token::verify_token;

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
            last_login_at: user.last_login_at,
            created_at: user.created_at,
        }
    }
}

#[derive(Deserialize, ToSchema)]
pub struct UpdateNicknameRequest {
    pub nickname: String,
}

async fn get_user_from_header(headers: HeaderMap, state: &crate::state::AppState) -> HttpResult<UserModel> {
    let auth_header = headers
        .get("Authorization")
        .ok_or_else(|| HttpError::unauthorized("Missing Authorization header"))?
        .to_str()
        .map_err(|_| HttpError::unauthorized("Invalid Authorization header"))?;

    if !auth_header.starts_with("Bearer ") {
        return Err(HttpError::unauthorized("Invalid Authorization header format"));
    }

    let token = &auth_header[7..];
    let jwt_content = verify_token(token, &state.config)
        .map_err(|_| HttpError::forbidden("Invalid or expired token"))?;

    let user_id = Uuid::parse_str(&jwt_content.user_id)
        .map_err(|_| HttpError::bad_request("Invalid user ID in token"))?;

    let user = User::find_by_id(user_id)
        .one(&state.db)
        .await
        .map_err(|_| HttpError::internal_error("Database error"))?
        .ok_or_else(|| HttpError::bad_request("User not found"))?;

    if !user.is_active {
        return Err(HttpError::forbidden("User is not active"));
    }

    Ok(user)
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
    )
)]
pub async fn get_me(
    State(state): AxumAppState,
    headers: HeaderMap,
) -> HttpResult<Json<UserResponse>> {
    let state = state.read().await;
    let user = get_user_from_header(headers, &state).await?;
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
    )
)]
pub async fn update_nickname(
    State(state): AxumAppState,
    headers: HeaderMap,
    Json(body): Json<UpdateNicknameRequest>,
) -> HttpResult<Json<UserResponse>> {
    let state_guard = state.read().await;
    let user = get_user_from_header(headers, &state_guard).await?;

    if body.nickname.trim().is_empty() {
        return Err(HttpError::bad_request("Nickname cannot be empty"));
    }

    let nickname_regex = regex::Regex::new(r"^[a-zA-Z0-9_]{2,16}$").unwrap();
    if !nickname_regex.is_match(&body.nickname) {
        return Err(HttpError::bad_request("Nickname must be 2-16 characters long and contain only letters, numbers, and underscores"));
    }

    let mut active_user: crate::entities::UserActiveModel = user.into();
    active_user.username = Set(body.nickname);

    let updated_user = active_user.update(&state_guard.db).await
        .map_err(|e| HttpError::internal_error(format!("Failed to update user: {}", e)))?;

    Ok(Json(updated_user.into()))
}
