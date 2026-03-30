use crate::entities::{AuthRay, auth_ray::TokenDeliveryMethod};
use crate::misc::{HttpError, HttpResult};
use crate::routes::AxumAppState;
use axum::extract::State;
use axum::Json;
use sea_orm::ModelTrait;
use serde::{Deserialize, Serialize};
use tracing::error;
use crate::services::discord::exchange_code;
use crate::services::token::sign_token;
use crate::state::AuthPollResult;
use uuid::Uuid;

#[derive(Serialize)]
pub struct PrepareAuthResponse {
    #[serde(rename = "powPrefix")]
    pow_prefix: String,
    #[serde(rename = "powComplexity")]
    pow_complexity: u8,
    #[serde(rename = "oauthUrl")]
    oauth2_url: String,
    #[serde(rename = "deliveryMethod")]
    delivery_method: String,
    #[serde(rename = "deliveryTarget")]
    delivery_target: String,
}

#[derive(Deserialize)]
pub struct PrepareAuthRequest {
    #[serde(rename = "redirectUrl")]
    pub redirect_url: Option<String>,
    #[serde(rename = "deliveryMethod")]
    pub delivery_method: String,
}

pub async fn prepare_auth(
    State(state): AxumAppState,
    Json(body): Json<PrepareAuthRequest>,
) -> HttpResult<Json<PrepareAuthResponse>> {
    let state = &state.read().await;
    let db = &state.db;

    let delivery_method = match body.delivery_method.as_str() {
        "redirect" => TokenDeliveryMethod::Redirect,
        "polling" => TokenDeliveryMethod::Polling,
        _ => return Err(HttpError::bad_request("Invalid delivery method")),
    };
    let delivery_target = match delivery_method {
        TokenDeliveryMethod::Redirect => {
            match body.redirect_url {
                Some(url) => url,
                None => return Err(HttpError::bad_request("redirectUrl is required for redirect delivery method")),
            }
        },
        TokenDeliveryMethod::Polling => Uuid::new_v4().to_string(),
    };

    let ray = AuthRay::create_with_complexity(
        db,
        state.config.pow_complexity,
        delivery_method,
        delivery_target,
    ).await.map_err(|e| {
        error!("Failed to create auth ray: {:?}", e);
        HttpError::internal_error("Failed to create auth ray")
    })?;

    Ok(Json(PrepareAuthResponse {
        pow_prefix: ray.pow_prefix,
        pow_complexity: ray.pow_complexity,
        oauth2_url: state.config.discord.oauth2_url.to_string(),
        delivery_method: match ray.delivery_method {
            TokenDeliveryMethod::Redirect => "redirect".to_string(),
            TokenDeliveryMethod::Polling => "polling".to_string(),
        },
        delivery_target: ray.delivery_target,
    }))
}

#[derive(Serialize)]
pub struct AuthorizeResponse {
    #[serde(rename = "accessToken")]
    access_token: String,
    id: String,
    username: String,
    #[serde(rename = "avatarUrl")]
    avatar_url: String,
    #[serde(rename = "deliveryMethod")]
    delivery_method: String,
    #[serde(rename = "deliveryTarget")]
    delivery_target: String,
}

#[derive(Deserialize)]
pub struct AuthorizeRequest {
    #[serde(rename = "powPrefix")]
    pub pow_prefix: String,
    #[serde(rename = "powSolution")]
    pub pow_solution: String,
    #[serde(rename = "discordCode")]
    pub discord_code: String,
}

