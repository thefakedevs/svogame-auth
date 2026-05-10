use anyhow::{Result, anyhow, bail};
use aws_sdk_s3::error::ProvideErrorMetadata;
use aws_sdk_s3::primitives::ByteStream;
use axum::body::Bytes;
use chrono::{Duration, Utc};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait,
    PaginatorTrait, QueryFilter, QueryOrder,
};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::app::config::{AppConfig, LittlemiceConfig};
use crate::entities::{
    LittlemiceCheck, LittlemiceCheckActiveModel, LittlemiceCheckColumn, LittlemiceCheckModel,
};
use crate::services::discord;
use crate::services::service_tokens::AuthenticatedServiceToken;

pub const STATUS_PENDING: &str = "pending";
pub const STATUS_PASSED: &str = "passed";
pub const STATUS_FAILED_TIMEOUT: &str = "failed_timeout";
pub const STATUS_FAILED_CLIENT_ERROR: &str = "failed_client_error";

pub const FAILURE_REASON_PUSH_TIMEOUT: &str = "push_timeout";
pub const FAILURE_REASON_CLIENT_ERROR: &str = "client_error";

const S3_PREFIX: &str = "littlemice";

#[derive(Clone, Debug)]
pub struct CreateLittlemiceCheckResult {
    pub model: LittlemiceCheckModel,
    pub push_url: String,
}

#[derive(Clone, Debug)]
pub struct PushLittlemicePayload {
    pub screenshot_bytes: Vec<u8>,
    pub screenshot_content_type: String,
    pub screenshot2_bytes: Option<Vec<u8>>,
    pub screenshot2_content_type: Option<String>,
    pub log_bytes: Option<Vec<u8>>,
    pub log_content_type: Option<String>,
    pub client_info_text: Option<String>,
}

#[derive(Clone, Debug)]
pub struct FailLittlemiceCheckPayload {
    pub error_message: String,
    pub stacktrace: Option<String>,
}

#[derive(Clone, Debug)]
pub struct LittlemiceListQuery {
    pub player_uuid: Option<Uuid>,
    pub page: u64,
    pub per_page: u64,
}

#[derive(Clone, Debug)]
pub struct LittlemiceListResult {
    pub items: Vec<LittlemiceCheckModel>,
    pub total: u64,
}

pub async fn create_check(
    db: &DatabaseConnection,
    config: &AppConfig,
    service: &AuthenticatedServiceToken,
    player_uuid: Uuid,
) -> Result<CreateLittlemiceCheckResult> {
    let push_secret = generate_push_secret();
    let now = Utc::now();
    let expires_at = now + Duration::seconds(config.littlemice.push_ttl_seconds);
    let model = LittlemiceCheckActiveModel {
        id: Set(Uuid::new_v4()),
        player_uuid: Set(player_uuid),
        service_token_id: Set(service.id),
        service_system_name: Set(service.system_name.clone()),
        status: Set(STATUS_PENDING.to_string()),
        failure_reason: Set(None),
        push_token_hash: Set(Some(hash_push_secret(&push_secret, config))),
        push_token_expires_at: Set(expires_at),
        requested_at: Set(now),
        received_at: Set(None),
        completed_at: Set(None),
        screenshot_s3_key: Set(None),
        screenshot_content_type: Set(None),
        screenshot_size_bytes: Set(None),
        screenshot2_s3_key: Set(None),
        screenshot2_content_type: Set(None),
        screenshot2_size_bytes: Set(None),
        log_s3_key: Set(None),
        log_content_type: Set(None),
        log_size_bytes: Set(None),
        client_info_text: Set(None),
        client_info_size_bytes: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
    }
    .insert(db)
    .await?;

    Ok(CreateLittlemiceCheckResult {
        push_url: format!(
            "{}/api/littlemice/push/{}",
            config.littlemice.public_base_url,
            push_secret
        ),
        model,
    })
}

