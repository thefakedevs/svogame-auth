use std::sync::Arc;

use anyhow::{Context, Result, anyhow};
use rand::Rng;
use reqwest::StatusCode;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, Condition, ConnectionTrait,
    DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::app::config::{MyTaxConfig, ReceiptProviderKind, ReceiptsConfig};
use crate::entities::{
    AppKv, AppKvActiveModel, ShopOrder, ShopOrderModel, ShopPaymentAttempt,
    ShopPaymentAttemptColumn, ShopReceipt, ShopReceiptActiveModel, ShopReceiptColumn,
    ShopReceiptModel,
};

pub const RECEIPT_STATUS_NOT_REQUIRED: &str = "not_required";
pub const RECEIPT_STATUS_TEST_PAYMENT: &str = "test_payment";
pub const RECEIPT_STATUS_PENDING: &str = "pending";
pub const RECEIPT_STATUS_FORMING: &str = "forming";
pub const RECEIPT_STATUS_COMPLETED: &str = "completed";
pub const RECEIPT_STATUS_FAILED: &str = "failed";

const APP_KV_MYTAX_DEVICE_ID: &str = "mytax.device_id";
const RECEIPT_LOCK_SECONDS: i64 = 300;

#[derive(Clone)]
pub struct ReceiptRuntime {
    mytax: Option<Arc<MyTaxClient>>,
}

impl ReceiptRuntime {
    pub fn new(config: &ReceiptsConfig) -> Self {
        let mytax = config.mytax.clone().map(MyTaxClient::new).map(Arc::new);
        Self { mytax }
    }

    fn mytax(&self) -> Result<Arc<MyTaxClient>> {
        self.mytax
            .clone()
            .ok_or_else(|| anyhow!("MyTax receipt client is not configured"))
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct ShopReceiptView {
    pub id: Uuid,
    pub provider: String,
    pub status: String,
    pub display_status: String,
    pub receipt_uuid: Option<String>,
    pub print_url: Option<String>,
    pub json_url: Option<String>,
    pub failure_problem: Option<String>,
    pub attempt_count: i32,
    pub next_attempt_at: Option<chrono::DateTime<chrono::Utc>>,
    pub deadline_at: Option<chrono::DateTime<chrono::Utc>>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Clone, Debug)]
struct ReceiptSubmission {
    receipt_uuid: String,
    json_url: String,
    print_url: String,
    request_payload: Value,
    response_payload: Value,
}

#[derive(Default)]
struct MyTaxAuthState {
    token: Option<String>,
    refresh_token: Option<String>,
    token_expires_at: Option<chrono::DateTime<chrono::Utc>>,
    inn: Option<String>,
}

struct MyTaxClient {
    http: reqwest::Client,
    config: MyTaxConfig,
    auth: Mutex<MyTaxAuthState>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MyTaxAuthResponse {
    token: String,
    refresh_token: String,
    token_expire_in: String,
    profile: MyTaxProfile,
}

#[derive(Debug, Deserialize)]
struct MyTaxProfile {
    inn: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MyTaxTokenResponse {
    token: String,
    refresh_token: String,
    token_expire_in: String,
}

impl MyTaxClient {
    fn new(config: MyTaxConfig) -> Self {
        Self {
            http: reqwest::Client::builder()
                .connect_timeout(std::time::Duration::from_secs(5))
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .expect("valid reqwest client"),
            config,
            auth: Mutex::new(MyTaxAuthState::default()),
        }
    }

    async fn add_income_for_order(
        &self,
        db: &DatabaseConnection,
        order: &ShopOrderModel,
    ) -> Result<ReceiptSubmission> {
        let token = self.ensure_authenticated(db, false).await?;
        match self.send_income(order, &token).await {
            Ok(receipt) => Ok(receipt),
            Err(error) if error.is_auth_error() => {
                self.invalidate_auth().await;
                let token = self.ensure_authenticated(db, true).await?;
                self.send_income(order, &token).await.map_err(Into::into)
            }
            Err(error) => Err(error.into()),
        }
    }

