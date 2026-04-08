use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::app::auth::require_human_superuser;
use crate::app::http::{HttpError, HttpResult};
use crate::app::state::AppStateExtractor;
use crate::services::service_tokens::{
    self, CreateServiceTokenInput, RevokeServiceTokenInput, RotateServiceTokenInput,
};

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateServiceTokenRequest {
    #[serde(rename = "systemName")]
    pub system_name: String,
    pub description: Option<String>,
    #[serde(rename = "expiresAt")]
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct RotateServiceTokenRequest {
    #[serde(rename = "expiresAt")]
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    pub reason: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct RevokeServiceTokenRequest {
    pub reason: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ServiceTokenResponse {
    pub id: String,
    #[serde(rename = "systemName")]
    pub system_name: String,
    pub description: Option<String>,
    #[serde(rename = "isActive")]
    pub is_active: bool,
    #[serde(rename = "expiresAt")]
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(rename = "lastUsedAt")]
    pub last_used_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(rename = "createdByUserId")]
    pub created_by_user_id: String,
    #[serde(rename = "rotatedFromId")]
    pub rotated_from_id: Option<String>,
    #[serde(rename = "createdAt")]
    pub created_at: chrono::DateTime<chrono::Utc>,
    #[serde(rename = "updatedAt")]
    pub updated_at: chrono::DateTime<chrono::Utc>,
    #[serde(rename = "plaintextToken")]
    pub plaintext_token: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ServiceTokenAuditResponse {
    pub id: i64,
    #[serde(rename = "serviceTokenId")]
    pub service_token_id: String,
    pub action: String,
    #[serde(rename = "actorUserId")]
    pub actor_user_id: String,
    pub reason: Option<String>,
    pub metadata: Value,
    #[serde(rename = "createdAt")]
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[utoipa::path(
    get,
    path = "/api/admin/service-tokens",
    responses(
        (status = 200, description = "List all issued service tokens without revealing secrets.", body = [ServiceTokenResponse]),
        (status = 401, description = "Missing bearer token."),
        (status = 403, description = "Human superuser permissions required.")
    ),
    security(("bearer_auth" = [])),
    tag = "admin"
)]
pub async fn list_service_tokens(
    State(state): AppStateExtractor,
    headers: HeaderMap,
) -> HttpResult<Json<Vec<ServiceTokenResponse>>> {
    let state = state.read().await;
    require_human_superuser(&headers, &state).await?;
    let items = service_tokens::list_service_tokens(&state.db)
        .await
        .map_err(map_domain_error)?;
    Ok(Json(items.into_iter().map(|item| map_service_token(item, None)).collect()))
}

#[utoipa::path(
    post,
    path = "/api/admin/service-tokens",
    request_body(
        content = CreateServiceTokenRequest,
        description = "Issue a new service token. `systemName` must be unique, immutable, lowercase, and URL-safe (`[a-z0-9_-]+`). The plaintext token is returned only once in this response."
    ),
    responses(
        (status = 200, description = "Service token created. Save the plaintext token immediately; it cannot be retrieved again.", body = ServiceTokenResponse),
        (status = 400, description = "Invalid payload, duplicate system name, or invalid system name format."),
        (status = 401, description = "Missing bearer token."),
        (status = 403, description = "Human superuser permissions required.")
    ),
    security(("bearer_auth" = [])),
    tag = "admin"
)]
pub async fn create_service_token(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Json(body): Json<CreateServiceTokenRequest>,
) -> HttpResult<Json<ServiceTokenResponse>> {
    let state = state.read().await;
    let admin = require_human_superuser(&headers, &state).await?;
    let created = service_tokens::create_service_token(
        &state.db,
        &state.config,
        CreateServiceTokenInput {
            system_name: body.system_name,
            description: body.description,
            expires_at: body.expires_at,
            actor_user_id: admin.id,
        },
    )
    .await
    .map_err(map_domain_error)?;

    Ok(Json(map_service_token(created.model, Some(created.plaintext_token))))
}

#[utoipa::path(
    get,
    path = "/api/admin/service-tokens/{token_id}",
    params(
        ("token_id" = String, Path, description = "Service token UUID.")
    ),
    responses(
        (status = 200, description = "Service token metadata without secret value.", body = ServiceTokenResponse),
        (status = 400, description = "Invalid token ID."),
        (status = 401, description = "Missing bearer token."),
        (status = 403, description = "Human superuser permissions required."),
        (status = 404, description = "Service token not found.")
    ),
    security(("bearer_auth" = [])),
    tag = "admin"
)]
pub async fn get_service_token(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Path(token_id): Path<String>,
) -> HttpResult<Json<ServiceTokenResponse>> {
    let state = state.read().await;
    require_human_superuser(&headers, &state).await?;
    let token_id = parse_uuid(&token_id)?;
    let token = service_tokens::get_service_token(&state.db, token_id)
        .await
        .map_err(map_domain_error)?;
    Ok(Json(map_service_token(token, None)))
}

