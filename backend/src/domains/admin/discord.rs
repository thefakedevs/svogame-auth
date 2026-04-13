use axum::Json;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::json;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::app::auth::require_human_superuser;
use crate::app::http::{HttpError, HttpResult, ProblemResponse};
use crate::app::state::AppStateExtractor;
use crate::services::audit::{
    ACTION_ADMIN_DISCORD_BROADCAST_CREATED, ACTION_ADMIN_DISCORD_NOTIFICATION_SENT,
    write_audit_log,
};
use crate::services::discord_notifications::{self, BroadcastProgress};

#[derive(Debug, Deserialize, ToSchema)]
pub struct SendDiscordNotificationRequest {
    #[serde(rename = "userId")]
    pub user_id: String,
    pub message: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateDiscordBroadcastRequest {
    pub message: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DiscordDeliveryResponse {
    pub id: String,
    #[serde(rename = "broadcastId")]
    pub broadcast_id: Option<String>,
    #[serde(rename = "userId")]
    pub user_id: String,
    #[serde(rename = "templateKey")]
    pub template_key: String,
    pub message: String,
    pub status: String,
    #[serde(rename = "errorMessage")]
    pub error_message: Option<String>,
    #[serde(rename = "attemptCount")]
    pub attempt_count: i32,
    #[serde(rename = "createdAt")]
    pub created_at: chrono::DateTime<chrono::Utc>,
    #[serde(rename = "updatedAt")]
    pub updated_at: chrono::DateTime<chrono::Utc>,
    #[serde(rename = "deliveredAt")]
    pub delivered_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(rename = "finishedAt")]
    pub finished_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DiscordBroadcastResponse {
    pub id: String,
    #[serde(rename = "templateKey")]
    pub template_key: String,
    pub message: String,
    pub status: String,
    #[serde(rename = "totalCount")]
    pub total_count: i64,
    #[serde(rename = "processedCount")]
    pub processed_count: u64,
    #[serde(rename = "deliveredCount")]
    pub delivered_count: u64,
    #[serde(rename = "timeoutCount")]
    pub timeout_count: u64,
    #[serde(rename = "forbiddenCount")]
    pub forbidden_count: u64,
    #[serde(rename = "errorCount")]
    pub error_count: u64,
    #[serde(rename = "createdAt")]
    pub created_at: chrono::DateTime<chrono::Utc>,
    #[serde(rename = "updatedAt")]
    pub updated_at: chrono::DateTime<chrono::Utc>,
    #[serde(rename = "startedAt")]
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(rename = "finishedAt")]
    pub finished_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[utoipa::path(
    post,
    path = "/api/admin/discord/notify",
    request_body = SendDiscordNotificationRequest,
    responses(
        (status = 200, description = "Discord notification delivery result.", body = DiscordDeliveryResponse),
        (status = 400, description = "Invalid payload.", body = ProblemResponse),
        (status = 401, description = "Missing bearer token.", body = ProblemResponse),
        (status = 403, description = "Superuser permissions required.", body = ProblemResponse),
        (status = 404, description = "User not found.", body = ProblemResponse),
        (status = 503, description = "Discord bot is not configured.", body = ProblemResponse)
    ),
    security(("bearer_auth" = [])),
    tag = "admin"
)]
pub async fn send_notification(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Json(body): Json<SendDiscordNotificationRequest>,
) -> HttpResult<Json<DiscordDeliveryResponse>> {
    let message = body.message.trim().to_string();
    if message.is_empty() {
        return Err(HttpError::bad_request("Message is required"));
    }

    let user_id = parse_uuid(&body.user_id, "Invalid user ID")?;
    let state = state.read().await;
    let admin = require_human_superuser(&headers, &state).await?;
    let delivery = discord_notifications::create_single_delivery(
        &state.db,
        Some(admin.id),
        user_id,
        message,
    )
    .await
    .map_err(map_discord_error)?;
    let delivery = discord_notifications::send_delivery_now(&state.db, &state.config.discord, delivery.id)
        .await
        .map_err(map_discord_error)?;

    write_audit_log(
        &state.db,
        ACTION_ADMIN_DISCORD_NOTIFICATION_SENT,
        Some(admin.id),
        Some(user_id),
        None,
        Some(json!({
            "deliveryId": delivery.id,
            "status": delivery.status
        })),
    )
    .await
    .map_err(|error| HttpError::internal_error(format!("Failed to write audit log: {error}")))?;

    Ok(Json(map_delivery(delivery)))
}

#[utoipa::path(
    post,
    path = "/api/admin/discord/broadcasts",
    request_body = CreateDiscordBroadcastRequest,
    responses(
        (status = 200, description = "Broadcast created.", body = DiscordBroadcastResponse),
        (status = 400, description = "Invalid payload.", body = ProblemResponse),
        (status = 401, description = "Missing bearer token.", body = ProblemResponse),
        (status = 403, description = "Superuser permissions required.", body = ProblemResponse)
    ),
    security(("bearer_auth" = [])),
    tag = "admin"
)]
pub async fn create_broadcast(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Json(body): Json<CreateDiscordBroadcastRequest>,
) -> HttpResult<Json<DiscordBroadcastResponse>> {
    let message = body.message.trim().to_string();
    if message.is_empty() {
        return Err(HttpError::bad_request("Message is required"));
    }

    let state = state.read().await;
    let admin = require_human_superuser(&headers, &state).await?;
    let broadcast = discord_notifications::create_broadcast(&state.db, admin.id, message)
        .await
        .map_err(map_discord_error)?;
    let progress = discord_notifications::get_broadcast_progress(&state.db, broadcast.id)
        .await
        .map_err(map_discord_error)?;

    write_audit_log(
        &state.db,
        ACTION_ADMIN_DISCORD_BROADCAST_CREATED,
        Some(admin.id),
        None,
        None,
        Some(json!({
            "broadcastId": broadcast.id,
            "totalCount": broadcast.total_count
        })),
    )
    .await
    .map_err(|error| HttpError::internal_error(format!("Failed to write audit log: {error}")))?;

    Ok(Json(map_broadcast(broadcast, progress)))
}