    async fn ensure_authenticated(
        &self,
        db: &DatabaseConnection,
        force_login: bool,
    ) -> Result<String> {
        let mut auth = self.auth.lock().await;
        if !force_login {
            if let Some(token) = valid_token(&auth) {
                return Ok(token);
            }
            if auth.refresh_token.is_some() {
                match self
                    .refresh_token(db, auth.refresh_token.as_deref().unwrap())
                    .await
                {
                    Ok(refreshed) => {
                        apply_token_response(&mut auth, refreshed)?;
                        if let Some(token) = auth.token.clone() {
                            return Ok(token);
                        }
                    }
                    Err(error) if error.is_auth_error() => {
                        auth.token = None;
                        auth.refresh_token = None;
                        auth.token_expires_at = None;
                    }
                    Err(_) => {}
                }
            }
        }

        let authenticated = self.authenticate(db).await?;
        auth.inn = Some(authenticated.profile.inn.clone());
        auth.token = Some(authenticated.token.clone());
        auth.refresh_token = Some(authenticated.refresh_token);
        auth.token_expires_at = Some(parse_mytax_expiry(&authenticated.token_expire_in)?);
        Ok(authenticated.token)
    }

    async fn invalidate_auth(&self) {
        let mut auth = self.auth.lock().await;
        auth.token = None;
        auth.refresh_token = None;
        auth.token_expires_at = None;
    }

    async fn authenticate(
        &self,
        db: &DatabaseConnection,
    ) -> Result<MyTaxAuthResponse, MyTaxApiError> {
        let device_id = get_or_create_mytax_device_id(db, &self.config.device_prefix)
            .await
            .map_err(MyTaxApiError::Other)?;
        let payload = json!({
            "username": self.config.inn,
            "password": self.config.password,
            "deviceInfo": device_info(&device_id),
        });
        let response = self
            .http
            .post(format!("{}/auth/lkfl", self.config.api_base_url))
            .headers(common_headers())
            .header("Referer", "https://lknpd.nalog.ru/auth/login")
            .json(&payload)
            .send()
            .await
            .map_err(MyTaxApiError::Transport)?;
        parse_mytax_response(response, "authentication").await
    }

    async fn refresh_token(
        &self,
        db: &DatabaseConnection,
        refresh_token: &str,
    ) -> Result<MyTaxTokenResponse, MyTaxApiError> {
        let device_id = get_or_create_mytax_device_id(db, &self.config.device_prefix)
            .await
            .map_err(MyTaxApiError::Other)?;
        let payload = json!({
            "deviceInfo": device_info(&device_id),
            "refreshToken": refresh_token,
        });
        let response = self
            .http
            .post(format!("{}/auth/token", self.config.api_base_url))
            .headers(common_headers())
            .header("Referer", "https://lknpd.nalog.ru/sales")
            .json(&payload)
            .send()
            .await
            .map_err(MyTaxApiError::Transport)?;
        parse_mytax_response(response, "refresh token").await
    }

