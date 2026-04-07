use axum::http::HeaderMap;
use sea_orm::EntityTrait;
use uuid::Uuid;

use crate::app::http::{HttpError, HttpResult};
use crate::app::state::AppState;
use crate::entities::{User, UserModel};
use crate::services::token::verify_token;

pub async fn get_user_from_headers(headers: &HeaderMap, state: &AppState) -> HttpResult<UserModel> {
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
