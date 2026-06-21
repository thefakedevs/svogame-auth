use axum::Json;
use axum::body::Bytes;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::app::auth::require_human_superuser;
use crate::app::http::{HttpError, HttpResult};
use crate::app::state::AppStateExtractor;
use crate::services::littlemice::{self, LittlemiceListQuery};

const DEFAULT_PAGE: u64 = 1;
const DEFAULT_PER_PAGE: u64 = 20;
const MAX_PER_PAGE: u64 = 100;

#[derive(Debug, Deserialize, IntoParams)]
pub struct ListLittlemiceChecksQuery {
    #[serde(rename = "playerUuid")]
    pub player_uuid: Option<String>,
    #[serde(rename = "recentMinutes")]
    pub recent_minutes: Option<i64>,
    pub page: Option<u64>,
    #[serde(rename = "perPage")]
    pub per_page: Option<u64>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct LittlemiceCheckListItemResponse {
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
pub struct LittlemiceCheckListResponse {
    pub items: Vec<LittlemiceCheckListItemResponse>,
    pub total: u64,
    pub page: u64,
    #[serde(rename = "perPage")]
    pub per_page: u64,
    #[serde(rename = "totalPages")]
    pub total_pages: u64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct LittlemiceCheckDetailResponse {
    pub id: String,
    #[serde(rename = "playerUuid")]
    pub player_uuid: String,
    #[serde(rename = "serviceTokenId")]
    pub service_token_id: String,
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
    #[serde(rename = "screenshotSizeBytes")]
    pub screenshot_size_bytes: Option<i64>,
    #[serde(rename = "screenshot2SizeBytes")]
    pub screenshot2_size_bytes: Option<i64>,
    #[serde(rename = "logSizeBytes")]
    pub log_size_bytes: Option<i64>,
    #[serde(rename = "clientInfoText")]
    pub client_info_text: Option<String>,
    #[serde(rename = "clientInfoSizeBytes")]
    pub client_info_size_bytes: Option<i64>,
    #[serde(rename = "screenshotUrl")]
    pub screenshot_url: Option<String>,
    #[serde(rename = "screenshot2Url")]
    pub screenshot2_url: Option<String>,
    #[serde(rename = "logUrl")]
    pub log_url: Option<String>,
}

#[utoipa::path(
    get,
    path = "/api/admin/littlemice/checks",
    params(ListLittlemiceChecksQuery),
    responses(
        (status = 200, description = "Lists littlemice checks. Can optionally filter by player UUID.", body = LittlemiceCheckListResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    ),
    security(("bearer_auth" = [])),
    tag = "admin"
)]
pub async fn list_checks(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Query(query): Query<ListLittlemiceChecksQuery>,
) -> HttpResult<Json<LittlemiceCheckListResponse>> {
    let state = state.read().await;
    require_human_superuser(&headers, &state).await?;

    let page = query.page.unwrap_or(DEFAULT_PAGE).max(1);
    let per_page = query.per_page.unwrap_or(DEFAULT_PER_PAGE).clamp(1, MAX_PER_PAGE);
    let player_uuid = match query.player_uuid.as_deref() {
        Some(value) => Some(Uuid::parse_str(value).map_err(|_| HttpError::bad_request("Invalid player UUID"))?),
        None => None,
    };
    let since_requested_at = match query.recent_minutes {
        Some(value) if value > 0 => Some(Utc::now() - Duration::minutes(value)),
        Some(_) => return Err(HttpError::bad_request("recentMinutes must be positive")),
        None => None,
    };
    let result = littlemice::list_checks(
        &state.db,
        LittlemiceListQuery {
            player_uuid,
            since_requested_at,
            page,
            per_page,
        },
    )
    .await
    .map_err(map_domain_error)?;
    let total_pages = if result.total == 0 { 0 } else { result.total.div_ceil(per_page) };

    Ok(Json(LittlemiceCheckListResponse {
        items: result.items.into_iter().map(map_list_item).collect(),
        total: result.total,
        page,
        per_page,
        total_pages,
    }))
}

#[utoipa::path(
    get,
    path = "/api/admin/users/{player_uuid}/littlemice-checks",
    params(
        ("player_uuid" = String, Path, description = "Player UUID."),
        ListLittlemiceChecksQuery
    ),
    responses(
        (status = 200, description = "Lists littlemice checks for one player.", body = LittlemiceCheckListResponse),
        (status = 400, description = "Invalid player UUID."),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    ),
    security(("bearer_auth" = [])),
    tag = "admin"
)]
pub async fn list_checks_by_player(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Path(player_uuid): Path<String>,
    Query(query): Query<ListLittlemiceChecksQuery>,
) -> HttpResult<Json<LittlemiceCheckListResponse>> {
    list_checks(
        State(state),
        headers,
        Query(ListLittlemiceChecksQuery {
            player_uuid: Some(player_uuid),
            recent_minutes: query.recent_minutes,
            page: query.page,
            per_page: query.per_page,
        }),
    )
    .await
}

