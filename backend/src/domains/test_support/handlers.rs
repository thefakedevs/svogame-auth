use axum::extract::State;
use axum::Json;
use sea_orm::{ActiveModelTrait, ActiveValue, ColumnTrait, EntityTrait, QueryFilter, Set, TransactionTrait};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::app::http::{HttpError, HttpResult};
use crate::app::state::AppStateExtractor;
use crate::entities::{
    User, UserActiveModel, UserRestriction, UserRestrictionActiveModel, UserRestrictionColumn,
};
use crate::services::restrictions::RestrictionKind;
use crate::services::token::sign_token;

#[derive(Deserialize, ToSchema)]
pub struct IssueTokenRequest {
    #[serde(rename = "userId")]
    pub user_id: Option<String>,
    #[serde(rename = "discordId")]
    pub discord_id: Option<String>,
    pub username: Option<String>,
    #[serde(rename = "avatarUrl")]
    pub avatar_url: Option<String>,
    pub email: Option<String>,
    #[serde(rename = "isSuperuser")]
    pub is_superuser: Option<bool>,
    #[serde(rename = "isActive")]
    pub is_active: Option<bool>,
    #[serde(rename = "authEpoch")]
    pub auth_epoch: Option<i32>,
    pub restrictions: Option<Vec<String>>,
}

#[derive(Serialize, ToSchema)]
pub struct IssueTokenResponse {
    #[serde(rename = "accessToken")]
    pub access_token: String,
    #[serde(rename = "userId")]
    pub user_id: String,
    pub username: String,
    #[serde(rename = "isSuperuser")]
    pub is_superuser: bool,
    #[serde(rename = "isActive")]
    pub is_active: bool,
    pub restrictions: Vec<String>,
}

#[utoipa::path(
    post,
    path = "/api/test/issue-token",
    request_body(
        content = IssueTokenRequest,
        description = "Debug-only test helper that creates or mutates a user and returns a signed JWT. This route exists only in debug builds and should never be used as production authentication."
    ),
    responses(
        (status = 200, description = "Token issued for test flows.", body = IssueTokenResponse),
        (status = 400, description = "Invalid UUID or unknown restriction key in the request."),
        (status = 500, description = "Failed to persist debug user or sign test token.")
    ),
    tag = "debug"
)]
pub async fn issue_token(
    State(state): AppStateExtractor,
    Json(body): Json<IssueTokenRequest>,
) -> HttpResult<Json<IssueTokenResponse>> {
    let state = state.read().await;
    let tx = state
        .db
        .begin()
        .await
        .map_err(|e| HttpError::internal_error(format!("Failed to start transaction: {e}")))?;

    let requested_user_id = body
        .user_id
        .as_deref()
        .map(|value| Uuid::parse_str(value).map_err(|_| HttpError::bad_request("Invalid userId")))
        .transpose()?
        .unwrap_or_else(Uuid::new_v4);
    let requested_restrictions = body.restrictions.unwrap_or_default();
    let parsed_restrictions = requested_restrictions
        .iter()
        .map(|key| key.parse::<RestrictionKind>().map_err(|e| HttpError::bad_request(e.to_string())))
        .collect::<Result<Vec<_>, _>>()?;

    let existing_user = User::find_by_id(requested_user_id)
        .one(&tx)
        .await
        .map_err(|e| HttpError::internal_error(format!("Failed to load user: {e}")))?;

    let user = if let Some(existing_user) = existing_user {
        let mut active_user: UserActiveModel = existing_user.into();
        if let Some(username) = body.username {
            active_user.username = Set(username);
        }
        if let Some(avatar_url) = body.avatar_url {
            active_user.avatar_url = Set(Some(avatar_url));
        }
        if let Some(email) = body.email {
            active_user.email = Set(Some(email));
        }
        if let Some(is_superuser) = body.is_superuser {
            active_user.is_superuser = Set(is_superuser);
        }
        if let Some(is_active) = body.is_active {
            active_user.is_active = Set(is_active);
        }
        if let Some(auth_epoch) = body.auth_epoch {
            active_user.auth_epoch = Set(auth_epoch);
        }
        active_user
            .update(&tx)
            .await
            .map_err(|e| HttpError::internal_error(format!("Failed to update test user: {e}")))?
    } else {
        UserActiveModel {
            id: ActiveValue::Set(requested_user_id),
            discord_id: ActiveValue::Set(
                body.discord_id
                    .unwrap_or_else(|| format!("debug-discord-{}", requested_user_id)),
            ),
            username: ActiveValue::Set(
                body.username
                    .unwrap_or_else(|| format!("User{}", &requested_user_id.simple().to_string()[..8])),
            ),
            avatar_url: ActiveValue::Set(body.avatar_url),
            email: ActiveValue::Set(body.email),
            auth_epoch: ActiveValue::Set(body.auth_epoch.unwrap_or(0)),
            is_active: ActiveValue::Set(body.is_active.unwrap_or(true)),
            is_superuser: ActiveValue::Set(body.is_superuser.unwrap_or(false)),
            squad_id: ActiveValue::Set(None),
            deactivation_reason: ActiveValue::Set(None),
            last_login_at: ActiveValue::Set(chrono::Utc::now()),
            created_at: ActiveValue::Set(chrono::Utc::now()),
        }
        .insert(&tx)
        .await
        .map_err(|e| HttpError::internal_error(format!("Failed to create test user: {e}")))?
    };

    UserRestriction::delete_many()
        .filter(UserRestrictionColumn::UserId.eq(user.id))
        .exec(&tx)
        .await
        .map_err(|e| HttpError::internal_error(format!("Failed to reset user restrictions: {e}")))?;

    for restriction in &parsed_restrictions {
        UserRestrictionActiveModel {
            id: ActiveValue::NotSet,
            user_id: Set(user.id),
            restriction_key: Set(restriction.as_str().to_string()),
            reason: Set(Some("Issued by debug test token endpoint".to_string())),
            created_at: Set(chrono::Utc::now()),
        }
        .insert(&tx)
        .await
        .map_err(|e| HttpError::internal_error(format!("Failed to create test restriction: {e}")))?;
    }

    tx.commit()
        .await
        .map_err(|e| HttpError::internal_error(format!("Failed to commit test token transaction: {e}")))?;

    let token = sign_token(&user, &state.config)
        .map_err(|e| HttpError::internal_error(format!("Failed to sign token: {e}")))?;

    Ok(Json(IssueTokenResponse {
        access_token: token,
        user_id: user.id.to_string(),
        username: user.username,
        is_superuser: user.is_superuser,
        is_active: user.is_active,
        restrictions: parsed_restrictions
            .into_iter()
            .map(|restriction| restriction.as_str().to_string())
            .collect(),
    }))
}