pub async fn accept_push(
    db: &DatabaseConnection,
    s3: &aws_sdk_s3::Client,
    config: &AppConfig,
    push_secret: &str,
    payload: PushLittlemicePayload,
) -> Result<LittlemiceCheckModel> {
    validate_push_payload(&payload, &config.littlemice)?;
    let token_hash = hash_push_secret(push_secret, config);
    let model = LittlemiceCheck::find()
        .filter(LittlemiceCheckColumn::PushTokenHash.eq(token_hash))
        .one(db)
        .await?
        .ok_or_else(|| anyhow!("not_found: Littlemice push token not found"))?;

    if model.status != STATUS_PENDING {
        bail!("forbidden: Littlemice check is no longer pending");
    }
    if model.push_token_expires_at <= Utc::now() {
        bail!("gone: Littlemice push token expired");
    }

    let screenshot_key = screenshot_key(model.id);
    s3.put_object()
        .bucket(&config.s3.bucket)
        .key(&screenshot_key)
        .content_type(&payload.screenshot_content_type)
        .body(ByteStream::from(payload.screenshot_bytes.clone()))
        .send()
        .await?;

    let mut stored_screenshot2_key = None;
    if let Some(screenshot2_bytes) = payload.screenshot2_bytes.clone() {
        let key = screenshot2_key(model.id);
        s3.put_object()
            .bucket(&config.s3.bucket)
            .key(&key)
            .content_type(
                payload
                    .screenshot2_content_type
                    .as_deref()
                    .unwrap_or("application/octet-stream"),
            )
            .body(ByteStream::from(screenshot2_bytes))
            .send()
            .await?;
        stored_screenshot2_key = Some(key);
    }

    let mut stored_log_key = None;
    if let Some(log_bytes) = payload.log_bytes.clone() {
        let key = log_key(model.id);
        s3.put_object()
            .bucket(&config.s3.bucket)
            .key(&key)
            .content_type(
                payload
                    .log_content_type
                    .as_deref()
                    .unwrap_or("text/plain; charset=utf-8"),
            )
            .body(ByteStream::from(log_bytes))
            .send()
            .await?;
        stored_log_key = Some(key);
    }

    let now = Utc::now();
    let mut active: LittlemiceCheckActiveModel = model.into();
    active.status = Set(STATUS_PASSED.to_string());
    active.failure_reason = Set(None);
    active.push_token_hash = Set(None);
    active.received_at = Set(Some(now));
    active.completed_at = Set(Some(now));
    active.screenshot_s3_key = Set(Some(screenshot_key));
    active.screenshot_content_type = Set(Some(payload.screenshot_content_type));
    active.screenshot_size_bytes = Set(Some(payload.screenshot_bytes.len() as i64));
    active.screenshot2_s3_key = Set(stored_screenshot2_key);
    active.screenshot2_content_type = Set(payload.screenshot2_content_type);
    active.screenshot2_size_bytes =
        Set(payload.screenshot2_bytes.map(|bytes| bytes.len() as i64));
    active.log_s3_key = Set(stored_log_key);
    active.log_content_type = Set(payload.log_content_type);
    active.log_size_bytes = Set(payload.log_bytes.map(|bytes| bytes.len() as i64));
    active.client_info_size_bytes = Set(
        payload
            .client_info_text
            .as_ref()
            .map(|value| value.as_bytes().len() as i64),
    );
    active.client_info_text = Set(payload.client_info_text);
    active.updated_at = Set(now);
    Ok(active.update(db).await?)
}

pub async fn get_check(db: &DatabaseConnection, check_id: Uuid) -> Result<LittlemiceCheckModel> {
    LittlemiceCheck::find_by_id(check_id)
        .one(db)
        .await?
        .ok_or_else(|| anyhow!("not_found: Littlemice check not found"))
}

