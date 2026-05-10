use axum::Json;
use axum::body::Bytes;
use axum::extract::{Multipart, Path, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::app::auth::{get_actor_from_headers, AuthenticatedActor};
use crate::app::http::{HttpError, HttpResult};
use crate::app::state::AppStateExtractor;
use crate::services::littlemice::{self, FailLittlemiceCheckPayload, PushLittlemicePayload};

const MAX_MULTIPART_BODY_SIZE: usize = 13 * 1024 * 1024;
const MULTIPART_READ_TIMEOUT_SECS: u64 = 10;

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateLittlemiceCheckRequest {
    #[serde(rename = "playerUuid")]
    pub player_uuid: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct LittlemiceCreateCheckResponse {
    pub id: String,
    #[serde(rename = "playerUuid")]
    pub player_uuid: String,
    #[serde(rename = "pushUrl")]
    pub push_url: String,
    pub status: String,
    #[serde(rename = "expiresAt")]
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct LittlemiceCheckStatusResponse {
    pub id: String,
    #[serde(rename = "playerUuid")]
    pub player_uuid: String,
    #[serde(rename = "serviceSystemName")]
    pub service_system_name: String,
    pub status: String,
    #[serde(rename = "failureReason")]
    pub failure_reason: Option<String>,
    #[serde(rename = "requestedAt")]
    pub requested_at: chrono::DateTime<chrono::Utc>,
    #[serde(rename = "receivedAt")]
    pub received_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(rename = "completedAt")]
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(rename = "expiresAt")]
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct LittlemicePushAckResponse {
    pub status: &'static str,
    #[serde(rename = "checkId")]
    pub check_id: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct FailLittlemiceCheckRequest {
    #[serde(rename = "errorMessage")]
    pub error_message: String,
    pub stacktrace: Option<String>,
}

#[utoipa::path(
    post,
    path = "/api/littlemice/checks",
    request_body = CreateLittlemiceCheckRequest,
    responses(
        (status = 200, description = "Creates a pending littlemice anti-cheat check and returns a one-time upload URL.", body = LittlemiceCreateCheckResponse),
        (status = 400, description = "Invalid player UUID."),
        (status = 401, description = "Missing bearer token."),
        (status = 403, description = "Service token required.")
    ),
    security(("bearer_auth" = [])),
    tag = "littlemice"
)]
pub async fn create_check(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Json(body): Json<CreateLittlemiceCheckRequest>,
) -> HttpResult<Json<LittlemiceCreateCheckResponse>> {
    let state = state.read().await;
    let service = match get_actor_from_headers(&headers, &state).await? {
        AuthenticatedActor::Service(service) => service,
        AuthenticatedActor::User(_) => {
            return Err(HttpError::forbidden("Service token required"));
        }
    };
    let player_uuid = Uuid::parse_str(&body.player_uuid)
        .map_err(|_| HttpError::bad_request("Invalid player UUID"))?;
    let created = littlemice::create_check(
        &state.db,
        &state.config,
        &service,
        player_uuid,
    )
    .await
    .map_err(map_domain_error)?;

    Ok(Json(LittlemiceCreateCheckResponse {
        id: created.model.id.to_string(),
        player_uuid: created.model.player_uuid.to_string(),
        push_url: created.push_url,
        status: created.model.status,
        expires_at: created.model.push_token_expires_at,
    }))
}

#[utoipa::path(
    post,
    path = "/api/littlemice/push/{push_token}",
    params(
        ("push_token" = String, Path, description = "One-time littlemice upload token.")
    ),
    request_body(
        content_type = "multipart/form-data",
        description = "Multipart payload with `screenshot` file and optional `log` file plus optional `clientInfo` text."
    ),
    responses(
        (status = 200, description = "Littlemice payload uploaded.", body = LittlemicePushAckResponse),
        (status = 400, description = "Invalid payload."),
        (status = 404, description = "Unknown push token."),
        (status = 410, description = "Expired push token.")
    ),
    tag = "littlemice"
)]
pub async fn push_check(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Path(push_token): Path<String>,
    mut multipart: Multipart,
) -> Result<Json<LittlemicePushAckResponse>, Response> {
    validate_multipart_headers(&headers)?;

    let mut screenshot_bytes = None;
    let mut screenshot_content_type = None;
    let mut log_bytes = None;
    let mut log_content_type = None;
    let mut client_info_text = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|error| HttpError::bad_request(format!("Invalid multipart body: {error}")).into_response())?
    {
        let name = field.name().unwrap_or_default().to_string();
        match name.as_str() {
            "screenshot" => {
                screenshot_content_type = field.content_type().map(|value| value.to_string());
                screenshot_bytes = Some(read_multipart_bytes(field).await?);
            }
            "log" => {
                log_content_type = field.content_type().map(|value| value.to_string());
                log_bytes = Some(read_multipart_bytes(field).await?);
            }
            "clientInfo" => {
                let data = read_multipart_bytes(field).await?;
                client_info_text = Some(
                    String::from_utf8(data.to_vec())
                        .map_err(|_| HttpError::bad_request("clientInfo must be valid UTF-8").into_response())?,
                );
            }
            _ => {}
        }
    }

    let payload = PushLittlemicePayload {
        screenshot_bytes: screenshot_bytes
            .ok_or_else(|| HttpError::bad_request("screenshot field is required").into_response())?
            .to_vec(),
        screenshot_content_type: screenshot_content_type
            .unwrap_or_else(|| "application/octet-stream".to_string()),
        log_bytes: log_bytes.map(|bytes| bytes.to_vec()),
        log_content_type,
        client_info_text,
    };

    let state = state.read().await;
    let saved = littlemice::accept_push(&state.db, &state.s3, &state.config, &push_token, payload)
        .await
        .map_err(map_domain_error_response)?;

    Ok(Json(LittlemicePushAckResponse {
        status: "ok",
        check_id: saved.id.to_string(),
    }))
}

