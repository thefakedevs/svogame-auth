use crate::app::http::{HttpError, HttpResult};
use crate::app::state::AppStateExtractor;
use crate::domains::auth::runtime::AuthPollResult;
use crate::entities::{AuthRay, AuthRayModel, User, auth_ray::TokenDeliveryMethod};
use crate::services::audit::{ACTION_USER_LEGAL_ACCEPTED, ACTION_USER_REGISTERED, write_audit_log};
use crate::services::discord::exchange_code;
use crate::services::token::sign_token;
use axum::Json;
use axum::extract::State;
use sea_orm::{ModelTrait, TransactionTrait};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::error;
use utoipa::ToSchema;

const LEGAL_USER_AGREEMENT_VERSION: &str = "2026-04-11";
const LEGAL_PRIVACY_POLICY_VERSION: &str = "2026-04-11";
const AUTH_SESSION_MAX_AGE_SECONDS: i64 = 3600;

#[derive(Serialize, ToSchema)]
pub struct PrepareAuthResponse {
    #[serde(rename = "powPrefix")]
    pow_prefix: String,
    #[serde(rename = "powComplexity")]
    pow_complexity: i16,
    #[serde(rename = "oauthUrl")]
    oauth2_url: String,
    #[serde(rename = "deliveryMethod")]
    delivery_method: String,
    #[serde(rename = "deliveryTarget")]
    delivery_target: String,
}

#[derive(Deserialize, ToSchema)]
pub struct PrepareAuthRequest {
    #[serde(rename = "redirectUrl")]
    pub redirect_url: Option<String>,
    #[serde(rename = "deliveryMethod")]
    pub delivery_method: String,
}

#[derive(Serialize, ToSchema)]
pub struct AuthorizeResponse {
    status: String,
    #[serde(rename = "registrationToken", skip_serializing_if = "Option::is_none")]
    registration_token: Option<String>,
    #[serde(rename = "accessToken", skip_serializing_if = "Option::is_none")]
    access_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    username: Option<String>,
    #[serde(rename = "avatarUrl", skip_serializing_if = "Option::is_none")]
    avatar_url: Option<String>,
    #[serde(rename = "deliveryMethod", skip_serializing_if = "Option::is_none")]
    delivery_method: Option<String>,
    #[serde(rename = "deliveryTarget", skip_serializing_if = "Option::is_none")]
    delivery_target: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    referral: Option<crate::services::referrals::PublicReferralCampaignView>,
}

impl AuthorizeResponse {
    fn authorized(
        access_token: String,
        user_id: String,
        username: String,
        avatar_url: String,
        delivery_method: String,
        delivery_target: String,
    ) -> Self {
        Self {
            status: "authorized".to_string(),
            registration_token: None,
            access_token: Some(access_token),
            id: Some(user_id),
            username: Some(username),
            avatar_url: Some(avatar_url),
            delivery_method: Some(delivery_method),
            delivery_target: Some(delivery_target),
            referral: None,
        }
    }

    fn terms_required(registration_token: String) -> Self {
        Self {
            status: "terms_required".to_string(),
            registration_token: Some(registration_token),
            access_token: None,
            id: None,
            username: None,
            avatar_url: None,
            delivery_method: None,
            delivery_target: None,
            referral: None,
        }
    }
}

#[derive(Deserialize, ToSchema)]
pub struct AuthorizeRequest {
    #[serde(rename = "powPrefix")]
    pub pow_prefix: String,
    #[serde(rename = "powSolution")]
    pub pow_solution: String,
    #[serde(rename = "discordCode")]
    pub discord_code: String,
}

#[derive(Deserialize, ToSchema)]
pub struct RegisterRequest {
    #[serde(rename = "registrationToken")]
    pub registration_token: String,
    #[serde(rename = "acceptedUserAgreement")]
    pub accepted_user_agreement: bool,
    #[serde(rename = "acceptedPrivacyPolicy")]
    pub accepted_privacy_policy: bool,
    #[serde(rename = "referralCode")]
    pub referral_code: Option<String>,
    #[serde(rename = "referralSource")]
    pub referral_source: Option<String>,
}

