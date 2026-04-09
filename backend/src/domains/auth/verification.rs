use axum::Json;
use axum::extract::{Query, State};
use sea_orm::EntityTrait;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::app::http::{HttpError, HttpResult};
use crate::app::state::AppStateExtractor;
use crate::entities::User;
use crate::services::token::verify_token;

#[derive(Deserialize, IntoParams, ToSchema)]
pub struct VerifyQuery {
    pub token: String,
}

#[derive(Serialize, ToSchema)]
pub struct VerifyResponse {
    pub id: String,
    pub username: String,
    #[serde(rename = "avatarUrl")]
    pub avatar_url: Option<String>,
    pub email: Option<String>,
    #[serde(rename = "isActive")]
    pub is_active: bool,
    #[serde(rename = "isSuperuser")]
    pub is_superuser: bool,
}

#[utoipa::path(
    get,
    path = "/api/auth/verify",
    params(VerifyQuery),
    responses(
        (status = 200, description = "Verify a JWT and return current user snapshot when token is valid and user is active.", body = VerifyResponse),
        (status = 400, description = "Token payload contains invalid user ID or user record is missing."),
        (status = 403, description = "Token invalid, expired, revoked, or user inactive."),
        (status = 500, description = "Database error while loading user.")
    ),
    tag = "auth"
)]
pub async fn verify(
    State(state): AppStateExtractor,
    Query(query): Query<VerifyQuery>,
) -> HttpResult<Json<VerifyResponse>> {
    let state = state.read().await;

    let jwt_content = verify_token(&query.token, &state.config)
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

    if user.auth_epoch != jwt_content.auth_epoch {
        return Err(HttpError::forbidden("Token has been revoked"));
    }

    Ok(Json(VerifyResponse {
        id: user.id.to_string(),
        username: user.username,
        avatar_url: user.avatar_url,
        email: user.email,
        is_active: user.is_active,
        is_superuser: user.is_superuser,
    }))
}
