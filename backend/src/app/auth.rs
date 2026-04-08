use axum::http::HeaderMap;
use sea_orm::EntityTrait;
use uuid::Uuid;

use crate::app::http::{HttpError, HttpResult};
use crate::app::state::AppState;
use crate::entities::{User, UserModel};
use crate::services::service_tokens::AuthenticatedServiceToken;
use crate::services::token::verify_token;

#[derive(Clone, Debug)]
pub enum AuthenticatedActor {
    User(UserModel),
    Service(AuthenticatedServiceToken),
}

#[derive(Clone, Debug)]
pub enum PrivilegedActor {
    User(UserModel),
    Service(AuthenticatedServiceToken),
}

impl PrivilegedActor {
    pub fn actor_user_id(&self) -> Option<Uuid> {
        match self {
            Self::User(user) => Some(user.id),
            Self::Service(_) => None,
        }
    }

    pub fn actor_service_name(&self) -> Option<&str> {
        match self {
            Self::User(_) => None,
            Self::Service(service) => Some(service.system_name.as_str()),
        }
    }
}

pub async fn get_user_from_headers(headers: &HeaderMap, state: &AppState) -> HttpResult<UserModel> {
    match get_actor_from_headers(headers, state).await? {
        AuthenticatedActor::User(user) => Ok(user),
        AuthenticatedActor::Service(_) => Err(HttpError::forbidden("User token required")),
    }
}

pub async fn get_actor_from_headers(
    headers: &HeaderMap,
    state: &AppState,
) -> HttpResult<AuthenticatedActor> {
    let auth_header = headers
        .get("Authorization")
        .ok_or_else(|| HttpError::unauthorized("Missing Authorization header"))?
        .to_str()
        .map_err(|_| HttpError::unauthorized("Invalid Authorization header"))?;

    if !auth_header.starts_with("Bearer ") {
        return Err(HttpError::unauthorized("Invalid Authorization header format"));
    }

    let token = &auth_header[7..];
    if let Ok(jwt_content) = verify_token(token, &state.config) {
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

        return Ok(AuthenticatedActor::User(user));
    }

    let service = crate::services::service_tokens::authenticate_service_token(&state.db, &state.config, token)
        .await
        .map_err(|_| HttpError::forbidden("Invalid or expired token"))?;
    Ok(AuthenticatedActor::Service(service))
}

pub async fn require_human_superuser(headers: &HeaderMap, state: &AppState) -> HttpResult<UserModel> {
    let user = get_user_from_headers(headers, state).await?;

    if !user.is_superuser {
        return Err(HttpError::forbidden("Superuser permissions required"));
    }

    Ok(user)
}

pub async fn require_privileged_actor(
    headers: &HeaderMap,
    state: &AppState,
) -> HttpResult<PrivilegedActor> {
    match get_actor_from_headers(headers, state).await? {
        AuthenticatedActor::User(user) => {
            if !user.is_superuser {
                return Err(HttpError::forbidden("Superuser permissions required"));
            }
            Ok(PrivilegedActor::User(user))
        }
        AuthenticatedActor::Service(service) => Ok(PrivilegedActor::Service(service)),
    }
}