#[utoipa::path(
    post,
    path = "/api/auth/prepare",
    request_body(
        content = PrepareAuthRequest,
        description = "Prepare an authentication session. `deliveryMethod` controls whether the final token is returned immediately through redirect flow or later through polling."
    ),
    responses(
        (status = 200, description = "Authentication challenge prepared. Response includes PoW parameters and token delivery metadata.", body = PrepareAuthResponse),
        (status = 400, description = "Invalid delivery method or missing redirect URL for redirect delivery mode."),
        (status = 500, description = "Server failed to create auth challenge.")
    ),
    tag = "auth"
)]
pub async fn prepare_auth(
    State(state): AppStateExtractor,
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
        TokenDeliveryMethod::Redirect => match body.redirect_url {
            Some(url) => url,
            None => {
                return Err(HttpError::bad_request(
                    "redirectUrl is required for redirect delivery method",
                ));
            }
        },
        TokenDeliveryMethod::Polling => uuid::Uuid::new_v4().to_string(),
    };

    let ray = AuthRay::create_with_complexity(
        db,
        state.config.pow_complexity,
        delivery_method,
        delivery_target,
    )
    .await
    .map_err(|e| {
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

#[utoipa::path(
    post,
    path = "/api/auth/authorize",
    request_body(
        content = AuthorizeRequest,
        description = "Complete Discord-based authentication by presenting PoW solution and Discord OAuth code. Existing users are logged in immediately. New users receive a temporary registration token and must accept legal documents before account creation."
    ),
    responses(
        (status = 200, description = "Authentication completed or additional legal acceptance is required.", body = AuthorizeResponse),
        (status = 400, description = "Invalid or expired challenge prefix."),
        (status = 403, description = "PoW invalid, Discord code invalid, missing required scopes, email not verified, user deactivated, or token exchange rejected."),
        (status = 500, description = "Internal failure while registering user, writing audit log, or signing token.")
    ),
    tag = "auth"
)]
pub async fn authorize(
    State(state): AppStateExtractor,
    Json(body): Json<AuthorizeRequest>,
) -> HttpResult<Json<AuthorizeResponse>> {
    let state = &state.read().await;
    let ray = load_auth_ray_by_prefix(&state.db, &body.pow_prefix).await?;

    if ray.registration_token.is_some() {
        return Err(HttpError::bad_request(
            "Auth session is already waiting for legal acceptance",
        ));
    }

    let is_polling = matches!(ray.delivery_method, TokenDeliveryMethod::Polling);
    let delivery_method = delivery_method_name(&ray.delivery_method);
    let delivery_target = ray.delivery_target.clone();

    let notify_error = |msg: &str| {
        if is_polling {
            state.auth.notify_complete(
                delivery_target.clone(),
                AuthPollResult::Error {
                    message: msg.to_string(),
                },
            );
        }
    };

    if !crate::services::pow::verify_pow(
        &body.pow_solution,
        &ray.pow_prefix,
        ray.pow_complexity,
        &ray.created_at,
    ) {
        delete_auth_ray(&state.db, &ray).await?;
        notify_error("Invalid PoW solution");
        return Err(HttpError::forbidden("Invalid PoW solution"));
    }

    let discord_creds = match exchange_code(&state.config.discord, &body.discord_code).await {
        Ok(creds) => creds,
        Err(e) => {
            error!("Failed to exchange Discord code: {:?}", e);
            let _ = delete_auth_ray(&state.db, &ray).await;
            notify_error("Failed to exchange Discord code");
            return Err(HttpError::forbidden("Failed to exchange Discord code"));
        }
    };

    {
        let scopes = discord_creds
            .scope
            .split(' ')
            .map(|scope| scope.to_string())
            .collect::<Vec<String>>();
        if state
            .config
            .discord
            .required_scopes
            .iter()
            .any(|scope| !scopes.contains(scope))
        {
            let _ = delete_auth_ray(&state.db, &ray).await;
            notify_error("Missing required Discord scopes");
            return Err(HttpError::forbidden("Missing required Discord scopes"));
        }
    }

    let user_info = match crate::services::discord::get_user_info(
        &state.config.discord,
        &discord_creds.access_token,
    )
    .await
    {
        Ok(info) => info,
        Err(e) => {
            error!("Failed to fetch Discord user info: {:?}", e);
            let _ = delete_auth_ray(&state.db, &ray).await;
            notify_error("Failed to fetch Discord user info");
            return Err(HttpError::forbidden("Failed to fetch Discord user info"));
        }
    };

    if user_info.verified != Some(true) {
        let _ = delete_auth_ray(&state.db, &ray).await;
        notify_error("Discord email not verified");
        return Err(HttpError::forbidden("Discord email not verified"));
    }

    if User::find_by_discord_id(&state.db, &user_info.id)
        .await
        .map_err(|e| {
            error!("Failed to look up user by discord id: {:?}", e);
            HttpError::internal_error("Failed to look up user")
        })?
        .is_some()
    {
        let user_result = match User::update_or_register_by_discord_id(
            &state.db,
            user_info.id.clone(),
            user_info.username.clone(),
            discord_creds.build_avatar_url(&user_info),
            user_info.email.clone(),
        )
        .await
        {
            Ok(user) => user,
            Err(e) => {
                error!("Failed to update user during auth: {:?}", e);
                let _ = delete_auth_ray(&state.db, &ray).await;
                notify_error("Failed to update user");
                return Err(HttpError::internal_error("Failed to update user"));
            }
        };

        if !user_result.user.is_active {
            let _ = delete_auth_ray(&state.db, &ray).await;
            notify_error("User is deactivated");
            return Err(HttpError::forbidden("User is deactivated"));
        }

        let response = authorize_existing_user(
            state,
            user_result.user,
            delivery_method,
            delivery_target.clone(),
            is_polling,
        )
        .await?;

        delete_auth_ray(&state.db, &ray).await?;
        return Ok(Json(response));
    }

    let pending_ray = AuthRay::mark_pending_registration(
        &state.db,
        ray,
        crate::entities::auth_ray::PendingRegistrationProfile {
            discord_id: user_info.id.clone(),
            username: user_info.username.clone(),
            avatar_url: discord_creds.build_avatar_url(&user_info),
            email: user_info.email,
        },
    )
    .await
    .map_err(|e| {
        error!("Failed to persist pending registration: {:?}", e);
        HttpError::internal_error("Failed to persist pending registration")
    })?;

    Ok(Json(AuthorizeResponse::terms_required(
        pending_ray.registration_token.unwrap_or_default(),
    )))
}

