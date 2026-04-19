use std::sync::Arc;

use anyhow::{Result, anyhow};
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use reqwest::StatusCode;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, DatabaseConnection,
    EntityTrait, QueryFilter, QueryOrder,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::app::config::{EmailConfig, EmailProviderKind, XyecocMailConfig};
use crate::entities::{
    EmailDelivery, EmailDeliveryActiveModel, EmailDeliveryColumn, EmailDeliveryModel,
    ShopOrderModel, ShopReceiptModel, User,
};
use crate::services::email_templates;

pub const DELIVERY_STATUS_PENDING: &str = "pending";
pub const DELIVERY_STATUS_PROCESSING: &str = "processing";
pub const DELIVERY_STATUS_DELIVERED: &str = "delivered";
pub const DELIVERY_STATUS_TIMEOUT: &str = "timeout";
pub const DELIVERY_STATUS_FORBIDDEN: &str = "forbidden";
pub const DELIVERY_STATUS_ERROR: &str = "error";

#[derive(Clone)]
pub struct EmailRuntime {
    xyecoc: Option<Arc<XyecocMailClient>>,
}

impl EmailRuntime {
    pub fn new(config: &EmailConfig) -> Self {
        let xyecoc = config
            .xyecoc
            .clone()
            .map(XyecocMailClient::new)
            .map(Arc::new);
        Self { xyecoc }
    }

    fn xyecoc(&self) -> Result<Arc<XyecocMailClient>> {
        self.xyecoc
            .clone()
            .ok_or_else(|| anyhow!("Xyecoc mail client is not configured"))
    }
}

#[derive(Default)]
struct XyecocAuthState {
    token: Option<String>,
}

struct XyecocMailClient {
    http: reqwest::Client,
    config: XyecocMailConfig,
    auth: Mutex<XyecocAuthState>,
}

#[derive(Debug, Serialize)]
struct XyecocRequest<'a> {
    service: &'a str,
    action: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    token: Option<&'a str>,
    #[serde(rename = "currentLang")]
    current_lang: &'a str,
    data: Value,
}

#[derive(Debug, Deserialize)]
struct XyecocResponse {
    #[serde(default)]
    status: XyecocStatus,
    #[serde(default)]
    message: String,
    #[serde(default)]
    data: Value,
}

#[derive(Debug, Default, PartialEq, Eq)]
struct XyecocStatus(i64);

impl<'de> Deserialize<'de> for XyecocStatus {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        Ok(Self(match value {
            Value::Number(number) => number.as_i64().unwrap_or_default(),
            Value::String(value) => value.trim().parse::<i64>().unwrap_or_default(),
            Value::Bool(true) => 1,
            _ => 0,
        }))
    }
}

#[derive(Clone, Debug)]
pub struct EmailAttachment {
    pub url: String,
    pub filename: String,
    pub content_type: String,
}

struct PreparedAttachment {
    payload: Value,
    inline_src: String,
}