pub async fn fail_check(
    db: &DatabaseConnection,
    config: &AppConfig,
    service: &AuthenticatedServiceToken,
    check_id: Uuid,
    payload: FailLittlemiceCheckPayload,
) -> Result<LittlemiceCheckModel> {
    let model = get_check(db, check_id).await?;
    if model.service_system_name != service.system_name {
        bail!("forbidden: Service token cannot access checks created by another service");
    }
    if model.status != STATUS_PENDING {
        bail!("forbidden: Littlemice check is no longer pending");
    }

    let client_info_text = format_client_error_text(&payload, config)?;
    let now = Utc::now();
    let mut active: LittlemiceCheckActiveModel = model.into();
    active.status = Set(STATUS_FAILED_CLIENT_ERROR.to_string());
    active.failure_reason = Set(Some(FAILURE_REASON_CLIENT_ERROR.to_string()));
    active.push_token_hash = Set(None);
    active.client_info_size_bytes = Set(Some(client_info_text.as_bytes().len() as i64));
    active.client_info_text = Set(Some(client_info_text));
    active.completed_at = Set(Some(now));
    active.updated_at = Set(now);
    active.update(db).await.map_err(Into::into)
}

pub async fn list_checks(
    db: &DatabaseConnection,
    query: LittlemiceListQuery,
) -> Result<LittlemiceListResult> {
    let mut finder = LittlemiceCheck::find().order_by_desc(LittlemiceCheckColumn::RequestedAt);
    if let Some(player_uuid) = query.player_uuid {
        finder = finder.filter(LittlemiceCheckColumn::PlayerUuid.eq(player_uuid));
    }
    let paginator = finder.paginate(db, query.per_page);
    let total = paginator.num_items().await?;
    let items = paginator.fetch_page(query.page.saturating_sub(1)).await?;
    Ok(LittlemiceListResult { items, total })
}

pub async fn load_s3_object(
    s3: &aws_sdk_s3::Client,
    bucket: &str,
    key: &str,
) -> Result<Option<Bytes>> {
    let object = match s3.get_object().bucket(bucket).key(key).send().await {
        Ok(object) => object,
        Err(error) => {
            let maybe_code = error
                .as_service_error()
                .and_then(|service_error| service_error.code());
            if matches!(maybe_code, Some("NoSuchKey") | Some("NotFound") | Some("404")) {
                return Ok(None);
            }
            return Err(error.into());
        }
    };

    Ok(Some(object.body.collect().await?.into_bytes()))
}

pub async fn expire_due_checks(
    db: &DatabaseConnection,
    config: &AppConfig,
) -> Result<Vec<LittlemiceCheckModel>> {
    let expired = LittlemiceCheck::find()
        .filter(LittlemiceCheckColumn::Status.eq(STATUS_PENDING))
        .filter(LittlemiceCheckColumn::PushTokenExpiresAt.lte(Utc::now()))
        .order_by_asc(LittlemiceCheckColumn::PushTokenExpiresAt)
        .all(db)
        .await?;

    let mut updated = Vec::with_capacity(expired.len());
    for item in expired {
        let now = Utc::now();
        let mut active: LittlemiceCheckActiveModel = item.into();
        active.status = Set(STATUS_FAILED_TIMEOUT.to_string());
        active.failure_reason = Set(Some(FAILURE_REASON_PUSH_TIMEOUT.to_string()));
        active.push_token_hash = Set(None);
        active.completed_at = Set(Some(now));
        active.updated_at = Set(now);
        updated.push(active.update(db).await?);
    }

    for item in &updated {
        notify_timeout(config, item).await;
    }

    Ok(updated)
}

pub async fn cleanup_old_checks(
    db: &DatabaseConnection,
    s3: &aws_sdk_s3::Client,
    config: &AppConfig,
) -> Result<u64> {
    let cutoff = Utc::now() - Duration::days(config.littlemice.retention_days);
    let items = LittlemiceCheck::find()
        .filter(LittlemiceCheckColumn::CreatedAt.lt(cutoff))
        .all(db)
        .await?;

    for item in &items {
        if let Some(key) = item.screenshot_s3_key.as_deref() {
            let _ = s3.delete_object().bucket(&config.s3.bucket).key(key).send().await;
        }
        if let Some(key) = item.log_s3_key.as_deref() {
            let _ = s3.delete_object().bucket(&config.s3.bucket).key(key).send().await;
        }
        LittlemiceCheck::delete_by_id(item.id).exec(db).await?;
    }

    Ok(items.len() as u64)
}