#[utoipa::path(
    get,
    path = "/api/admin/discord/broadcasts",
    responses(
        (status = 200, description = "List Discord broadcasts.", body = [DiscordBroadcastResponse]),
        (status = 401, description = "Missing bearer token.", body = ProblemResponse),
        (status = 403, description = "Superuser permissions required.", body = ProblemResponse)
    ),
    security(("bearer_auth" = [])),
    tag = "admin"
)]
pub async fn list_broadcasts(
    State(state): AppStateExtractor,
    headers: HeaderMap,
) -> HttpResult<Json<Vec<DiscordBroadcastResponse>>> {
    let state = state.read().await;
    require_human_superuser(&headers, &state).await?;
    let broadcasts = discord_notifications::list_broadcasts(&state.db)
        .await
        .map_err(map_discord_error)?;

    let mut response = Vec::with_capacity(broadcasts.len());
    for broadcast in broadcasts {
        let progress = discord_notifications::get_broadcast_progress(&state.db, broadcast.id)
            .await
            .map_err(map_discord_error)?;
        response.push(map_broadcast(broadcast, progress));
    }

    Ok(Json(response))
}

#[utoipa::path(
    get,
    path = "/api/admin/discord/broadcasts/{broadcast_id}",
    params(
        ("broadcast_id" = String, Path, description = "Discord broadcast UUID.")
    ),
    responses(
        (status = 200, description = "Get one Discord broadcast.", body = DiscordBroadcastResponse),
        (status = 400, description = "Invalid broadcast ID.", body = ProblemResponse),
        (status = 401, description = "Missing bearer token.", body = ProblemResponse),
        (status = 403, description = "Superuser permissions required.", body = ProblemResponse),
        (status = 404, description = "Broadcast not found.", body = ProblemResponse)
    ),
    security(("bearer_auth" = [])),
    tag = "admin"
)]
pub async fn get_broadcast(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Path(broadcast_id): Path<String>,
) -> HttpResult<Json<DiscordBroadcastResponse>> {
    let broadcast_id = parse_uuid(&broadcast_id, "Invalid broadcast ID")?;
    let state = state.read().await;
    require_human_superuser(&headers, &state).await?;
    let broadcast = discord_notifications::get_broadcast(&state.db, broadcast_id)
        .await
        .map_err(map_discord_error)?;
    let progress = discord_notifications::get_broadcast_progress(&state.db, broadcast.id)
        .await
        .map_err(map_discord_error)?;
    Ok(Json(map_broadcast(broadcast, progress)))
}

fn map_delivery(item: crate::entities::DiscordDeliveryModel) -> DiscordDeliveryResponse {
    DiscordDeliveryResponse {
        id: item.id.to_string(),
        broadcast_id: item.broadcast_id.map(|id| id.to_string()),
        user_id: item.user_id.to_string(),
        template_key: item.template_key,
        message: item.message,
        status: item.status,
        error_message: item.error_message,
        attempt_count: item.attempt_count,
        created_at: item.created_at,
        updated_at: item.updated_at,
        delivered_at: item.delivered_at,
        finished_at: item.finished_at,
    }
}

fn map_broadcast(
    item: crate::entities::DiscordBroadcastModel,
    progress: BroadcastProgress,
) -> DiscordBroadcastResponse {
    let processed_count = progress.delivered_count
        + progress.timeout_count
        + progress.forbidden_count
        + progress.error_count;
    DiscordBroadcastResponse {
        id: item.id.to_string(),
        template_key: item.template_key,
        message: item.message,
        status: item.status,
        total_count: item.total_count,
        processed_count,
        delivered_count: progress.delivered_count,
        timeout_count: progress.timeout_count,
        forbidden_count: progress.forbidden_count,
        error_count: progress.error_count,
        created_at: item.created_at,
        updated_at: item.updated_at,
        started_at: item.started_at,
        finished_at: item.finished_at,
    }
}

fn parse_uuid(value: &str, message: &str) -> HttpResult<Uuid> {
    Uuid::parse_str(value).map_err(|_| HttpError::bad_request(message))
}

fn map_discord_error(error: anyhow::Error) -> HttpError {
    let message = error.to_string();
    if let Some(stripped) = message.strip_prefix("not_found: ") {
        HttpError::not_found(stripped)
    } else if message.contains("bot is not configured") {
        HttpError::new(StatusCode::SERVICE_UNAVAILABLE, message)
    } else {
        HttpError::internal_error(message)
    }
}