impl XyecocMailClient {
    fn new(config: XyecocMailConfig) -> Self {
        Self {
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_millis(config.http_timeout_ms))
                .build()
                .expect("valid reqwest client"),
            config,
            auth: Mutex::new(XyecocAuthState::default()),
        }
    }

    async fn send_email(
        &self,
        recipient: &str,
        subject: &str,
        html_body: &str,
        attachment: Option<EmailAttachment>,
    ) -> std::result::Result<Option<String>, EmailApiError> {
        let token = self.ensure_authenticated(false).await?;
        match self
            .send_email_with_token(&token, recipient, subject, html_body, attachment.clone())
            .await
        {
            Ok(message_id) => Ok(message_id),
            Err(error) if error.is_auth_error() => {
                self.invalidate_auth().await;
                let token = self.ensure_authenticated(true).await?;
                self.send_email_with_token(&token, recipient, subject, html_body, attachment)
                    .await
            }
            Err(error) => Err(error),
        }
    }

    async fn ensure_authenticated(
        &self,
        force_login: bool,
    ) -> std::result::Result<String, EmailApiError> {
        let mut auth = self.auth.lock().await;
        if !force_login && let Some(token) = auth.token.clone() {
            return Ok(token);
        }

        let login = remote_auth_email(&self.config.login, self.config.auth_local_part_only);
        let response = self
            .post(&XyecocRequest {
                service: "account",
                action: "authorization",
                token: None,
                current_lang: "ru",
                data: json!({
                    "email": login,
                    "password": self.config.password,
                }),
            })
            .await?;
        if response.status.0 != 1 {
            return Err(EmailApiError::Rejected(format!(
                "Xyecoc login failed: {}",
                response.message
            )));
        }
        let token = extract_token(&response).ok_or_else(|| {
            EmailApiError::Rejected("Xyecoc login response has no token".to_string())
        })?;
        auth.token = Some(token.clone());
        Ok(token)
    }

    async fn invalidate_auth(&self) {
        let mut auth = self.auth.lock().await;
        auth.token = None;
    }

    async fn send_email_with_token(
        &self,
        token: &str,
        recipient: &str,
        subject: &str,
        html_body: &str,
        attachment: Option<EmailAttachment>,
    ) -> std::result::Result<Option<String>, EmailApiError> {
        let (message, attaches) = match attachment {
            Some(attachment) => {
                let prepared = self.download_attachment_payload(attachment).await?;
                (
                    html_body.replace("__SVO_RECEIPT_IMAGE_SRC__", &prepared.inline_src),
                    vec![prepared.payload],
                )
            }
            None => (
                html_body.replace("__SVO_RECEIPT_IMAGE_SRC__", ""),
                Vec::new(),
            ),
        };
        let response = self
            .post(&XyecocRequest {
                service: "mail",
                action: "message-new",
                token: Some(token),
                current_lang: &self.config.current_lang,
                data: json!({
                    "subject": subject,
                    "message": message,
                    "users": recipient,
                    "attaches": attaches,
                }),
            })
            .await?;
        if mail_mutation_ok(&response) {
            return Ok(extract_message_id(&response));
        }
        Err(EmailApiError::Rejected(format!(
            "Xyecoc message-new rejected: {}",
            response.message
        )))
    }

    async fn download_attachment_payload(
        &self,
        attachment: EmailAttachment,
    ) -> std::result::Result<PreparedAttachment, EmailApiError> {
        let response = self
            .http
            .get(&attachment.url)
            .send()
            .await
            .map_err(EmailApiError::Transport)?;
        let status = response.status();
        if !status.is_success() {
            return Err(EmailApiError::Status {
                operation: "download attachment".to_string(),
                status,
                body: response.text().await.unwrap_or_default(),
            });
        }
        let bytes = response.bytes().await.map_err(EmailApiError::Transport)?;
        let encoded = BASE64.encode(bytes);
        Ok(PreparedAttachment {
            inline_src: format!("data:{};base64,{}", attachment.content_type, encoded),
            payload: json!({
                "filename": attachment.filename,
                "content": encoded,
            }),
        })
    }

    async fn post(
        &self,
        request: &XyecocRequest<'_>,
    ) -> std::result::Result<XyecocResponse, EmailApiError> {
        let response = self
            .http
            .post(format!("{}/request", self.config.api_base_url))
            .header(reqwest::header::ACCEPT, "application/json")
            .header(
                reqwest::header::CONTENT_TYPE,
                "application/json;charset=utf-8",
            )
            .json(request)
            .send()
            .await
            .map_err(EmailApiError::Transport)?;
        let status = response.status();
        let body = response.text().await.map_err(EmailApiError::Transport)?;
        if !status.is_success() {
            return Err(EmailApiError::Status {
                operation: request.action.to_string(),
                status,
                body,
            });
        }
        serde_json::from_str(&body).map_err(EmailApiError::Serde)
    }
}