#[utoipa::path(
    get,
    path = "/api/admin/littlemice/checks/{check_id}",
    params(
        ("check_id" = String, Path, description = "Littlemice check UUID.")
    ),
    responses(
        (status = 200, description = "Loads one littlemice check including stored free-form client info and content URLs.", body = LittlemiceCheckDetailResponse),
        (status = 400, description = "Invalid check ID."),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Check not found.")
    ),
    security(("bearer_auth" = [])),
    tag = "admin"
)]
pub async fn get_check(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Path(check_id): Path<String>,
) -> HttpResult<Json<LittlemiceCheckDetailResponse>> {
    let state = state.read().await;
    require_human_superuser(&headers, &state).await?;
    let check_id =
        Uuid::parse_str(&check_id).map_err(|_| HttpError::bad_request("Invalid check ID"))?;
    let item = littlemice::get_check(&state.db, check_id)
        .await
        .map_err(map_domain_error)?;

    Ok(Json(map_detail_item(item)))
}

#[utoipa::path(
    get,
    path = "/api/admin/littlemice/checks/{check_id}/screenshot",
    params(
        ("check_id" = String, Path, description = "Littlemice check UUID.")
    ),
    responses(
        (status = 200, description = "Stored littlemice screenshot binary."),
        (status = 400, description = "Invalid check ID."),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Screenshot not found.")
    ),
    security(("bearer_auth" = [])),
    tag = "admin"
)]
pub async fn get_screenshot(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Path(check_id): Path<String>,
) -> Result<Response, Response> {
    let state = state.read().await;
    require_human_superuser(&headers, &state)
        .await
        .map_err(IntoResponse::into_response)?;
    let check_id =
        Uuid::parse_str(&check_id).map_err(|_| HttpError::bad_request("Invalid check ID").into_response())?;
    let item = littlemice::get_check(&state.db, check_id)
        .await
        .map_err(map_domain_error_response)?;
    let key = item
        .screenshot_s3_key
        .ok_or_else(|| HttpError::not_found("Screenshot not found").into_response())?;
    let bytes = littlemice::load_s3_object(&state.s3, &state.config.s3.bucket, &key)
        .await
        .map_err(|error| HttpError::internal_error(format!("Failed to load screenshot: {error}")).into_response())?
        .ok_or_else(|| HttpError::not_found("Screenshot not found").into_response())?;

    Ok(binary_response(
        item.screenshot_content_type
            .as_deref()
            .unwrap_or("application/octet-stream"),
        bytes,
    ))
}

#[utoipa::path(
    get,
    path = "/api/admin/littlemice/checks/{check_id}/screenshot2",
    params(
        ("check_id" = String, Path, description = "Littlemice check UUID.")
    ),
    responses(
        (status = 200, description = "Stored littlemice secondary screenshot binary."),
        (status = 400, description = "Invalid check ID."),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Screenshot2 not found.")
    ),
    security(("bearer_auth" = [])),
    tag = "admin"
)]
pub async fn get_screenshot2(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Path(check_id): Path<String>,
) -> Result<Response, Response> {
    let state = state.read().await;
    require_human_superuser(&headers, &state)
        .await
        .map_err(IntoResponse::into_response)?;
    let check_id = Uuid::parse_str(&check_id)
        .map_err(|_| HttpError::bad_request("Invalid check ID").into_response())?;
    let item = littlemice::get_check(&state.db, check_id)
        .await
        .map_err(map_domain_error_response)?;
    let key = item
        .screenshot2_s3_key
        .ok_or_else(|| HttpError::not_found("Screenshot2 not found").into_response())?;
    let bytes = littlemice::load_s3_object(&state.s3, &state.config.s3.bucket, &key)
        .await
        .map_err(|error| {
            HttpError::internal_error(format!("Failed to load screenshot2: {error}")).into_response()
        })?
        .ok_or_else(|| HttpError::not_found("Screenshot2 not found").into_response())?;

    Ok(binary_response(
        item.screenshot2_content_type
            .as_deref()
            .unwrap_or("application/octet-stream"),
        bytes,
    ))
}