fn validate_push_payload(payload: &PushLittlemicePayload, config: &LittlemiceConfig) -> Result<()> {
    if payload.screenshot_bytes.is_empty() {
        bail!("bad_request: screenshot is required");
    }
    if payload.screenshot_bytes.len() > config.screenshot_max_bytes {
        bail!("bad_request: screenshot exceeds size limit");
    }
    if let Some(screenshot2_bytes) = payload.screenshot2_bytes.as_ref()
        && screenshot2_bytes.len() > config.screenshot_max_bytes
    {
        bail!("bad_request: screenshot2 exceeds size limit");
    }
    if let Some(info_text) = payload.client_info_text.as_ref()
        && info_text.as_bytes().len() > config.info_max_bytes
    {
        bail!("bad_request: client info exceeds size limit");
    }
    Ok(())
}

fn format_client_error_text(
    payload: &FailLittlemiceCheckPayload,
    config: &AppConfig,
) -> Result<String> {
    let mut parts = vec![format!("clientError: {}", payload.error_message.trim())];
    if let Some(stacktrace) = payload
        .stacktrace
        .as_ref()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
    {
        parts.push("stacktrace:".to_string());
        parts.push(stacktrace.to_string());
    }
    let text = parts.join("\n");
    if text.as_bytes().len() > config.littlemice.info_max_bytes {
        bail!("bad_request: client error payload exceeds size limit");
    }
    Ok(text)
}

fn generate_push_secret() -> String {
    format!("lm_{}_{}", Uuid::new_v4(), Uuid::new_v4().simple())
}

fn hash_push_secret(secret: &str, config: &AppConfig) -> String {
    let mut hasher = Sha256::new();
    hasher.update(config.jwt_secret.as_bytes());
    hasher.update(b":littlemice:");
    hasher.update(secret.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn screenshot_key(check_id: Uuid) -> String {
    format!("{}/{}/screenshot", S3_PREFIX, check_id)
}

fn screenshot2_key(check_id: Uuid) -> String {
    format!("{}/{}/screenshot2", S3_PREFIX, check_id)
}

fn log_key(check_id: Uuid) -> String {
    format!("{}/{}/log", S3_PREFIX, check_id)
}

pub fn truncate_log_bytes(log_bytes: Vec<u8>, max_bytes: usize) -> Vec<u8> {
    if log_bytes.len() <= max_bytes {
        log_bytes
    } else {
        log_bytes[log_bytes.len() - max_bytes..].to_vec()
    }
}

async fn notify_timeout(config: &AppConfig, item: &LittlemiceCheckModel) {
    let Some(channel_id) = config.littlemice.discord_log_channel_id.as_deref() else {
        return;
    };
    let Some(bot_token) = config.discord.bot_token.as_deref() else {
        return;
    };

    let message = format!(
        "Littlemice timeout: player `{}` did not upload snapshot in time. Check `{}` requested by service `{}` expired at `{}`.",
        item.player_uuid,
        item.id,
        item.service_system_name,
        item.push_token_expires_at.to_rfc3339()
    );
    if let Err(error) = discord::send_channel_message(&config.discord, bot_token, channel_id, &message).await
    {
        tracing::warn!("Failed to send littlemice timeout log to Discord: {error:?}");
    }
}

pub fn parse_domain_error(error: anyhow::Error) -> (&'static str, String) {
    let message = error.to_string();
    if let Some(detail) = message.strip_prefix("not_found: ") {
        ("not_found", detail.to_string())
    } else if let Some(detail) = message.strip_prefix("gone: ") {
        ("gone", detail.to_string())
    } else if let Some(detail) = message.strip_prefix("forbidden: ") {
        ("forbidden", detail.to_string())
    } else if let Some(detail) = message.strip_prefix("bad_request: ") {
        ("bad_request", detail.to_string())
    } else {
        ("internal", message)
    }
}