#[derive(Debug)]
enum EmailApiError {
    Status {
        operation: String,
        status: StatusCode,
        body: String,
    },
    Transport(reqwest::Error),
    Serde(serde_json::Error),
    Rejected(String),
}

impl EmailApiError {
    fn is_auth_error(&self) -> bool {
        matches!(
            self,
            Self::Status {
                status: StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN,
                ..
            }
        )
    }
}

impl std::fmt::Display for EmailApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Status {
                operation,
                status,
                body,
            } => write!(f, "Xyecoc {operation} failed with status {status}: {body}"),
            Self::Transport(error) => write!(f, "Xyecoc transport error: {error}"),
            Self::Serde(error) => write!(f, "Xyecoc parse error: {error}"),
            Self::Rejected(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for EmailApiError {}

pub async fn queue_shop_receipt_email(
    db: &impl ConnectionTrait,
    order: &ShopOrderModel,
    receipt: &ShopReceiptModel,
) -> Result<Option<EmailDeliveryModel>> {
    let Some(user) = User::find_by_id(order.user_id).one(db).await? else {
        return Ok(None);
    };
    let Some(recipient_email) = user.email.filter(|email| !email.trim().is_empty()) else {
        return Ok(None);
    };
    let Some(print_url) = receipt.print_url.clone() else {
        return Ok(None);
    };

    let template = email_templates::shop_receipt(order, receipt);
    create_delivery(
        db,
        NewEmailDelivery {
            user_id: Some(order.user_id),
            requested_by_user_id: None,
            recipient_email,
            template_key: email_templates::TEMPLATE_SHOP_RECEIPT.to_string(),
            subject: template.subject,
            html_body: template.html_body,
            metadata: json!({
                "source": "receipts.completed",
                "orderId": order.id,
                "receiptId": receipt.id,
                "receiptUuid": receipt.receipt_uuid,
                "printUrl": print_url,
            }),
            attachment: Some(EmailAttachment {
                url: print_url,
                filename: format!(
                    "svo-receipt-{}.png",
                    receipt.receipt_uuid.as_deref().unwrap_or("receipt")
                ),
                content_type: "image/png".to_string(),
            }),
        },
    )
    .await
    .map(Some)
}

pub async fn create_test_receipt_delivery(
    db: &impl ConnectionTrait,
    requested_by_user_id: Uuid,
    recipient_email: String,
    receipt_print_url: String,
) -> Result<EmailDeliveryModel> {
    let template = email_templates::test_shop_receipt(&receipt_print_url);
    create_delivery(
        db,
        NewEmailDelivery {
            user_id: None,
            requested_by_user_id: Some(requested_by_user_id),
            recipient_email,
            template_key: "shop.receipt_test".to_string(),
            subject: template.subject,
            html_body: template.html_body,
            metadata: json!({
                "source": "admin.email.test_receipt",
                "printUrl": receipt_print_url,
            }),
            attachment: Some(EmailAttachment {
                url: receipt_print_url,
                filename: "svo-test-receipt.png".to_string(),
                content_type: "image/png".to_string(),
            }),
        },
    )
    .await
}

pub async fn process_next_delivery(
    db: &DatabaseConnection,
    config: &EmailConfig,
    runtime: &EmailRuntime,
) -> Result<bool> {
    if !config.enabled {
        return Ok(false);
    }
    let Some(delivery) = EmailDelivery::find()
        .filter(EmailDeliveryColumn::Status.eq(DELIVERY_STATUS_PENDING))
        .filter(EmailDeliveryColumn::UpdatedAt.lte(chrono::Utc::now()))
        .order_by_asc(EmailDeliveryColumn::CreatedAt)
        .one(db)
        .await?
    else {
        return Ok(false);
    };
    attempt_delivery(db, config, runtime, delivery).await?;
    Ok(true)
}

pub async fn send_delivery_now(
    db: &DatabaseConnection,
    config: &EmailConfig,
    runtime: &EmailRuntime,
    delivery_id: Uuid,
) -> Result<EmailDeliveryModel> {
    let delivery = EmailDelivery::find_by_id(delivery_id)
        .one(db)
        .await?
        .ok_or_else(|| anyhow!("not_found: Email delivery not found"))?;
    attempt_delivery(db, config, runtime, delivery).await
}

async fn attempt_delivery(
    db: &DatabaseConnection,
    config: &EmailConfig,
    runtime: &EmailRuntime,
    delivery: EmailDeliveryModel,
) -> Result<EmailDeliveryModel> {
    let processing = mark_processing(db, &delivery).await?;
    let attachment = processing
        .attachment_url
        .clone()
        .map(|url| EmailAttachment {
            url,
            filename: processing
                .attachment_filename
                .clone()
                .unwrap_or_else(|| "attachment.bin".to_string()),
            content_type: processing
                .attachment_content_type
                .clone()
                .unwrap_or_else(|| "application/octet-stream".to_string()),
        });

    let outcome = match config.provider {
        EmailProviderKind::Xyecoc => {
            runtime
                .xyecoc()?
                .send_email(
                    &processing.recipient_email,
                    &processing.subject,
                    &processing.html_body,
                    attachment,
                )
                .await
        }
    };

    match outcome {
        Ok(provider_message_id) => mark_delivered(db, &processing, provider_message_id).await,
        Err(error) => {
            let (status, description) = map_error_to_status(error);
            mark_failed(db, config, &processing, status, description).await
        }
    }
}

async fn create_delivery(
    db: &impl ConnectionTrait,
    input: NewEmailDelivery,
) -> Result<EmailDeliveryModel> {
    let now = chrono::Utc::now();
    let attachment_url = input.attachment.as_ref().map(|item| item.url.clone());
    let attachment_filename = input.attachment.as_ref().map(|item| item.filename.clone());
    let attachment_content_type = input
        .attachment
        .as_ref()
        .map(|item| item.content_type.clone());
    EmailDeliveryActiveModel {
        id: Set(Uuid::new_v4()),
        user_id: Set(input.user_id),
        requested_by_user_id: Set(input.requested_by_user_id),
        recipient_email: Set(input.recipient_email),
        template_key: Set(input.template_key),
        subject: Set(input.subject),
        html_body: Set(input.html_body),
        status: Set(DELIVERY_STATUS_PENDING.to_string()),
        error_message: Set(None),
        provider_message_id: Set(None),
        attempt_count: Set(0),
        metadata: Set(input.metadata.to_string()),
        attachment_url: Set(attachment_url),
        attachment_filename: Set(attachment_filename),
        attachment_content_type: Set(attachment_content_type),
        created_at: Set(now),
        updated_at: Set(now),
        last_attempt_at: Set(None),
        delivered_at: Set(None),
        finished_at: Set(None),
    }
    .insert(db)
    .await
    .map_err(Into::into)
}

async fn mark_processing(
    db: &DatabaseConnection,
    delivery: &EmailDeliveryModel,
) -> Result<EmailDeliveryModel> {
    let now = chrono::Utc::now();
    let mut active: EmailDeliveryActiveModel = delivery.clone().into();
    active.status = Set(DELIVERY_STATUS_PROCESSING.to_string());
    active.attempt_count = Set(delivery.attempt_count + 1);
    active.last_attempt_at = Set(Some(now));
    active.error_message = Set(None);
    active.updated_at = Set(now);
    active.update(db).await.map_err(Into::into)
}

async fn mark_delivered(
    db: &DatabaseConnection,
    delivery: &EmailDeliveryModel,
    provider_message_id: Option<String>,
) -> Result<EmailDeliveryModel> {
    let now = chrono::Utc::now();
    let mut active: EmailDeliveryActiveModel = delivery.clone().into();
    active.status = Set(DELIVERY_STATUS_DELIVERED.to_string());
    active.error_message = Set(None);
    active.provider_message_id = Set(provider_message_id);
    active.delivered_at = Set(Some(now));
    active.finished_at = Set(Some(now));
    active.updated_at = Set(now);
    active.update(db).await.map_err(Into::into)
}

async fn mark_failed(
    db: &DatabaseConnection,
    config: &EmailConfig,
    delivery: &EmailDeliveryModel,
    status: &'static str,
    error_message: String,
) -> Result<EmailDeliveryModel> {
    let now = chrono::Utc::now();
    let deadline = delivery.created_at + chrono::Duration::seconds(config.failure_after_seconds);
    let should_retry = status != DELIVERY_STATUS_FORBIDDEN && now < deadline;
    let final_status = if should_retry {
        DELIVERY_STATUS_PENDING
    } else if now >= deadline {
        DELIVERY_STATUS_ERROR
    } else {
        status
    };
    let mut active: EmailDeliveryActiveModel = delivery.clone().into();
    active.status = Set(final_status.to_string());
    active.error_message = Set(Some(error_message));
    active.finished_at = Set((final_status != DELIVERY_STATUS_PENDING).then_some(now));
    active.updated_at = Set(if should_retry {
        now + chrono::Duration::seconds(config.retry_interval_seconds)
    } else {
        now
    });
    active.update(db).await.map_err(Into::into)
}

fn map_error_to_status(error: EmailApiError) -> (&'static str, String) {
    match error {
        EmailApiError::Transport(error) if error.is_timeout() => (
            DELIVERY_STATUS_TIMEOUT,
            "Xyecoc request timed out".to_string(),
        ),
        EmailApiError::Status {
            status: StatusCode::FORBIDDEN,
            body,
            ..
        } => (DELIVERY_STATUS_FORBIDDEN, body),
        other => (DELIVERY_STATUS_ERROR, other.to_string()),
    }
}

fn extract_token(response: &XyecocResponse) -> Option<String> {
    if let Some(token) = response
        .data
        .as_str()
        .filter(|value| !value.trim().is_empty())
    {
        return Some(token.to_string());
    }
    for value in [
        &response.data,
        response.data.get("user").unwrap_or(&Value::Null),
    ] {
        for key in ["token", "jwt", "access_token"] {
            if let Some(token) = value.get(key).and_then(Value::as_str) {
                if !token.trim().is_empty() {
                    return Some(token.to_string());
                }
            }
        }
    }
    None
}

fn extract_message_id(response: &XyecocResponse) -> Option<String> {
    response
        .data
        .get("id")
        .and_then(Value::as_i64)
        .map(|id| id.to_string())
        .or_else(|| {
            response
                .data
                .get("id")
                .and_then(Value::as_str)
                .map(str::to_string)
        })
}

fn mail_mutation_ok(response: &XyecocResponse) -> bool {
    if response.status.0 == 1 {
        return true;
    }
    let message = response.message.trim().to_lowercase();
    if message == "success" || message == "ok" {
        return true;
    }
    response
        .data
        .get("status")
        .map(parse_status_value)
        .is_some_and(|status| status == 1)
}

fn parse_status_value(value: &Value) -> i64 {
    match value {
        Value::Number(number) => number.as_i64().unwrap_or_default(),
        Value::String(value) => value.trim().parse::<i64>().unwrap_or_default(),
        Value::Bool(true) => 1,
        _ => 0,
    }
}

fn remote_auth_email(login: &str, local_part_only: bool) -> String {
    let login = login.trim();
    if !local_part_only {
        return login.to_string();
    }
    login
        .split_once('@')
        .map(|(local_part, _)| local_part.to_string())
        .unwrap_or_else(|| login.to_string())
}

struct NewEmailDelivery {
    user_id: Option<Uuid>,
    requested_by_user_id: Option<Uuid>,
    recipient_email: String,
    template_key: String,
    subject: String,
    html_body: String,
    metadata: Value,
    attachment: Option<EmailAttachment>,
}
