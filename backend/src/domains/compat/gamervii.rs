use axum::Json;
use axum::extract::State;
use sea_orm::EntityTrait;
use serde::{Deserialize, Serialize};
use tracing::{error, info};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::app::state::AppStateExtractor;
use crate::entities::User;
use crate::services::token::verify_token;

#[derive(Deserialize, ToSchema)]
pub struct GamerViiAuthRequest {
    #[serde(rename = "Login")]
    pub login: String,
    #[serde(rename = "Password")]
    pub password: String,
}

#[derive(Serialize, ToSchema)]
pub struct GamerViiAuthResponse {
    #[serde(rename = "Login")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub login: Option<String>,
    #[serde(rename = "UserUuid")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_uuid: Option<String>,
    #[serde(rename = "IsSlim")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_slim: Option<bool>,
    #[serde(rename = "Message")]
    pub message: String,
}

impl GamerViiAuthResponse {
    pub fn success(login: String, user_uuid: String) -> Self {
        Self {
            login: Some(login),
            user_uuid: Some(user_uuid),
            is_slim: Some(false),
            message: "Успешная авторизация".to_string(),
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            login: None,
            user_uuid: None,
            is_slim: None,
            message: message.into(),
        }
    }
}

#[utoipa::path(
    post,
    path = "/api/compat/gamervii/auth",
    request_body(
        content = GamerViiAuthRequest,
        description = "Compatibility login endpoint for GamerVii clients. `Login` must contain the user UUID and `Password` must contain the JWT issued by this auth service."
    ),
    responses(
        (status = 200, description = "Compatibility auth response. Failures are encoded in response body message for client compatibility.", body = GamerViiAuthResponse)
    ),
    tag = "compat"
)]
pub async fn gamervii_auth(
    State(state): AppStateExtractor,
    Json(body): Json<GamerViiAuthRequest>,
) -> Json<GamerViiAuthResponse> {
    let state = state.read().await;

    let user_id = match Uuid::parse_str(&body.login) {
        Ok(id) => id,
        Err(_) => {
            info!("Invalid UUID format in GamerVii auth login: {}", body.login);
            return Json(GamerViiAuthResponse::error(
                "Неверный формат логина (ожидается UUID)",
            ));
        }
    };

    let jwt_content = match verify_token(&body.password, &state.config) {
        Ok(content) => content,
        Err(_) => {
            info!(
                "Invalid or expired token in GamerVii auth for user_id: {}",
                user_id
            );
            return Json(GamerViiAuthResponse::error("Неверный или истёкший токен"));
        }
    };

    let token_user_id = match Uuid::parse_str(&jwt_content.user_id) {
        Ok(id) => id,
        Err(_) => {
            info!(
                "Invalid UUID format in token user_id: {}",
                jwt_content.user_id
            );
            return Json(GamerViiAuthResponse::error(
                "Неверный формат user_id в токене",
            ));
        }
    };

    if user_id != token_user_id {
        return Json(GamerViiAuthResponse::error("Логин не соответствует токену"));
    }

    let user = match User::find_by_id(user_id).one(&state.db).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            info!("User not found in GamerVii auth: {}", user_id);
            return Json(GamerViiAuthResponse::error("Пользователь не найден"));
        }
        Err(_) => {
            error!(
                "Database error while fetching user in GamerVii auth: {}",
                user_id
            );
            return Json(GamerViiAuthResponse::error("Ошибка базы данных"));
        }
    };

    if !user.is_active {
        info!("Deactivated user attempted GamerVii auth: {}", user_id);
        return Json(GamerViiAuthResponse::error("Пользователь деактивирован"));
    }

    Json(GamerViiAuthResponse::success(
        user.username,
        user.id.to_string(),
    ))
}
