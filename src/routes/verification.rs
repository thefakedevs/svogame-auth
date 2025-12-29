use axum::extract::{Query, State};
use axum::Json;
use sea_orm::EntityTrait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::entities::User;
use crate::misc::{HttpError, HttpResult};
use crate::routes::AxumAppState;
use crate::services::token::verify_token;

#[derive(Deserialize)]
pub struct VerifyQuery {
    pub token: String,
}

#[derive(Serialize)]
pub struct VerifyResponse {
    pub id: String,
    pub username: String,
    #[serde(rename = "avatarUrl")]
    pub avatar_url: Option<String>,
    pub email: Option<String>,
    #[serde(rename = "isActive")]
    pub is_active: bool,
}

pub async fn verify(
    State(state): AxumAppState,
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
    
    Ok(Json(VerifyResponse {
        id: user.id.to_string(),
        username: user.username,
        avatar_url: user.avatar_url,
        email: user.email,
        is_active: user.is_active,
    }))
}