#[utoipa::path(
    get,
    path = "/api/littlemice/checks/{check_id}",
    params(
        ("check_id" = String, Path, description = "Littlemice check UUID.")
    ),
    responses(
        (status = 200, description = "Littlemice check status.", body = LittlemiceCheckStatusResponse),
        (status = 400, description = "Invalid check ID."),
        (status = 401, description = "Missing bearer token."),
        (status = 403, description = "Service token required."),
        (status = 404, description = "Check not found.")
    ),
    security(("bearer_auth" = [])),
    tag = "littlemice"
)]
pub async fn get_check_status(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Path(check_id): Path<String>,
) -> HttpResult<Json<LittlemiceCheckStatusResponse>> {
    let state = state.read().await;
    let service = match get_actor_from_headers(&headers, &state).await? {
        AuthenticatedActor::Service(service) => service,
        AuthenticatedActor::User(_) => {
            return Err(HttpError::forbidden("Service token required"));
        }
    };

    let check_id =
        Uuid::parse_str(&check_id).map_err(|_| HttpError::bad_request("Invalid check ID"))?;
    let item = littlemice::get_check(&state.db, check_id)
        .await
        .map_err(map_domain_error)?;
    if item.service_system_name != service.system_name {
        return Err(HttpError::forbidden(
            "Service token cannot access checks created by another service",
        ));
    }

    Ok(Json(map_status_response(item)))
}

#[utoipa::path(
    post,
    path = "/api/littlemice/checks/{check_id}/fail",
    params(
        ("check_id" = String, Path, description = "Littlemice check UUID.")
    ),
    request_body = FailLittlemiceCheckRequest,
    responses(
        (status = 200, description = "Littlemice check marked as failed because the client reported an internal error.", body = LittlemiceCheckStatusResponse),
        (status = 400, description = "Invalid check ID or invalid payload."),
        (status = 401, description = "Missing bearer token."),
        (status = 403, description = "Service token required or check belongs to another service."),
        (status = 404, description = "Check not found.")
    ),
    security(("bearer_auth" = [])),
    tag = "littlemice"
)]
pub async fn fail_check(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Path(check_id): Path<String>,
    Json(body): Json<FailLittlemiceCheckRequest>,
) -> HttpResult<Json<LittlemiceCheckStatusResponse>> {
    let state = state.read().await;
    let service = match get_actor_from_headers(&headers, &state).await? {
        AuthenticatedActor::Service(service) => service,
        AuthenticatedActor::User(_) => {
            return Err(HttpError::forbidden("Service token required"));
        }
    };

    let check_id =
        Uuid::parse_str(&check_id).map_err(|_| HttpError::bad_request("Invalid check ID"))?;
    let item = littlemice::fail_check(
        &state.db,
        &state.config,
        &service,
        check_id,
        FailLittlemiceCheckPayload {
            error_message: body.error_message,
            stacktrace: body.stacktrace,
        },
    )
    .await
    .map_err(map_domain_error)?;

    Ok(Json(map_status_response(item)))
}

fn map_status_response(item: crate::entities::LittlemiceCheckModel) -> LittlemiceCheckStatusResponse {
    LittlemiceCheckStatusResponse {
        id: item.id.to_string(),
        player_uuid: item.player_uuid.to_string(),
        service_system_name: item.service_system_name,
        status: item.status,
        failure_reason: item.failure_reason,
        requested_at: item.requested_at,
        received_at: item.received_at,
        completed_at: item.completed_at,
        expires_at: item.push_token_expires_at,
    }
}

fn validate_multipart_headers(headers: &HeaderMap) -> Result<(), Response> {
    if let Some(content_length) = headers.get(header::CONTENT_LENGTH)
        && let Ok(len) = content_length.to_str().unwrap_or_default().parse::<usize>()
        && len > MAX_MULTIPART_BODY_SIZE
    {
        return Err(HttpError::bad_request("Littlemice upload exceeds body size limit").into_response());
    }
    let content_type = headers
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| HttpError::bad_request("Missing Content-Type").into_response())?;
    if !content_type.starts_with("multipart/form-data") {
        return Err(HttpError::bad_request("Expected multipart/form-data").into_response());
    }
    Ok(())
}

async fn read_multipart_bytes(field: axum::extract::multipart::Field<'_>) -> Result<Bytes, Response> {
    tokio::time::timeout(std::time::Duration::from_secs(MULTIPART_READ_TIMEOUT_SECS), field.bytes())
        .await
        .map_err(|_| HttpError::bad_request("Multipart field read timeout").into_response())?
        .map_err(|error| HttpError::bad_request(format!("Failed to read multipart field: {error}")).into_response())
}

fn map_domain_error(error: anyhow::Error) -> HttpError {
    let (kind, detail) = littlemice::parse_domain_error(error);
    match kind {
        "not_found" => HttpError::not_found(detail),
        "forbidden" => HttpError::forbidden(detail),
        "bad_request" => HttpError::bad_request(detail),
        "gone" => HttpError::new(StatusCode::GONE, detail),
        _ => HttpError::internal_error(detail),
    }
}

fn map_domain_error_response(error: anyhow::Error) -> Response {
    map_domain_error(error).into_response()
}
