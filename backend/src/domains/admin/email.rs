use axum::Json;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::json;
use utoipa::ToSchema;

use crate::app::auth::require_human_superuser;
use crate::app::http::{HttpError, HttpResult, ProblemResponse};
use crate::app::state::AppStateExtractor;
use crate::services::audit::write_audit_log;
use crate::services::email;

const ACTION_ADMIN_EMAIL_TEST_RECEIPT_SENT: &str = "admin.email.test_receipt_sent";
const DEFAULT_TEST_RECEIPT_URL: &str =
    "https://lknpd.nalog.ru/api/v1/receipt/753615309230/2003414kju/print";

#[derive(Debug, Deserialize, ToSchema)]
pub struct SendTestReceiptEmailRequest {
    #[serde(rename = "recipientEmail")]
    pub recipient_email: String,
    #[serde(rename = "receiptPrintUrl")]
    pub receipt_print_url: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct EmailDeliveryResponse {
    pub id: String,
    #[serde(rename = "userId")]
    pub user_id: Option<String>,
    #[serde(rename = "recipientEmail")]
    pub recipient_email: String,
    #[serde(rename = "templateKey")]
    pub template_key: String,
    pub subject: String,
    pub status: String,
    #[serde(rename = "errorMessage")]
    pub error_message: Option<String>,
    #[serde(rename = "providerMessageId")]
    pub provider_message_id: Option<String>,
    #[serde(rename = "attemptCount")]
    pub attempt_count: i32,
    #[serde(rename = "attachmentUrl")]
    pub attachment_url: Option<String>,
    #[serde(rename = "createdAt")]
    pub created_at: chrono::DateTime<chrono::Utc>,
    #[serde(rename = "updatedAt")]
    pub updated_at: chrono::DateTime<chrono::Utc>,
    #[serde(rename = "deliveredAt")]
    pub delivered_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(rename = "finishedAt")]
    pub finished_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[utoipa::path(
    post,
    path = "/api/admin/email/test-receipt",
    request_body = SendTestReceiptEmailRequest,
    responses(
        (status = 200, description = "Send a test receipt email through the configured email provider.", body = EmailDeliveryResponse),
        (status = 400, description = "Invalid payload.", body = ProblemResponse),
        (status = 401, description = "Missing bearer token.", body = ProblemResponse),
        (status = 403, description = "Superuser permissions required.", body = ProblemResponse),
        (status = 503, description = "Email delivery is not configured.", body = ProblemResponse)
    ),
    security(("bearer_auth" = [])),
    tag = "admin"
)]
pub async fn send_test_receipt(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Json(body): Json<SendTestReceiptEmailRequest>,
) -> HttpResult<Json<EmailDeliveryResponse>> {
    let recipient_email = body.recipient_email.trim().to_string();
    if recipient_email.is_empty() || !recipient_email.contains('@') {
        return Err(HttpError::bad_request(
            "recipientEmail must be a valid email address",
        ));
    }
    let receipt_print_url = body
        .receipt_print_url
        .unwrap_or_else(|| DEFAULT_TEST_RECEIPT_URL.to_string())
        .trim()
        .to_string();
    if receipt_print_url.is_empty() {
        return Err(HttpError::bad_request("receiptPrintUrl cannot be empty"));
    }

    let state = state.read().await;
    if !state.config.email.enabled {
        return Err(HttpError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "Email delivery is disabled by configuration",
        ));
    }
    let admin = require_human_superuser(&headers, &state).await?;
    let delivery = email::create_test_receipt_delivery(
        &state.db,
        admin.id,
        recipient_email,
        receipt_print_url,
    )
    .await
    .map_err(map_email_error)?;
    let delivery =
        email::send_delivery_now(&state.db, &state.config.email, &state.email, delivery.id)
            .await
            .map_err(map_email_error)?;

    write_audit_log(
        &state.db,
        ACTION_ADMIN_EMAIL_TEST_RECEIPT_SENT,
        Some(admin.id),
        None,
        None,
        Some(json!({
            "deliveryId": delivery.id,
            "status": delivery.status,
            "recipientEmail": delivery.recipient_email,
        })),
    )
    .await
    .map_err(|error| HttpError::internal_error(format!("Failed to write audit log: {error}")))?;

    Ok(Json(map_delivery(delivery)))
}

fn map_delivery(item: crate::entities::EmailDeliveryModel) -> EmailDeliveryResponse {
    EmailDeliveryResponse {
        id: item.id.to_string(),
        user_id: item.user_id.map(|id| id.to_string()),
        recipient_email: item.recipient_email,
        template_key: item.template_key,
        subject: item.subject,
        status: item.status,
        error_message: item.error_message,
        provider_message_id: item.provider_message_id,
        attempt_count: item.attempt_count,
        attachment_url: item.attachment_url,
        created_at: item.created_at,
        updated_at: item.updated_at,
        delivered_at: item.delivered_at,
        finished_at: item.finished_at,
    }
}

fn map_email_error(error: anyhow::Error) -> HttpError {
    let message = error.to_string();
    if let Some(stripped) = message.strip_prefix("not_found: ") {
        HttpError::not_found(stripped)
    } else if message.contains("not configured") || message.contains("disabled") {
        HttpError::new(StatusCode::SERVICE_UNAVAILABLE, message)
    } else {
        HttpError::internal_error(message)
    }
}