#[utoipa::path(
    get,
    path = "/api/admin/littlemice/checks/{check_id}/log",
    params(
        ("check_id" = String, Path, description = "Littlemice check UUID.")
    ),
    responses(
        (status = 200, description = "Stored littlemice client log."),
        (status = 400, description = "Invalid check ID."),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Log not found.")
    ),
    security(("bearer_auth" = [])),
    tag = "admin"
)]
pub async fn get_log(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Path(check_id): Path<String>,
) -> Result<Response, Response> {
    let state = state.read().await;
    require_human_superuser(&headers, &state)
        .await
        .map_err(IntoResponse::into_response)?;
    let check_id =
        Uuid::parse_str(&check_id).map_err(|_| HttpError::bad_request("Invalid check ID").into_response())?;
    let item = littlemice::get_check(&state.db, check_id)
        .await
        .map_err(map_domain_error_response)?;
    let key = item
        .log_s3_key
        .ok_or_else(|| HttpError::not_found("Log not found").into_response())?;
    let bytes = littlemice::load_s3_object(&state.s3, &state.config.s3.bucket, &key)
        .await
        .map_err(|error| HttpError::internal_error(format!("Failed to load log: {error}")).into_response())?
        .ok_or_else(|| HttpError::not_found("Log not found").into_response())?;

    Ok(binary_response(
        item.log_content_type
            .as_deref()
            .unwrap_or("text/plain; charset=utf-8"),
        bytes,
    ))
}

fn map_list_item(item: crate::entities::LittlemiceCheckModel) -> LittlemiceCheckListItemResponse {
    LittlemiceCheckListItemResponse {
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

fn map_detail_item(item: crate::entities::LittlemiceCheckModel) -> LittlemiceCheckDetailResponse {
    let base = format!("/api/admin/littlemice/checks/{}", item.id);
    LittlemiceCheckDetailResponse {
        id: item.id.to_string(),
        player_uuid: item.player_uuid.to_string(),
        service_token_id: item.service_token_id.to_string(),
        service_system_name: item.service_system_name,
        status: item.status,
        failure_reason: item.failure_reason,
        requested_at: item.requested_at,
        received_at: item.received_at,
        completed_at: item.completed_at,
        expires_at: item.push_token_expires_at,
        screenshot_size_bytes: item.screenshot_size_bytes,
        screenshot2_size_bytes: item.screenshot2_size_bytes,
        log_size_bytes: item.log_size_bytes,
        client_info_text: item.client_info_text,
        client_info_size_bytes: item.client_info_size_bytes,
        screenshot_url: item.screenshot_s3_key.as_ref().map(|_| format!("{base}/screenshot")),
        screenshot2_url: item
            .screenshot2_s3_key
            .as_ref()
            .map(|_| format!("{base}/screenshot2")),
        log_url: item.log_s3_key.as_ref().map(|_| format!("{base}/log")),
    }
}

fn binary_response(content_type: &str, body: Bytes) -> Response {
    let header_value = HeaderValue::from_str(content_type)
        .unwrap_or_else(|_| HeaderValue::from_static("application/octet-stream"));
    (StatusCode::OK, [(header::CONTENT_TYPE, header_value)], body).into_response()
}

fn map_domain_error(error: anyhow::Error) -> HttpError {
    let (kind, detail) = littlemice::parse_domain_error(error);
    match kind {
        "not_found" => HttpError::not_found(detail),
        "bad_request" => HttpError::bad_request(detail),
        "forbidden" => HttpError::forbidden(detail),
        "gone" => HttpError::new(StatusCode::GONE, detail),
        _ => HttpError::internal_error(detail),
    }
}

fn map_domain_error_response(error: anyhow::Error) -> Response {
    map_domain_error(error).into_response()
}