#[utoipa::path(
    post,
    path = "/api/auth/register",
    request_body(
        content = RegisterRequest,
        description = "Finalize first-time registration after Discord identity is verified by accepting the required legal documents."
    ),
    responses(
        (status = 200, description = "Registration completed successfully and token issued.", body = AuthorizeResponse),
        (status = 400, description = "Invalid or expired registration token, or required legal documents were not accepted."),
        (status = 403, description = "User is deactivated."),
        (status = 500, description = "Internal failure while creating the user, writing audit log, or signing token.")
    ),
    tag = "auth"
)]
pub async fn register(
    State(state): AppStateExtractor,
    Json(body): Json<RegisterRequest>,
) -> HttpResult<Json<AuthorizeResponse>> {
    validate_legal_acceptance(&body)?;

    let state = &state.read().await;
    let ray = AuthRay::find_by_registration_token(&state.db, &body.registration_token)
        .await
        .map_err(|e| {
            error!("Database error while finding registration token: {:?}", e);
            HttpError::internal_error("Database error")
        })?
        .ok_or_else(|| HttpError::bad_request("Invalid or expired registration token"))?;

    if auth_session_expired(ray.created_at) {
        delete_auth_ray(&state.db, &ray).await?;
        return Err(HttpError::bad_request("Registration session expired"));
    }

    let pending_discord_id = ray
        .pending_discord_id
        .clone()
        .ok_or_else(|| HttpError::bad_request("Registration session is incomplete"))?;
    let pending_username = ray
        .pending_username
        .clone()
        .ok_or_else(|| HttpError::bad_request("Registration session is incomplete"))?;
    let pending_avatar_url = ray.pending_avatar_url.clone();
    let pending_email = ray.pending_email.clone();

    let tx = state.db.begin().await.map_err(|e| {
        error!("Failed to start registration transaction: {:?}", e);
        HttpError::internal_error("Failed to start registration transaction")
    })?;

    let user_result = User::update_or_register_by_discord_id(
        &tx,
        pending_discord_id.clone(),
        pending_username,
        pending_avatar_url,
        pending_email,
    )
    .await
    .map_err(|e| {
        error!("Failed to register user after legal acceptance: {:?}", e);
        HttpError::internal_error("Failed to register user")
    })?;
    let user = user_result.user;

    let mut applied_referral_code = None;

    if user_result.created {
        write_audit_log(
            &tx,
            ACTION_USER_REGISTERED,
            None,
            Some(user.id),
            None,
            Some(json!({
                "discordId": user.discord_id,
                "username": user.username,
            })),
        )
        .await
        .map_err(|e| {
            error!("Failed to write registration audit log: {:?}", e);
            HttpError::internal_error("Failed to write registration audit log")
        })?;

        write_audit_log(
            &tx,
            ACTION_USER_LEGAL_ACCEPTED,
            None,
            Some(user.id),
            None,
            Some(legal_acceptance_metadata()),
        )
        .await
        .map_err(|e| {
            error!("Failed to write legal acceptance audit log: {:?}", e);
            HttpError::internal_error("Failed to write legal acceptance audit log")
        })?;

        if let Some(referral_code) = body
            .referral_code
            .as_deref()
            .map(str::trim)
            .filter(|code| !code.is_empty())
        {
            let source = body
                .referral_source
                .as_deref()
                .unwrap_or(crate::services::referrals::REGISTRATION_SOURCE_MANUAL);
            let applied = crate::services::referrals::apply_registration_referral(
                &tx,
                user.id,
                referral_code,
                source,
            )
            .await
            .map_err(|e| {
                error!("Failed to apply referral registration: {:?}", e);
                HttpError::bad_request(e.to_string())
            })?;
            if applied.is_some() {
                applied_referral_code = Some(referral_code.to_string());
            }
        }
    }

    if !user.is_active {
        return Err(HttpError::forbidden("User is deactivated"));
    }

    delete_auth_ray(&tx, &ray).await?;

    tx.commit().await.map_err(|e| {
        error!("Failed to commit registration transaction: {:?}", e);
        HttpError::internal_error("Failed to commit registration")
    })?;

    let mut response = authorize_existing_user(
        state,
        user,
        delivery_method_name(&ray.delivery_method),
        ray.delivery_target.clone(),
        matches!(ray.delivery_method, TokenDeliveryMethod::Polling),
    )
    .await?;
    if let Some(referral_code) = applied_referral_code {
        response.referral =
            crate::services::referrals::get_public_campaign(&state.db, &referral_code)
                .await
                .ok();
    }

    Ok(Json(response))
}