pub async fn authorize(
    State(state): AxumAppState,
    Json(body): Json<AuthorizeRequest>,
) -> HttpResult<Json<AuthorizeResponse>> {
    let state = &state.read().await;
    // Find the AuthRay
    let ray = AuthRay::find_by_prefix(&state.db, &body.pow_prefix)
        .await
        .map_err(|e| {
            error!("Database error while finding auth ray: {:?}", e);
            HttpError::internal_error("Database error")
        })?;
    let ray = match ray {
        Some(ray) => ray,
        None => return Err(crate::misc::HttpError::bad_request("Invalid or expired pow_prefix")),
    };

    let pow_complexity = ray.pow_complexity;
    let pow_prefix = ray.pow_prefix.clone();
    let pow_creation = ray.created_at;
    let is_polling = matches!(ray.delivery_method, TokenDeliveryMethod::Polling);
    let delivery_method = match ray.delivery_method {
        TokenDeliveryMethod::Redirect => "redirect".to_string(),
        TokenDeliveryMethod::Polling => "polling".to_string(),
    };
    let delivery_target = ray.delivery_target.clone();

    ray.delete(&state.db).await.map_err(|e| {
        error!("Failed to delete auth ray: {:?}", e);
        crate::misc::HttpError::internal_error("Failed to delete auth ray")
    })?;

    let notify_error = |msg: &str| {
        if is_polling {
            state.notify_auth_complete(
                delivery_target.clone(),
                AuthPollResult::Error { message: msg.to_string() },
            );
        }
    };

    if !crate::services::pow::verify_pow(&body.pow_solution, &pow_prefix, pow_complexity, &pow_creation) {
        notify_error("Invalid PoW solution");
        return Err(HttpError::forbidden("Invalid PoW solution"));
    }

    let discord_creds = match exchange_code(&state.config.discord, &body.discord_code).await {
        Ok(creds) => creds,
        Err(e) => {
            error!("Failed to exchange Discord code: {:?}", e);
            notify_error("Failed to exchange Discord code");
            return Err(HttpError::forbidden("Failed to exchange Discord code"));
        }
    };

    {
        let scopes = discord_creds.scope.split(" ").map(|it| it.to_string()).collect::<Vec<String>>();
        if state.config.discord.required_scopes.iter().any(|scope| !scopes.contains(scope)) {
            notify_error("Missing required Discord scopes");
            return Err(HttpError::forbidden("Missing required Discord scopes"));
        }
    }

    let user_info = match crate::services::discord::get_user_info(&state.config.discord, &discord_creds.access_token).await {
        Ok(info) => info,
        Err(e) => {
            error!("Failed to fetch Discord user info: {:?}", e);
            notify_error("Failed to fetch Discord user info");
            return Err(HttpError::forbidden("Failed to fetch Discord user info"));
        }
    };

    if user_info.verified.is_none() || !user_info.verified.unwrap() {
        notify_error("Discord email not verified");
        return Err(HttpError::forbidden("Discord email not verified"));
    }

    let user = match crate::entities::User::update_or_register_by_discord_id(
        &state.db,
        user_info.id.clone(),
        user_info.username.clone(),
        discord_creds.build_avatar_url(&user_info),
        user_info.email.clone(),
    ).await {
        Ok(user) => user,
        Err(e) => {
            error!("Failed to register user: {:?}", e);
            notify_error("Failed to register user");
            return Err(HttpError::internal_error("Failed to register user"));
        }
    };

    let jwt_token = match sign_token(&user, &state.config) {
        Ok(token) => token,
        Err(e) => {
            error!("Failed to sign token: {:?}", e);
            notify_error("Failed to sign token");
            return Err(HttpError::internal_error("Failed to sign token"));
        }
    };

    if is_polling {
        state.notify_auth_complete(
            delivery_target.clone(),
            AuthPollResult::Success {
                access_token: jwt_token.clone(),
                user_id: user.id.to_string(),
                username: user.username.clone(),
                avatar_url: user.avatar_url.clone().unwrap_or_default(),
            },
        );
    }

    Ok(Json(AuthorizeResponse {
        access_token: jwt_token,
        id: user.id.to_string(),
        username: user.username.clone(),
        avatar_url: user.avatar_url.unwrap_or_default(),
        delivery_method,
        delivery_target,
    }))
}