    async fn send_income(
        &self,
        order: &ShopOrderModel,
        token: &str,
    ) -> Result<ReceiptSubmission, MyTaxApiError> {
        let operation_time =
            current_mytax_time(&self.config.zone_offset).map_err(MyTaxApiError::Other)?;
        let amount = json!(order.total_price_rub);
        let payload = json!({
            "paymentType": "CASH",
            "ignoreMaxTotalIncomeRestriction": false,
            "client": {
                "contactPhone": null,
                "displayName": null,
                "inn": null,
                "incomeType": "FROM_INDIVIDUAL",
            },
            "operationTime": operation_time,
            "requestTime": operation_time,
            "services": [{
                "name": order.product_name,
                "quantity": 1,
                "amount": amount,
            }],
            "totalAmount": amount,
        });

        let response = self
            .http
            .post(format!("{}/income", self.config.api_base_url))
            .headers(common_headers())
            .header("Referer", "https://lknpd.nalog.ru/sales/create")
            .bearer_auth(token)
            .json(&payload)
            .send()
            .await
            .map_err(MyTaxApiError::Transport)?;

        let status = response.status();
        let body = response.text().await.map_err(MyTaxApiError::Transport)?;
        if !status.is_success() {
            return Err(MyTaxApiError::Status {
                operation: "add income",
                status,
                body,
            });
        }
        let response_payload: Value = serde_json::from_str(&body).map_err(MyTaxApiError::Serde)?;
        let receipt_uuid = response_payload
            .get("approvedReceiptUuid")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| {
                MyTaxApiError::Other(anyhow!("MyTax response is missing approvedReceiptUuid"))
            })?
            .to_string();
        let inn = {
            let auth = self.auth.lock().await;
            auth.inn.clone().unwrap_or_else(|| self.config.inn.clone())
        };
        Ok(ReceiptSubmission {
            receipt_uuid: receipt_uuid.clone(),
            json_url: format!(
                "{}/receipt/{}/{}/json",
                self.config.api_base_url, inn, receipt_uuid
            ),
            print_url: format!(
                "{}/receipt/{}/{}/print",
                self.config.api_base_url, inn, receipt_uuid
            ),
            request_payload: payload,
            response_payload,
        })
    }
}

#[derive(Debug)]
enum MyTaxApiError {
    Status {
        operation: &'static str,
        status: StatusCode,
        body: String,
    },
    Transport(reqwest::Error),
    Serde(serde_json::Error),
    Other(anyhow::Error),
}

impl MyTaxApiError {
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

impl std::fmt::Display for MyTaxApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Status {
                operation,
                status,
                body,
            } => write!(f, "MyTax {operation} failed with status {status}: {body}"),
            Self::Transport(error) => write!(f, "MyTax transport error: {error}"),
            Self::Serde(error) => write!(f, "MyTax parse error: {error}"),
            Self::Other(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for MyTaxApiError {}

pub async fn ensure_receipt_for_order_in_tx(
    db: &impl ConnectionTrait,
    order: &ShopOrderModel,
    config: &ReceiptsConfig,
) -> Result<()> {
    if ShopReceipt::find()
        .filter(ShopReceiptColumn::OrderId.eq(order.id))
        .one(db)
        .await?
        .is_some()
    {
        return Ok(());
    }

    let now = chrono::Utc::now();
    let (status, next_attempt_at, deadline_at, failure_problem) = if !config.enabled {
        (
            RECEIPT_STATUS_NOT_REQUIRED,
            None,
            None,
            Some("Shop receipts are disabled".to_string()),
        )
    } else if is_test_payment(db, order).await? {
        (
            RECEIPT_STATUS_TEST_PAYMENT,
            None,
            None,
            Some("Receipt skipped for test payment".to_string()),
        )
    } else {
        (
            RECEIPT_STATUS_PENDING,
            Some(now),
            Some(now + chrono::Duration::seconds(config.failure_after_seconds)),
            None,
        )
    };

    ShopReceiptActiveModel {
        id: Set(Uuid::new_v4()),
        order_id: Set(order.id),
        provider: Set(config.provider.as_str().to_string()),
        status: Set(status.to_string()),
        receipt_uuid: Set(None),
        json_url: Set(None),
        print_url: Set(None),
        attempt_count: Set(0),
        first_attempt_at: Set(None),
        last_attempt_at: Set(None),
        next_attempt_at: Set(next_attempt_at),
        deadline_at: Set(deadline_at),
        locked_until: Set(None),
        failure_problem: Set(failure_problem),
        request_payload: Set(json!({}).to_string()),
        response_payload: Set(json!({}).to_string()),
        completed_at: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
    }
    .insert(db)
    .await?;

    Ok(())
}

pub async fn process_due_receipts(
    db: &DatabaseConnection,
    config: &ReceiptsConfig,
    runtime: &ReceiptRuntime,
) -> Result<()> {
    if !config.enabled {
        return Ok(());
    }

    let now = chrono::Utc::now();
    let receipt_ids = ShopReceipt::find()
        .filter(ShopReceiptColumn::Status.is_in([
            RECEIPT_STATUS_PENDING.to_string(),
            RECEIPT_STATUS_FORMING.to_string(),
        ]))
        .filter(
            Condition::any()
                .add(ShopReceiptColumn::NextAttemptAt.is_null())
                .add(ShopReceiptColumn::NextAttemptAt.lte(now)),
        )
        .filter(
            Condition::any()
                .add(ShopReceiptColumn::LockedUntil.is_null())
                .add(ShopReceiptColumn::LockedUntil.lte(now)),
        )
        .order_by_asc(ShopReceiptColumn::NextAttemptAt)
        .all(db)
        .await?
        .into_iter()
        .take(25)
        .map(|receipt| receipt.id)
        .collect::<Vec<_>>();

    for receipt_id in receipt_ids {
        if let Err(error) = process_one_receipt(db, config, runtime, receipt_id).await {
            tracing::warn!("Receipt processing failed for {receipt_id}: {error}");
        }
    }

    Ok(())
}

pub async fn get_receipt_for_user_order(
    db: &DatabaseConnection,
    user_id: Uuid,
    order_id: Uuid,
) -> Result<Option<ShopReceiptView>> {
    let Some(order) = ShopOrder::find_by_id(order_id).one(db).await? else {
        return Ok(None);
    };
    if order.user_id != user_id {
        return Ok(None);
    }
    let receipt = ShopReceipt::find()
        .filter(ShopReceiptColumn::OrderId.eq(order_id))
        .one(db)
        .await?;
    Ok(receipt.map(map_receipt_view))
}

pub fn map_receipt_view(receipt: ShopReceiptModel) -> ShopReceiptView {
    ShopReceiptView {
        id: receipt.id,
        provider: receipt.provider,
        status: receipt.status.clone(),
        display_status: display_status(&receipt.status).to_string(),
        receipt_uuid: receipt.receipt_uuid,
        print_url: receipt.print_url,
        json_url: receipt.json_url,
        failure_problem: receipt.failure_problem,
        attempt_count: receipt.attempt_count,
        next_attempt_at: receipt.next_attempt_at,
        deadline_at: receipt.deadline_at,
        completed_at: receipt.completed_at,
        created_at: receipt.created_at,
        updated_at: receipt.updated_at,
    }
}

fn display_status(status: &str) -> &'static str {
    match status {
        RECEIPT_STATUS_TEST_PAYMENT => "Тестовый платеж",
        RECEIPT_STATUS_PENDING | RECEIPT_STATUS_FORMING => "Чек формируется",
        RECEIPT_STATUS_FAILED => "Ошибка формирования чека",
        RECEIPT_STATUS_COMPLETED => "Завершен с чеком",
        RECEIPT_STATUS_NOT_REQUIRED => "Чек не требуется",
        _ => "Статус чека неизвестен",
    }
}

async fn process_one_receipt(
    db: &DatabaseConnection,
    config: &ReceiptsConfig,
    runtime: &ReceiptRuntime,
    receipt_id: Uuid,
) -> Result<()> {
    let Some(receipt) = claim_receipt(db, receipt_id).await? else {
        return Ok(());
    };

    let now = chrono::Utc::now();
    if receipt.deadline_at.is_some_and(|deadline| deadline <= now) {
        mark_receipt_failed(db, receipt, "Receipt deadline exceeded".to_string()).await?;
        return Ok(());
    }

    let order = ShopOrder::find_by_id(receipt.order_id)
        .one(db)
        .await?
        .ok_or_else(|| anyhow!("Receipt order not found"))?;
    if is_test_payment(db, &order).await? {
        mark_receipt_test_payment(db, receipt).await?;
        return Ok(());
    }

    let submission = match config.provider {
        ReceiptProviderKind::MyTax => runtime.mytax()?.add_income_for_order(db, &order).await,
    };

    match submission {
        Ok(submission) => mark_receipt_completed(db, receipt, submission).await?,
        Err(error) => schedule_receipt_retry(db, config, receipt, error.to_string()).await?,
    }
    Ok(())
}

async fn claim_receipt(
    db: &DatabaseConnection,
    receipt_id: Uuid,
) -> Result<Option<ShopReceiptModel>> {
    let now = chrono::Utc::now();
    let locked_until = now + chrono::Duration::seconds(RECEIPT_LOCK_SECONDS);
    let update = ShopReceipt::update_many()
        .set(ShopReceiptActiveModel {
            status: Set(RECEIPT_STATUS_FORMING.to_string()),
            locked_until: Set(Some(locked_until)),
            updated_at: Set(now),
            ..Default::default()
        })
        .filter(ShopReceiptColumn::Id.eq(receipt_id))
        .filter(ShopReceiptColumn::Status.is_in([
            RECEIPT_STATUS_PENDING.to_string(),
            RECEIPT_STATUS_FORMING.to_string(),
        ]))
        .filter(
            Condition::any()
                .add(ShopReceiptColumn::LockedUntil.is_null())
                .add(ShopReceiptColumn::LockedUntil.lte(now)),
        )
        .exec(db)
        .await?;

    if update.rows_affected == 0 {
        return Ok(None);
    }

    ShopReceipt::find_by_id(receipt_id)
        .one(db)
        .await
        .map_err(Into::into)
}

async fn mark_receipt_completed(
    db: &DatabaseConnection,
    receipt: ShopReceiptModel,
    submission: ReceiptSubmission,
) -> Result<()> {
    let now = chrono::Utc::now();
    let mut active: ShopReceiptActiveModel = receipt.into();
    active.status = Set(RECEIPT_STATUS_COMPLETED.to_string());
    active.receipt_uuid = Set(Some(submission.receipt_uuid));
    active.json_url = Set(Some(submission.json_url));
    active.print_url = Set(Some(submission.print_url));
    active.request_payload = Set(submission.request_payload.to_string());
    active.response_payload = Set(submission.response_payload.to_string());
    active.completed_at = Set(Some(now));
    active.last_attempt_at = Set(Some(now));
    active.next_attempt_at = Set(None);
    active.locked_until = Set(None);
    active.failure_problem = Set(None);
    active.updated_at = Set(now);
    active.update(db).await?;
    Ok(())
}

async fn schedule_receipt_retry(
    db: &DatabaseConnection,
    config: &ReceiptsConfig,
    receipt: ShopReceiptModel,
    problem: String,
) -> Result<()> {
    let now = chrono::Utc::now();
    let deadline_reached = receipt.deadline_at.is_some_and(|deadline| deadline <= now);
    let attempt_count = receipt.attempt_count + 1;
    let first_attempt_at = receipt.first_attempt_at.or(Some(now));
    let mut active: ShopReceiptActiveModel = receipt.into();
    active.status = Set(if deadline_reached {
        RECEIPT_STATUS_FAILED.to_string()
    } else {
        RECEIPT_STATUS_FORMING.to_string()
    });
    active.attempt_count = Set(attempt_count);
    active.first_attempt_at = Set(first_attempt_at);
    active.last_attempt_at = Set(Some(now));
    active.next_attempt_at = Set(if deadline_reached {
        None
    } else {
        Some(now + chrono::Duration::seconds(config.retry_interval_seconds))
    });
    active.locked_until = Set(None);
    active.failure_problem = Set(Some(problem));
    active.updated_at = Set(now);
    active.update(db).await?;
    Ok(())
}

async fn mark_receipt_failed(
    db: &DatabaseConnection,
    receipt: ShopReceiptModel,
    problem: String,
) -> Result<()> {
    let now = chrono::Utc::now();
    let mut active: ShopReceiptActiveModel = receipt.into();
    active.status = Set(RECEIPT_STATUS_FAILED.to_string());
    active.next_attempt_at = Set(None);
    active.locked_until = Set(None);
    active.failure_problem = Set(Some(problem));
    active.updated_at = Set(now);
    active.update(db).await?;
    Ok(())
}

async fn mark_receipt_test_payment(
    db: &DatabaseConnection,
    receipt: ShopReceiptModel,
) -> Result<()> {
    let now = chrono::Utc::now();
    let mut active: ShopReceiptActiveModel = receipt.into();
    active.status = Set(RECEIPT_STATUS_TEST_PAYMENT.to_string());
    active.next_attempt_at = Set(None);
    active.locked_until = Set(None);
    active.failure_problem = Set(Some("Receipt skipped for test payment".to_string()));
    active.updated_at = Set(now);
    active.update(db).await?;
    Ok(())
}

async fn is_test_payment(db: &impl ConnectionTrait, order: &ShopOrderModel) -> Result<bool> {
    if order.payment_provider == "mock" {
        return Ok(true);
    }

    let Some(attempt) = ShopPaymentAttempt::find()
        .filter(ShopPaymentAttemptColumn::OrderId.eq(order.id))
        .order_by_desc(ShopPaymentAttemptColumn::CreatedAt)
        .one(db)
        .await?
    else {
        return Ok(false);
    };
    let payload: Value =
        serde_json::from_str(&attempt.response_payload).unwrap_or_else(|_| json!({}));
    Ok(payload
        .pointer("/verifiedPayment/test")
        .and_then(Value::as_bool)
        .or_else(|| payload.get("test").and_then(Value::as_bool))
        .unwrap_or(false))
}

async fn get_or_create_mytax_device_id(db: &DatabaseConnection, prefix: &str) -> Result<String> {
    if let Some(row) = AppKv::find_by_id(APP_KV_MYTAX_DEVICE_ID.to_string())
        .one(db)
        .await?
    {
        return Ok(row.value);
    }

    let now = chrono::Utc::now();
    let device_id = generate_device_id(prefix);
    match (AppKvActiveModel {
        key: Set(APP_KV_MYTAX_DEVICE_ID.to_string()),
        value: Set(device_id.clone()),
        created_at: Set(now),
        updated_at: Set(now),
    })
    .insert(db)
    .await
    {
        Ok(_) => Ok(device_id),
        Err(_) => AppKv::find_by_id(APP_KV_MYTAX_DEVICE_ID.to_string())
            .one(db)
            .await?
            .map(|row| row.value)
            .ok_or_else(|| anyhow!("Failed to create MyTax device id")),
    }
}

fn generate_device_id(prefix: &str) -> String {
    const CHARSET: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
    let target_len = 21usize;
    let prefix = prefix.chars().take(target_len).collect::<String>();
    let suffix_len = target_len.saturating_sub(prefix.len());
    let mut rng = rand::rng();
    let suffix = (0..suffix_len)
        .map(|_| {
            let idx = rng.random_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect::<String>();
    format!("{prefix}{suffix}")
}

fn valid_token(auth: &MyTaxAuthState) -> Option<String> {
    let token = auth.token.clone()?;
    let expires_at = auth.token_expires_at?;
    if expires_at > chrono::Utc::now() + chrono::Duration::minutes(5) {
        Some(token)
    } else {
        None
    }
}

fn apply_token_response(auth: &mut MyTaxAuthState, response: MyTaxTokenResponse) -> Result<()> {
    auth.token = Some(response.token);
    auth.refresh_token = Some(response.refresh_token);
    auth.token_expires_at = Some(parse_mytax_expiry(&response.token_expire_in)?);
    Ok(())
}

fn parse_mytax_expiry(value: &str) -> Result<chrono::DateTime<chrono::Utc>> {
    chrono::DateTime::parse_from_rfc3339(value)
        .map(|date| date.with_timezone(&chrono::Utc))
        .with_context(|| format!("Failed to parse MyTax token expiration: {value}"))
}

fn current_mytax_time(zone_offset: &str) -> Result<String> {
    let offset = parse_zone_offset(zone_offset)?;
    Ok(chrono::Utc::now()
        .with_timezone(&offset)
        .format("%Y-%m-%dT%H:%M:%S%:z")
        .to_string())
}

fn parse_zone_offset(value: &str) -> Result<chrono::FixedOffset> {
    if value == "Z" {
        return chrono::FixedOffset::east_opt(0).ok_or_else(|| anyhow!("invalid UTC offset"));
    }
    let sign = if value.starts_with('-') { -1 } else { 1 };
    let trimmed = value.trim_start_matches(['+', '-']);
    let mut parts = trimmed.split(':');
    let hours = parts
        .next()
        .ok_or_else(|| anyhow!("MYTAX_ZONE_OFFSET is invalid"))?
        .parse::<i32>()
        .context("MYTAX_ZONE_OFFSET hours are invalid")?;
    let minutes = parts
        .next()
        .unwrap_or("0")
        .parse::<i32>()
        .context("MYTAX_ZONE_OFFSET minutes are invalid")?;
    chrono::FixedOffset::east_opt(sign * (hours * 3600 + minutes * 60))
        .ok_or_else(|| anyhow!("MYTAX_ZONE_OFFSET is out of range"))
}

fn device_info(device_id: &str) -> Value {
    json!({
        "appVersion": "1.0.0",
        "sourceType": "WEB",
        "sourceDeviceId": device_id,
        "metaDetails": {
            "userAgent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0",
        }
    })
}

fn common_headers() -> reqwest::header::HeaderMap {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        reqwest::header::ACCEPT,
        "application/json, text/plain, */*".parse().unwrap(),
    );
    headers.insert(
        reqwest::header::ACCEPT_LANGUAGE,
        "ru,en;q=0.9".parse().unwrap(),
    );
    headers.insert(
        reqwest::header::CONTENT_TYPE,
        "application/json".parse().unwrap(),
    );
    headers
}

async fn parse_mytax_response<T: for<'de> Deserialize<'de>>(
    response: reqwest::Response,
    operation: &'static str,
) -> Result<T, MyTaxApiError> {
    let status = response.status();
    let body = response.text().await.map_err(MyTaxApiError::Transport)?;
    if !status.is_success() {
        return Err(MyTaxApiError::Status {
            operation,
            status,
            body,
        });
    }
    serde_json::from_str(&body).map_err(MyTaxApiError::Serde)
}