async fn authorize_existing_user(
    state: &crate::app::state::AppState,
    user: crate::entities::UserModel,
    delivery_method: String,
    delivery_target: String,
    is_polling: bool,
) -> HttpResult<AuthorizeResponse> {
    let jwt_token = sign_token(&user, &state.config).map_err(|e| {
        error!("Failed to sign token: {:?}", e);
        HttpError::internal_error("Failed to sign token")
    })?;

    if is_polling {
        state.auth.notify_complete(
            delivery_target.clone(),
            AuthPollResult::Success {
                access_token: jwt_token.clone(),
                user_id: user.id.to_string(),
                username: user.username.clone(),
                avatar_url: user.avatar_url.clone().unwrap_or_default(),
            },
        );
    }

    Ok(AuthorizeResponse::authorized(
        jwt_token,
        user.id.to_string(),
        user.username.clone(),
        user.avatar_url.unwrap_or_default(),
        delivery_method,
        delivery_target,
    ))
}

async fn load_auth_ray_by_prefix(
    db: &sea_orm::DatabaseConnection,
    pow_prefix: &str,
) -> HttpResult<AuthRayModel> {
    AuthRay::find_by_prefix(db, pow_prefix)
        .await
        .map_err(|e| {
            error!("Database error while finding auth ray: {:?}", e);
            HttpError::internal_error("Database error")
        })?
        .ok_or_else(|| HttpError::bad_request("Invalid or expired pow_prefix"))
}

async fn delete_auth_ray(db: &impl sea_orm::ConnectionTrait, ray: &AuthRayModel) -> HttpResult<()> {
    ray.clone().delete(db).await.map_err(|e| {
        error!("Failed to delete auth ray: {:?}", e);
        HttpError::internal_error("Failed to delete auth ray")
    })?;
    Ok(())
}

fn delivery_method_name(delivery_method: &TokenDeliveryMethod) -> String {
    match delivery_method {
        TokenDeliveryMethod::Redirect => "redirect".to_string(),
        TokenDeliveryMethod::Polling => "polling".to_string(),
    }
}

fn validate_legal_acceptance(body: &RegisterRequest) -> HttpResult<()> {
    if !body.accepted_user_agreement {
        return Err(HttpError::bad_request(
            "User agreement must be accepted before registration",
        ));
    }

    if !body.accepted_privacy_policy {
        return Err(HttpError::bad_request(
            "Privacy policy must be accepted before registration",
        ));
    }

    Ok(())
}

fn auth_session_expired(created_at: chrono::DateTime<chrono::Utc>) -> bool {
    chrono::Utc::now()
        .signed_duration_since(created_at)
        .num_seconds()
        > AUTH_SESSION_MAX_AGE_SECONDS
}

fn legal_acceptance_metadata() -> serde_json::Value {
    json!({
        "documents": [
            {
                "key": "user_agreement",
                "version": LEGAL_USER_AGREEMENT_VERSION,
            },
            {
                "key": "privacy_policy",
                "version": LEGAL_PRIVACY_POLICY_VERSION,
            }
        ]
    })
}