#[utoipa::path(
    get,
    path = "/api/admin/service-tokens/{token_id}/audit",
    params(
        ("token_id" = String, Path, description = "Service token UUID.")
    ),
    responses(
        (status = 200, description = "Append-only audit trail for service token lifecycle events such as create, rotate, and revoke.", body = [ServiceTokenAuditResponse]),
        (status = 400, description = "Invalid token ID."),
        (status = 401, description = "Missing bearer token."),
        (status = 403, description = "Human superuser permissions required.")
    ),
    security(("bearer_auth" = [])),
    tag = "admin"
)]
pub async fn get_service_token_audit(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Path(token_id): Path<String>,
) -> HttpResult<Json<Vec<ServiceTokenAuditResponse>>> {
    let state = state.read().await;
    require_human_superuser(&headers, &state).await?;
    let token_id = parse_uuid(&token_id)?;
    let items = service_tokens::get_service_token_audit(&state.db, token_id)
        .await
        .map_err(map_domain_error)?;
    Ok(Json(items.into_iter().map(map_service_token_audit).collect()))
}

#[utoipa::path(
    post,
    path = "/api/admin/service-tokens/{token_id}/rotate",
    params(
        ("token_id" = String, Path, description = "Service token UUID.")
    ),
    request_body(
        content = RotateServiceTokenRequest,
        description = "Rotate a service token by deactivating the old secret and issuing a new one for the same immutable `systemName`. The new plaintext token is returned only once."
    ),
    responses(
        (status = 200, description = "Service token rotated.", body = ServiceTokenResponse),
        (status = 400, description = "Invalid token ID or inactive token."),
        (status = 401, description = "Missing bearer token."),
        (status = 403, description = "Human superuser permissions required."),
        (status = 404, description = "Service token not found.")
    ),
    security(("bearer_auth" = [])),
    tag = "admin"
)]
pub async fn rotate_service_token(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Path(token_id): Path<String>,
    Json(body): Json<RotateServiceTokenRequest>,
) -> HttpResult<Json<ServiceTokenResponse>> {
    let state = state.read().await;
    let admin = require_human_superuser(&headers, &state).await?;
    let rotated = service_tokens::rotate_service_token(
        &state.db,
        &state.config,
        RotateServiceTokenInput {
            token_id: parse_uuid(&token_id)?,
            expires_at: body.expires_at,
            actor_user_id: admin.id,
            reason: body.reason,
        },
    )
    .await
    .map_err(map_domain_error)?;

    Ok(Json(map_service_token(rotated.model, Some(rotated.plaintext_token))))
}

#[utoipa::path(
    post,
    path = "/api/admin/service-tokens/{token_id}/revoke",
    params(
        ("token_id" = String, Path, description = "Service token UUID.")
    ),
    request_body(
        content = RevokeServiceTokenRequest,
        description = "Revoke a service token immediately. Repeated revoke is idempotent and keeps the token inactive."
    ),
    responses(
        (status = 200, description = "Service token revoked or already inactive.", body = ServiceTokenResponse),
        (status = 400, description = "Invalid token ID."),
        (status = 401, description = "Missing bearer token."),
        (status = 403, description = "Human superuser permissions required."),
        (status = 404, description = "Service token not found.")
    ),
    security(("bearer_auth" = [])),
    tag = "admin"
)]
pub async fn revoke_service_token(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Path(token_id): Path<String>,
    Json(body): Json<RevokeServiceTokenRequest>,
) -> HttpResult<Json<ServiceTokenResponse>> {
    let state = state.read().await;
    let admin = require_human_superuser(&headers, &state).await?;
    let token = service_tokens::revoke_service_token(
        &state.db,
        RevokeServiceTokenInput {
            token_id: parse_uuid(&token_id)?,
            actor_user_id: admin.id,
            reason: body.reason,
        },
    )
    .await
    .map_err(map_domain_error)?;

    Ok(Json(map_service_token(token, None)))
}

fn map_service_token(
    token: crate::entities::ServiceTokenModel,
    plaintext_token: Option<String>,
) -> ServiceTokenResponse {
    ServiceTokenResponse {
        id: token.id.to_string(),
        system_name: token.system_name,
        description: token.description,
        is_active: token.is_active,
        expires_at: token.expires_at,
        last_used_at: token.last_used_at,
        created_by_user_id: token.created_by_user_id.to_string(),
        rotated_from_id: token.rotated_from_id.map(|id| id.to_string()),
        created_at: token.created_at,
        updated_at: token.updated_at,
        plaintext_token,
    }
}

fn map_service_token_audit(
    item: crate::entities::ServiceTokenAuditModel,
) -> ServiceTokenAuditResponse {
    ServiceTokenAuditResponse {
        id: item.id,
        service_token_id: item.service_token_id.to_string(),
        action: item.action,
        actor_user_id: item.actor_user_id.to_string(),
        reason: item.reason,
        metadata: parse_metadata(item.metadata),
        created_at: item.created_at,
    }
}

fn parse_metadata(metadata: Option<String>) -> Value {
    metadata
        .as_deref()
        .and_then(|value| serde_json::from_str(value).ok())
        .unwrap_or_else(|| json!({}))
}

fn parse_uuid(value: &str) -> HttpResult<Uuid> {
    Uuid::parse_str(value).map_err(|_| HttpError::bad_request("Invalid service token ID"))
}

fn map_domain_error(error: anyhow::Error) -> HttpError {
    let message = error.to_string();
    if message.contains("not found") || message.contains("Not found") {
        HttpError::not_found(message)
    } else {
        HttpError::bad_request(message)
    }
}
