mod yookassa;

use anyhow::{Result, anyhow};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, Condition, ConnectionTrait,
    DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, TransactionTrait,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::app::config::{ReceiptsConfig, ShopConfig, ShopPaymentProviderKind};
use crate::entities::{
    AssetDefinition, AssetDefinitionColumn, AssetDefinitionModel, ShopOrder, ShopOrderActiveModel,
    ShopOrderColumn, ShopOrderModel, ShopPaymentAttempt, ShopPaymentAttemptActiveModel,
    ShopPaymentAttemptColumn, ShopPaymentAttemptModel, ShopProduct, ShopProductActiveModel,
    ShopProductColumn, ShopProductLocale, ShopProductLocaleActiveModel, ShopProductLocaleColumn,
    ShopProductLocaleModel, ShopProductModel, ShopReceipt, ShopReceiptColumn, User,
    UserEntitlement, UserStackableAsset,
};
use crate::services::discord_notifications::queue_shop_purchase_completed_notification;
use crate::services::ownership::inventory::{
    self, EntitlementMutation, ProlongExpirableMutation, StackableMutation, grant_entitlement_in_tx,
};
use crate::services::ownership::types::{
    OperationContext, OwnershipActor, OwnershipModel, normalize_metadata, validate_asset_key,
};
use crate::services::receipts::{self, ShopReceiptView};

const ORDER_STATUS_PENDING_PAYMENT: &str = "pending_payment";
const ORDER_STATUS_PAID: &str = "paid";
const ORDER_STATUS_FULFILLMENT_IN_PROGRESS: &str = "fulfillment_in_progress";
const ORDER_STATUS_FULFILLED: &str = "fulfilled";
const ORDER_STATUS_FULFILLMENT_FAILED: &str = "fulfillment_failed";
const ORDER_STATUS_PAYMENT_CANCELED: &str = "payment_canceled";
const ORDER_STATUS_PAYMENT_EXPIRED: &str = "payment_expired";
const ORDER_STATUS_PAYMENT_VALIDATION_FAILED: &str = "payment_validation_failed";

const PAYMENT_STATUS_PENDING: &str = "pending";
const PAYMENT_STATUS_WAITING_FOR_CAPTURE: &str = "waiting_for_capture";
const PAYMENT_STATUS_SUCCEEDED: &str = "succeeded";
const PAYMENT_STATUS_CANCELED: &str = "canceled";
const FULFILLMENT_RETRY_AFTER_SECONDS: i64 = 60;

#[derive(Clone, Debug, Serialize)]
pub struct ShopProductLocaleView {
    pub locale: String,
    pub name: String,
    pub description: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ShopProductView {
    pub id: Uuid,
    pub key: String,
    pub asset_definition_id: Uuid,
    pub asset_key: String,
    pub asset_display_name: String,
    pub ownership_model: OwnershipModel,
    pub price_rub: i64,
    pub stackable_amount: Option<i64>,
    pub duration_seconds: Option<i64>,
    pub max_per_purchase: Option<i64>,
    pub max_owned_amount: Option<i64>,
    pub localized_name: String,
    pub localized_description: Option<String>,
    pub locales: Vec<ShopProductLocaleView>,
    pub is_active: bool,
    pub is_public: bool,
    pub is_available_now: bool,
    pub sort_order: i32,
    pub starts_at: Option<chrono::DateTime<chrono::Utc>>,
    pub ends_at: Option<chrono::DateTime<chrono::Utc>>,
    pub metadata: Value,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ShopPaymentAttemptView {
    pub id: Uuid,
    pub provider: String,
    pub provider_payment_id: String,
    pub checkout_token: Option<String>,
    pub checkout_url: Option<String>,
    pub status: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ShopWebhookAckView {
    pub ok: bool,
    pub provider: String,
    pub event: String,
    pub provider_payment_id: String,
    pub order_id: Option<Uuid>,
    pub order_status: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ShopOrderView {
    pub id: Uuid,
    pub user_id: Uuid,
    pub product_id: Uuid,
    pub product_key: String,
    pub product_name: String,
    pub product_description: Option<String>,
    pub product_locale: Option<String>,
    pub asset_definition_id: Uuid,
    pub asset_key: String,
    pub ownership_model: OwnershipModel,
    pub quantity: i64,
    pub unit_price_rub: i64,
    pub total_price_rub: i64,
    pub granted_amount: Option<i64>,
    pub granted_duration_seconds: Option<i64>,
    pub max_owned_amount: Option<i64>,
    pub payment_provider: String,
    pub status: String,
    pub failure_problem: Option<String>,
    pub payment_expires_at: Option<chrono::DateTime<chrono::Utc>>,
    pub paid_at: Option<chrono::DateTime<chrono::Utc>>,
    pub fulfilled_at: Option<chrono::DateTime<chrono::Utc>>,
    pub payment: Option<ShopPaymentAttemptView>,
    pub receipt: Option<ShopReceiptView>,
    pub metadata: Value,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Clone, Debug, Deserialize, ToSchema)]
pub struct ProductLocaleInput {
    pub locale: String,
    pub name: String,
    pub description: Option<String>,
}

#[derive(Clone, Debug, Deserialize, ToSchema)]
pub struct CreateShopProductInput {
    pub key: String,
    pub asset_key: String,
    pub price_rub: i64,
    pub stackable_amount: Option<i64>,
    #[serde(rename = "durationSeconds")]
    pub duration_seconds: Option<i64>,
    pub max_per_purchase: Option<i64>,
    pub max_owned_amount: Option<i64>,
    pub is_active: Option<bool>,
    pub is_public: Option<bool>,
    pub sort_order: Option<i32>,
    pub starts_at: Option<chrono::DateTime<chrono::Utc>>,
    pub ends_at: Option<chrono::DateTime<chrono::Utc>>,
    pub metadata: Option<Value>,
    pub locales: Vec<ProductLocaleInput>,
}

#[derive(Clone, Debug, Deserialize, ToSchema)]
pub struct UpdateShopProductInput {
    pub price_rub: Option<i64>,
    pub stackable_amount: Option<Option<i64>>,
    #[serde(rename = "durationSeconds")]
    pub duration_seconds: Option<Option<i64>>,
    pub max_per_purchase: Option<Option<i64>>,
    pub max_owned_amount: Option<Option<i64>>,
    pub is_active: Option<bool>,
    pub is_public: Option<bool>,
    pub sort_order: Option<i32>,
    pub starts_at: Option<Option<chrono::DateTime<chrono::Utc>>>,
    pub ends_at: Option<Option<chrono::DateTime<chrono::Utc>>>,
    pub metadata: Option<Value>,
    pub locales: Option<Vec<ProductLocaleInput>>,
}

#[derive(Clone, Debug, Deserialize, ToSchema)]
pub struct CreateShopOrderInput {
    pub product_key: String,
    pub quantity: Option<i64>,
    pub locale: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Default)]
pub struct ShopCatalogQuery {
    pub locale: Option<String>,
}

#[async_trait::async_trait]
trait PaymentProvider {
    fn provider_name(&self) -> &'static str;

    async fn create_payment(&self, command: CreatePaymentCommand) -> Result<CreatePaymentResult>;
}

#[derive(Clone, Debug)]
pub(super) struct CreatePaymentCommand {
    pub order_id: Uuid,
    pub amount_rub: i64,
    pub description: String,
}

#[derive(Clone, Debug)]
pub(super) struct CreatePaymentResult {
    pub provider_payment_id: String,
    pub checkout_token: Option<String>,
    pub checkout_url: Option<String>,
    pub request_payload: Value,
    pub response_payload: Value,
}

struct MockPaymentProvider;

#[async_trait::async_trait]
impl PaymentProvider for MockPaymentProvider {
    fn provider_name(&self) -> &'static str {
        "mock"
    }

    async fn create_payment(&self, command: CreatePaymentCommand) -> Result<CreatePaymentResult> {
        let provider_payment_id = format!("mock-payment-{}", Uuid::new_v4().as_simple());
        let checkout_token = format!("mock-checkout-{}", Uuid::new_v4().as_simple());
        Ok(CreatePaymentResult {
            provider_payment_id: provider_payment_id.clone(),
            checkout_token: Some(checkout_token.clone()),
            checkout_url: Some(format!(
                "/api/user/me/shop/orders/{}/mock/complete",
                command.order_id
            )),
            request_payload: json!({
                "provider": self.provider_name(),
                "amountRub": command.amount_rub,
                "description": command.description,
            }),
            response_payload: json!({
                "providerPaymentId": provider_payment_id,
                "checkoutToken": checkout_token,
                "status": PAYMENT_STATUS_PENDING,
            }),
        })
    }
}

pub async fn list_products(
    db: &DatabaseConnection,
    query: ShopCatalogQuery,
    admin_view: bool,
) -> Result<Vec<ShopProductView>> {
    let products = ShopProduct::find()
        .order_by_asc(ShopProductColumn::SortOrder)
        .order_by_asc(ShopProductColumn::Key)
        .all(db)
        .await?;
    let assets = AssetDefinition::find().all(db).await?;
    let locales = ShopProductLocale::find()
        .order_by_asc(ShopProductLocaleColumn::Locale)
        .all(db)
        .await?;
    let now = chrono::Utc::now();
    let locale = normalize_optional_locale(query.locale)?;

    let mut result = Vec::new();
    for product in products {
        let asset = assets
            .iter()
            .find(|asset| asset.id == product.asset_definition_id)
            .ok_or_else(|| anyhow!("Missing asset definition for shop product"))?
            .clone();
        let localized = locales_for_product(&locales, product.id);
        let view = map_product_view(product, asset, localized, locale.as_deref(), now)?;
        if admin_view || (view.is_public && view.is_active && view.is_available_now) {
            result.push(view);
        }
    }

    Ok(result)
}

pub async fn get_product_by_key(
    db: &DatabaseConnection,
    product_key: &str,
    locale: Option<String>,
    admin_view: bool,
) -> Result<Option<ShopProductView>> {
    let product_key = validate_shop_key(product_key)?;
    let product = ShopProduct::find()
        .filter(ShopProductColumn::Key.eq(product_key))
        .one(db)
        .await?;
    let Some(product) = product else {
        return Ok(None);
    };
    let asset = AssetDefinition::find_by_id(product.asset_definition_id)
        .one(db)
        .await?
        .ok_or_else(|| anyhow!("Missing asset definition for shop product"))?;
    let locales = ShopProductLocale::find()
        .filter(ShopProductLocaleColumn::ProductId.eq(product.id))
        .all(db)
        .await?;
    let now = chrono::Utc::now();
    let locale = normalize_optional_locale(locale)?;
    let view = map_product_view(product, asset, locales, locale.as_deref(), now)?;

    if !admin_view && !(view.is_public && view.is_active && view.is_available_now) {
        return Ok(None);
    }

    Ok(Some(view))
}

pub async fn get_product_by_id(
    db: &DatabaseConnection,
    product_id: Uuid,
    locale: Option<String>,
) -> Result<Option<ShopProductView>> {
    let product = ShopProduct::find_by_id(product_id).one(db).await?;
    let Some(product) = product else {
        return Ok(None);
    };
    let asset = AssetDefinition::find_by_id(product.asset_definition_id)
        .one(db)
        .await?
        .ok_or_else(|| anyhow!("Missing asset definition for shop product"))?;
    let locales = ShopProductLocale::find()
        .filter(ShopProductLocaleColumn::ProductId.eq(product.id))
        .all(db)
        .await?;
    let locale = normalize_optional_locale(locale)?;
    Ok(Some(map_product_view(
        product,
        asset,
        locales,
        locale.as_deref(),
        chrono::Utc::now(),
    )?))
}

pub async fn create_product(
    db: &DatabaseConnection,
    input: CreateShopProductInput,
) -> Result<ShopProductView> {
    let product_key = validate_shop_key(&input.key)?;
    let asset_key =
        validate_asset_key(&input.asset_key).map_err(|error| bad_request(error.to_string()))?;
    let price_rub = validate_price_rub(input.price_rub)?;
    let locales = normalize_locale_inputs(input.locales)?;
    validate_availability_window(input.starts_at, input.ends_at)?;

    if ShopProduct::find()
        .filter(ShopProductColumn::Key.eq(&product_key))
        .one(db)
        .await?
        .is_some()
    {
        return Err(conflict("Shop product key already exists"));
    }

    let asset = get_product_asset_by_key(db, &asset_key).await?;
    let ownership_model = parse_ownership_model(&asset)?;
    validate_reward_configuration(
        ownership_model,
        input.stackable_amount,
        input.duration_seconds,
        input.max_per_purchase,
        input.max_owned_amount,
    )?;

    let tx = db.begin().await?;
    let now = chrono::Utc::now();
    let product = ShopProductActiveModel {
        id: Set(Uuid::new_v4()),
        key: Set(product_key),
        asset_definition_id: Set(asset.id),
        price_rub: Set(price_rub),
        stackable_amount: Set(input.stackable_amount),
        expirable_duration_seconds: Set(input.duration_seconds),
        max_per_purchase: Set(input.max_per_purchase),
        max_owned_amount: Set(input.max_owned_amount),
        is_active: Set(input.is_active.unwrap_or(true)),
        is_public: Set(input.is_public.unwrap_or(true)),
        sort_order: Set(input.sort_order.unwrap_or(0)),
        starts_at: Set(input.starts_at),
        ends_at: Set(input.ends_at),
        metadata: Set(normalize_metadata(input.metadata).to_string()),
        created_at: Set(now),
        updated_at: Set(now),
    }
    .insert(&tx)
    .await?;

    replace_product_locales(&tx, product.id, &locales, now).await?;
    tx.commit().await?;

    get_product_by_id(db, product.id, None)
        .await?
        .ok_or_else(|| anyhow!("Created product could not be loaded"))
}

pub async fn update_product(
    db: &DatabaseConnection,
    product_id: Uuid,
    input: UpdateShopProductInput,
) -> Result<ShopProductView> {
    let existing = ShopProduct::find_by_id(product_id)
        .one(db)
        .await?
        .ok_or_else(|| not_found("Shop product not found"))?;
    let asset = AssetDefinition::find_by_id(existing.asset_definition_id)
        .one(db)
        .await?
        .ok_or_else(|| anyhow!("Missing asset definition for shop product"))?;
    let ownership_model = parse_ownership_model(&asset)?;
    let price_rub = input.price_rub.unwrap_or(existing.price_rub);
    let stackable_amount = input.stackable_amount.unwrap_or(existing.stackable_amount);
    let duration_seconds = input
        .duration_seconds
        .unwrap_or(existing.expirable_duration_seconds);
    let max_per_purchase = input.max_per_purchase.unwrap_or(existing.max_per_purchase);
    let max_owned_amount = input.max_owned_amount.unwrap_or(existing.max_owned_amount);
    let starts_at = input.starts_at.unwrap_or(existing.starts_at);
    let ends_at = input.ends_at.unwrap_or(existing.ends_at);

    validate_price_rub(price_rub)?;
    validate_availability_window(starts_at, ends_at)?;
    validate_reward_configuration(
        ownership_model,
        stackable_amount,
        duration_seconds,
        max_per_purchase,
        max_owned_amount,
    )?;

    let locales = match input.locales {
        Some(locales) => Some(normalize_locale_inputs(locales)?),
        None => None,
    };

    let tx = db.begin().await?;
    let mut active: ShopProductActiveModel = existing.into();
    active.price_rub = Set(price_rub);
    active.stackable_amount = Set(stackable_amount);
    active.expirable_duration_seconds = Set(duration_seconds);
    active.max_per_purchase = Set(max_per_purchase);
    active.max_owned_amount = Set(max_owned_amount);
    if let Some(is_active) = input.is_active {
        active.is_active = Set(is_active);
    }
    if let Some(is_public) = input.is_public {
        active.is_public = Set(is_public);
    }
    if let Some(sort_order) = input.sort_order {
        active.sort_order = Set(sort_order);
    }
    if input.starts_at.is_some() {
        active.starts_at = Set(starts_at);
    }
    if input.ends_at.is_some() {
        active.ends_at = Set(ends_at);
    }
    if let Some(metadata) = input.metadata {
        active.metadata = Set(normalize_metadata(Some(metadata)).to_string());
    }
    active.updated_at = Set(chrono::Utc::now());
    let product = active.update(&tx).await?;

    if let Some(locales) = locales {
        replace_product_locales(&tx, product.id, &locales, chrono::Utc::now()).await?;
    }

    tx.commit().await?;
    get_product_by_id(db, product.id, None)
        .await?
        .ok_or_else(|| anyhow!("Updated product could not be loaded"))
}

pub async fn list_orders_for_user(
    db: &DatabaseConnection,
    user_id: Uuid,
    shop_config: &ShopConfig,
    receipts_config: &ReceiptsConfig,
) -> Result<Vec<ShopOrderView>> {
    ensure_user_exists(db, user_id).await?;
    reconcile_orders_for_user(db, user_id, shop_config, receipts_config).await?;
    let orders = ShopOrder::find()
        .filter(ShopOrderColumn::UserId.eq(user_id))
        .order_by_desc(ShopOrderColumn::CreatedAt)
        .all(db)
        .await?;
    map_orders_with_attempts(db, orders, shop_config).await
}

pub async fn get_order_for_user(
    db: &DatabaseConnection,
    user_id: Uuid,
    order_id: Uuid,
    shop_config: &ShopConfig,
    receipts_config: &ReceiptsConfig,
) -> Result<Option<ShopOrderView>> {
    ensure_user_exists(db, user_id).await?;
    reconcile_order_for_user(db, user_id, order_id, shop_config, receipts_config).await?;
    let order = ShopOrder::find_by_id(order_id).one(db).await?;
    let Some(order) = order else {
        return Ok(None);
    };
    if order.user_id != user_id {
        return Ok(None);
    }
    let mut mapped = map_orders_with_attempts(db, vec![order], shop_config).await?;
    Ok(mapped.pop())
}

pub async fn create_order(
    db: &DatabaseConnection,
    user_id: Uuid,
    input: CreateShopOrderInput,
    shop_config: ShopConfig,
    receipts_config: ReceiptsConfig,
) -> Result<ShopOrderView> {
    let product_key = validate_shop_key(&input.product_key)?;
    let quantity = input.quantity.unwrap_or(1);
    if quantity <= 0 {
        return Err(unprocessable("quantity must be positive"));
    }
    let locale = normalize_optional_locale(input.locale)?;

    let tx = db.begin().await?;
    ensure_user_exists(&tx, user_id).await?;
    let (product, asset, locales) = load_product_bundle_by_key(&tx, &product_key)
        .await?
        .ok_or_else(|| not_found("Shop product not found"))?;
    let now = chrono::Utc::now();
    ensure_product_is_sellable(&product, now)?;

    validate_order_quantity(&product, &asset, quantity)?;
    ensure_purchase_allowed(
        &tx,
        user_id,
        &product,
        &asset,
        quantity,
        product.max_owned_amount,
    )
    .await?;

    let localized = localize_product(
        &locales,
        locale.as_deref(),
        &asset.display_name,
        asset.description.as_deref(),
    );
    let total_price_rub = product
        .price_rub
        .checked_mul(quantity)
        .ok_or_else(|| bad_request("total price overflow"))?;

    let order_id = Uuid::new_v4();
    let payment_expires_at =
        Some(now + chrono::Duration::seconds(shop_config.pending_payment_ttl_seconds));
    let (provider_name, payment) = create_payment(
        &shop_config,
        CreatePaymentCommand {
            order_id,
            amount_rub: total_price_rub,
            description: localized.name.clone(),
        },
    )
    .await?;

    let order = ShopOrderActiveModel {
        id: Set(order_id),
        user_id: Set(user_id),
        product_id: Set(product.id),
        product_key: Set(product.key.clone()),
        product_locale: Set(locale.clone()),
        product_name: Set(localized.name),
        product_description: Set(localized.description),
        asset_definition_id: Set(asset.id),
        asset_key: Set(asset.key.clone()),
        ownership_model: Set(parse_ownership_model(&asset)?.as_str().to_string()),
        quantity: Set(quantity),
        unit_price_rub: Set(product.price_rub),
        total_price_rub: Set(total_price_rub),
        stackable_amount_per_unit: Set(product.stackable_amount),
        expirable_duration_seconds_per_unit: Set(product.expirable_duration_seconds),
        max_owned_amount_snapshot: Set(product.max_owned_amount),
        payment_provider: Set(provider_name.to_string()),
        status: Set(ORDER_STATUS_PENDING_PAYMENT.to_string()),
        failure_problem: Set(None),
        payment_expires_at: Set(payment_expires_at),
        paid_at: Set(None),
        fulfilled_at: Set(None),
        metadata: Set(json!({ "productKey": product.key }).to_string()),
        created_at: Set(now),
        updated_at: Set(now),
    }
    .insert(&tx)
    .await?;

    ShopPaymentAttemptActiveModel {
        id: Set(Uuid::new_v4()),
        order_id: Set(order.id),
        provider: Set(provider_name.to_string()),
        provider_payment_id: Set(payment.provider_payment_id),
        checkout_token: Set(payment.checkout_token),
        checkout_url: Set(payment.checkout_url),
        status: Set(PAYMENT_STATUS_PENDING.to_string()),
        request_payload: Set(payment.request_payload.to_string()),
        response_payload: Set(payment.response_payload.to_string()),
        created_at: Set(now),
        updated_at: Set(now),
    }
    .insert(&tx)
    .await?;

    tx.commit().await?;

    get_order_for_user(db, user_id, order.id, &shop_config, &receipts_config)
        .await?
        .ok_or_else(|| anyhow!("Created order could not be loaded"))
}

async fn create_payment(
    shop_config: &ShopConfig,
    command: CreatePaymentCommand,
) -> Result<(&'static str, CreatePaymentResult)> {
    match &shop_config.payment_provider {
        ShopPaymentProviderKind::Mock => {
            let provider = MockPaymentProvider;
            let payment = provider.create_payment(command).await?;
            Ok((provider.provider_name(), payment))
        }
        ShopPaymentProviderKind::YooKassa => {
            let provider = build_yookassa_client(shop_config)?;
            let payment = provider.create_payment(command).await?;
            Ok((yookassa::PROVIDER_NAME, payment))
        }
        ShopPaymentProviderKind::Disabled => Err(unavailable(
            "Shop payment provider is disabled by configuration",
        )),
    }
}

pub async fn reconcile_pending_orders(
    db: &DatabaseConnection,
    shop_config: &ShopConfig,
    receipts_config: &ReceiptsConfig,
) -> Result<()> {
    let order_ids = ShopOrder::find()
        .filter(reconcilable_order_status_condition())
        .all(db)
        .await?
        .into_iter()
        .map(|order| order.id)
        .collect::<Vec<_>>();

    for order_id in order_ids {
        reconcile_order_by_id(
            db,
            order_id,
            shop_config,
            receipts_config,
            "shop.reconciler",
            true,
        )
        .await?;
    }

    Ok(())
}

pub async fn handle_yookassa_webhook(
    db: &DatabaseConnection,
    shop_config: &ShopConfig,
    receipts_config: &ReceiptsConfig,
    headers: &axum::http::HeaderMap,
    body: &str,
) -> Result<ShopWebhookAckView> {
    if shop_config.payment_provider != ShopPaymentProviderKind::YooKassa {
        return Err(unavailable(
            "YooKassa webhook is unavailable because SHOP_PAYMENT_PROVIDER is not set to yookassa",
        ));
    }

    let notification = yookassa::parse_webhook_notification(body)
        .map_err(|error| bad_request(error.to_string()))?;
    let attempt = ShopPaymentAttempt::find()
        .filter(ShopPaymentAttemptColumn::Provider.eq(yookassa::PROVIDER_NAME))
        .filter(ShopPaymentAttemptColumn::ProviderPaymentId.eq(&notification.object.id))
        .one(db)
        .await?;

    let Some(attempt) = attempt else {
        return Ok(ShopWebhookAckView {
            ok: true,
            provider: yookassa::PROVIDER_NAME.to_string(),
            event: notification.event,
            provider_payment_id: notification.object.id,
            order_id: None,
            order_status: None,
        });
    };

    let provider = build_yookassa_client(shop_config)?;
    let payment = provider.get_payment(&notification.object.id).await?;

    let tx = db.begin().await?;
    let synced_order = sync_yookassa_attempt_and_order_in_tx(
        &tx,
        shop_config,
        receipts_config,
        attempt.id,
        &payment,
        "shop.yookassa_webhook",
        Some(&notification.event),
        notification.object.status.as_deref(),
        Some(headers),
    )
    .await?;
    let status = synced_order.status.clone();
    let order_id = synced_order.id;
    tx.commit().await?;

    Ok(ShopWebhookAckView {
        ok: true,
        provider: yookassa::PROVIDER_NAME.to_string(),
        event: notification.event,
        provider_payment_id: payment.id,
        order_id: Some(order_id),
        order_status: Some(status),
    })
}

fn build_yookassa_client(shop_config: &ShopConfig) -> Result<yookassa::YooKassaClient> {
    let yookassa_config = shop_config
        .yookassa
        .clone()
        .ok_or_else(|| unavailable("YooKassa configuration is missing"))?;
    Ok(yookassa::YooKassaClient::new(yookassa_config))
}

async fn reconcile_orders_for_user(
    db: &DatabaseConnection,
    user_id: Uuid,
    shop_config: &ShopConfig,
    receipts_config: &ReceiptsConfig,
) -> Result<()> {
    let order_ids = ShopOrder::find()
        .filter(ShopOrderColumn::UserId.eq(user_id))
        .filter(reconcilable_order_status_condition())
        .all(db)
        .await?
        .into_iter()
        .map(|order| order.id)
        .collect::<Vec<_>>();

    for order_id in order_ids {
        reconcile_order_by_id(
            db,
            order_id,
            shop_config,
            receipts_config,
            "shop.user_poll",
            true,
        )
        .await?;
    }

    Ok(())
}

async fn reconcile_order_for_user(
    db: &DatabaseConnection,
    user_id: Uuid,
    order_id: Uuid,
    shop_config: &ShopConfig,
    receipts_config: &ReceiptsConfig,
) -> Result<()> {
    let order = ShopOrder::find_by_id(order_id).one(db).await?;
    let Some(order) = order else {
        return Ok(());
    };
    if order.user_id != user_id || !should_reconcile_order(&order) {
        return Ok(());
    }

    reconcile_order_by_id(
        db,
        order_id,
        shop_config,
        receipts_config,
        "shop.user_poll",
        true,
    )
    .await?;
    Ok(())
}

async fn reconcile_order_by_id(
    db: &DatabaseConnection,
    order_id: Uuid,
    shop_config: &ShopConfig,
    receipts_config: &ReceiptsConfig,
    source: &'static str,
    tolerate_provider_errors: bool,
) -> Result<Option<ShopOrderModel>> {
    let order = ShopOrder::find_by_id(order_id).one(db).await?;
    let Some(order) = order else {
        return Ok(None);
    };
    if !should_reconcile_order(&order) {
        return Ok(Some(order));
    }

    let latest_attempt = latest_payment_attempt_for_order(db, order.id).await?;
    let reconciled = match order.payment_provider.as_str() {
        "mock" => {
            reconcile_mock_order(
                db,
                shop_config,
                receipts_config,
                &order,
                latest_attempt.as_ref(),
                source,
            )
            .await?
        }
        yookassa::PROVIDER_NAME => {
            reconcile_yookassa_order(
                db,
                shop_config,
                receipts_config,
                &order,
                latest_attempt.as_ref(),
                source,
                tolerate_provider_errors,
            )
            .await?
        }
        _ => reconcile_local_pending_order_in_tx(db, shop_config, &order).await?,
    };

    Ok(Some(reconciled))
}

async fn reconcile_mock_order(
    db: &DatabaseConnection,
    shop_config: &ShopConfig,
    receipts_config: &ReceiptsConfig,
    order: &ShopOrderModel,
    attempt: Option<&ShopPaymentAttemptModel>,
    source: &'static str,
) -> Result<ShopOrderModel> {
    let tx = db.begin().await?;
    let reloaded_order = ShopOrder::find_by_id(order.id)
        .one(&tx)
        .await?
        .ok_or_else(|| anyhow!("Shop order disappeared during reconciliation"))?;

    let synced = match attempt {
        Some(attempt) if attempt.status == PAYMENT_STATUS_SUCCEEDED => {
            process_successful_payment_in_tx(
                &tx,
                shop_config,
                receipts_config,
                &reloaded_order,
                OwnershipActor::system(source),
            )
            .await?
        }
        Some(attempt) if attempt.status == PAYMENT_STATUS_CANCELED => {
            mark_order_payment_canceled_in_tx(
                &tx,
                &reloaded_order,
                "Mock payment was canceled".to_string(),
            )
            .await?
        }
        _ => reconcile_local_pending_order_in_tx(&tx, shop_config, &reloaded_order).await?,
    };

    tx.commit().await?;
    Ok(synced)
}

async fn reconcile_yookassa_order(
    db: &DatabaseConnection,
    shop_config: &ShopConfig,
    receipts_config: &ReceiptsConfig,
    order: &ShopOrderModel,
    attempt: Option<&ShopPaymentAttemptModel>,
    source: &'static str,
    tolerate_provider_errors: bool,
) -> Result<ShopOrderModel> {
    let Some(attempt) = attempt else {
        let tx = db.begin().await?;
        let reloaded_order = ShopOrder::find_by_id(order.id)
            .one(&tx)
            .await?
            .ok_or_else(|| anyhow!("Shop order disappeared during reconciliation"))?;
        let synced = reconcile_local_pending_order_in_tx(&tx, shop_config, &reloaded_order).await?;
        tx.commit().await?;
        return Ok(synced);
    };

    let provider = build_yookassa_client(shop_config)?;
    let payment = match provider.get_payment(&attempt.provider_payment_id).await {
        Ok(payment) => payment,
        Err(error)
            if tolerate_provider_errors
                && should_expire_pending_order(order, shop_config, chrono::Utc::now()) =>
        {
            let tx = db.begin().await?;
            let reloaded_order = ShopOrder::find_by_id(order.id)
                .one(&tx)
                .await?
                .ok_or_else(|| anyhow!("Shop order disappeared during reconciliation"))?;
            let synced = mark_order_payment_expired_in_tx(&tx, &reloaded_order).await?;
            tx.commit().await?;
            return Ok(synced);
        }
        Err(error) => return Err(error),
    };

    let tx = db.begin().await?;
    let synced = sync_yookassa_attempt_and_order_in_tx(
        &tx,
        shop_config,
        receipts_config,
        attempt.id,
        &payment,
        source,
        None,
        None,
        None,
    )
    .await?;
    tx.commit().await?;
    Ok(synced)
}

async fn sync_yookassa_attempt_and_order_in_tx(
    db: &impl ConnectionTrait,
    shop_config: &ShopConfig,
    receipts_config: &ReceiptsConfig,
    attempt_id: Uuid,
    payment: &yookassa::YooKassaPayment,
    source: &'static str,
    webhook_event: Option<&str>,
    webhook_object_status: Option<&str>,
    request_headers: Option<&axum::http::HeaderMap>,
) -> Result<ShopOrderModel> {
    let payment_attempt = ShopPaymentAttempt::find_by_id(attempt_id)
        .one(db)
        .await?
        .ok_or_else(|| {
            anyhow!("Shop payment attempt disappeared during payment synchronization")
        })?;
    let order = ShopOrder::find_by_id(payment_attempt.order_id)
        .one(db)
        .await?
        .ok_or_else(|| anyhow!("Shop order disappeared during payment synchronization"))?;

    let mut response_payload = serde_json::Map::new();
    response_payload.insert(
        "verifiedPayment".to_string(),
        yookassa::payment_to_json(payment)?,
    );
    response_payload.insert("syncSource".to_string(), Value::String(source.to_string()));
    if let Some(webhook_event) = webhook_event {
        response_payload.insert(
            "webhookEvent".to_string(),
            Value::String(webhook_event.to_string()),
        );
    }
    if let Some(webhook_object_status) = webhook_object_status {
        response_payload.insert(
            "webhookObjectStatus".to_string(),
            Value::String(webhook_object_status.to_string()),
        );
    }
    if let Some(request_headers) = request_headers {
        response_payload.insert(
            "requestHeaders".to_string(),
            yookassa::extract_request_headers(request_headers),
        );
    }

    let now = chrono::Utc::now();
    let mut payment_attempt_active: ShopPaymentAttemptActiveModel = payment_attempt.into();
    payment_attempt_active.status = Set(payment.status.clone());
    payment_attempt_active.response_payload = Set(Value::Object(response_payload).to_string());
    payment_attempt_active.updated_at = Set(now);
    payment_attempt_active.update(db).await?;

    sync_order_with_payment_state_in_tx(
        db,
        shop_config,
        receipts_config,
        &order,
        payment,
        OwnershipActor::system(source),
    )
    .await
}

async fn sync_order_with_payment_state_in_tx(
    db: &impl ConnectionTrait,
    shop_config: &ShopConfig,
    receipts_config: &ReceiptsConfig,
    order: &ShopOrderModel,
    payment: &yookassa::YooKassaPayment,
    actor: OwnershipActor,
) -> Result<ShopOrderModel> {
    match payment.status.as_str() {
        PAYMENT_STATUS_SUCCEEDED => {
            if let Err(problem) = validate_yookassa_payment_matches_order(order, payment) {
                return mark_order_payment_validation_failed_in_tx(
                    db,
                    order,
                    strip_problem_prefix(problem.to_string()),
                )
                .await;
            }

            process_successful_payment_in_tx(db, shop_config, receipts_config, order, actor).await
        }
        PAYMENT_STATUS_CANCELED => {
            mark_order_payment_canceled_in_tx(db, order, yookassa::cancellation_problem(payment))
                .await
        }
        PAYMENT_STATUS_PENDING | PAYMENT_STATUS_WAITING_FOR_CAPTURE => {
            reconcile_local_pending_order_in_tx(db, shop_config, order).await
        }
        _ => Ok(order.clone()),
    }
}

async fn mark_order_paid_in_tx(
    db: &impl ConnectionTrait,
    order: &ShopOrderModel,
) -> Result<ShopOrderModel> {
    if matches!(
        order.status.as_str(),
        ORDER_STATUS_FULFILLED
            | ORDER_STATUS_FULFILLMENT_FAILED
            | ORDER_STATUS_PAYMENT_CANCELED
            | ORDER_STATUS_PAYMENT_VALIDATION_FAILED
            | ORDER_STATUS_FULFILLMENT_IN_PROGRESS
    ) {
        return Ok(order.clone());
    }
    if order.status == ORDER_STATUS_PAID {
        return Ok(order.clone());
    }

    let now = chrono::Utc::now();
    let paid_at = order.paid_at.or(Some(now));
    let update_result = ShopOrder::update_many()
        .set(ShopOrderActiveModel {
            status: Set(ORDER_STATUS_PAID.to_string()),
            paid_at: Set(paid_at),
            updated_at: Set(now),
            failure_problem: Set(None),
            ..Default::default()
        })
        .filter(ShopOrderColumn::Id.eq(order.id))
        .filter(
            ShopOrderColumn::Status
                .is_in([ORDER_STATUS_PENDING_PAYMENT, ORDER_STATUS_PAYMENT_EXPIRED]),
        )
        .exec(db)
        .await?;

    if update_result.rows_affected == 0 {
        return reload_order_in_tx(db, order.id).await;
    }

    reload_order_in_tx(db, order.id).await
}

async fn mark_order_payment_canceled_in_tx(
    db: &impl ConnectionTrait,
    order: &ShopOrderModel,
    problem: String,
) -> Result<ShopOrderModel> {
    if matches!(
        order.status.as_str(),
        ORDER_STATUS_FULFILLED
            | ORDER_STATUS_FULFILLMENT_IN_PROGRESS
            | ORDER_STATUS_FULFILLMENT_FAILED
            | ORDER_STATUS_PAYMENT_VALIDATION_FAILED
    ) {
        return Ok(order.clone());
    }

    let now = chrono::Utc::now();
    let update_result = ShopOrder::update_many()
        .set(ShopOrderActiveModel {
            status: Set(ORDER_STATUS_PAYMENT_CANCELED.to_string()),
            failure_problem: Set(Some(problem)),
            updated_at: Set(now),
            ..Default::default()
        })
        .filter(ShopOrderColumn::Id.eq(order.id))
        .filter(ShopOrderColumn::Status.is_in([
            ORDER_STATUS_PENDING_PAYMENT,
            ORDER_STATUS_PAYMENT_EXPIRED,
            ORDER_STATUS_PAID,
        ]))
        .exec(db)
        .await?;

    if update_result.rows_affected == 0 {
        return reload_order_in_tx(db, order.id).await;
    }

    reload_order_in_tx(db, order.id).await
}

pub async fn complete_mock_order_payment(
    db: &DatabaseConnection,
    user_id: Uuid,
    order_id: Uuid,
    shop_config: &ShopConfig,
    receipts_config: &ReceiptsConfig,
) -> Result<ShopOrderView> {
    let tx = db.begin().await?;
    ensure_user_exists(&tx, user_id).await?;

    let order = ShopOrder::find_by_id(order_id)
        .one(&tx)
        .await?
        .ok_or_else(|| not_found("Shop order not found"))?;
    if order.user_id != user_id {
        return Err(not_found("Shop order not found"));
    }
    if order.payment_provider != "mock" {
        return Err(bad_request(
            "Mock completion is available only for mock payments",
        ));
    }
    let order = if should_expire_pending_order(&order, shop_config, chrono::Utc::now()) {
        mark_order_payment_expired_in_tx(&tx, &order).await?
    } else {
        order
    };
    if order.status == ORDER_STATUS_FULFILLED {
        tx.commit().await?;
        return get_order_for_user(db, user_id, order_id, shop_config, receipts_config)
            .await?
            .ok_or_else(|| anyhow!("Order disappeared after fulfillment"));
    }
    if order.status == ORDER_STATUS_PAYMENT_EXPIRED {
        return Err(conflict("Shop order payment has expired"));
    }
    if order.status != ORDER_STATUS_PENDING_PAYMENT
        && order.status != ORDER_STATUS_PAID
        && order.status != ORDER_STATUS_FULFILLMENT_IN_PROGRESS
    {
        return Err(conflict(
            "Shop order cannot be completed in its current state",
        ));
    }

    let payment_attempt = ShopPaymentAttempt::find()
        .filter(ShopPaymentAttemptColumn::OrderId.eq(order.id))
        .order_by_desc(ShopPaymentAttemptColumn::CreatedAt)
        .one(&tx)
        .await?
        .ok_or_else(|| bad_request("Payment attempt not found for shop order"))?;

    let now = chrono::Utc::now();
    let mut payment_active: ShopPaymentAttemptActiveModel = payment_attempt.into();
    payment_active.status = Set(PAYMENT_STATUS_SUCCEEDED.to_string());
    payment_active.response_payload = Set(json!({
        "status": PAYMENT_STATUS_SUCCEEDED,
        "completedAt": now,
        "syncSource": "shop.mock_complete",
    })
    .to_string());
    payment_active.updated_at = Set(now);
    payment_active.update(&tx).await?;

    process_successful_payment_in_tx(
        &tx,
        shop_config,
        receipts_config,
        &order,
        OwnershipActor::user(user_id),
    )
    .await?;

    tx.commit().await?;
    get_order_for_user(db, user_id, order_id, shop_config, receipts_config)
        .await?
        .ok_or_else(|| anyhow!("Completed order could not be loaded"))
}

async fn reconcile_local_pending_order_in_tx(
    db: &impl ConnectionTrait,
    shop_config: &ShopConfig,
    order: &ShopOrderModel,
) -> Result<ShopOrderModel> {
    if should_expire_pending_order(order, shop_config, chrono::Utc::now()) {
        return mark_order_payment_expired_in_tx(db, order).await;
    }

    Ok(order.clone())
}

async fn process_successful_payment_in_tx(
    db: &impl ConnectionTrait,
    shop_config: &ShopConfig,
    receipts_config: &ReceiptsConfig,
    order: &ShopOrderModel,
    actor: OwnershipActor,
) -> Result<ShopOrderModel> {
    if matches!(
        order.status.as_str(),
        ORDER_STATUS_FULFILLED
            | ORDER_STATUS_FULFILLMENT_FAILED
            | ORDER_STATUS_PAYMENT_VALIDATION_FAILED
    ) {
        return Ok(order.clone());
    }

    let paid_order = mark_order_paid_in_tx(db, order).await?;
    let claimed_order = claim_order_fulfillment_in_tx(db, &paid_order, shop_config).await?;
    if claimed_order.status != ORDER_STATUS_FULFILLMENT_IN_PROGRESS {
        return Ok(claimed_order);
    }

    let fulfillment = fulfill_order_in_tx(db, receipts_config, &claimed_order, actor).await;
    match fulfillment {
        Ok(_) => reload_order_in_tx(db, order.id).await,
        Err(error) => {
            let mut failed: ShopOrderActiveModel = claimed_order.into();
            failed.status = Set(ORDER_STATUS_FULFILLMENT_FAILED.to_string());
            failed.failure_problem = Set(Some(strip_problem_prefix(error.to_string())));
            failed.updated_at = Set(chrono::Utc::now());
            failed.update(db).await?;
            reload_order_in_tx(db, order.id).await
        }
    }
}

async fn claim_order_fulfillment_in_tx(
    db: &impl ConnectionTrait,
    order: &ShopOrderModel,
    shop_config: &ShopConfig,
) -> Result<ShopOrderModel> {
    if order.status == ORDER_STATUS_FULFILLED || order.status == ORDER_STATUS_FULFILLMENT_FAILED {
        return Ok(order.clone());
    }

    let now = chrono::Utc::now();
    let reclaim_before =
        now - chrono::Duration::seconds(fulfillment_retry_after_seconds(shop_config));
    let update_result = ShopOrder::update_many()
        .set(ShopOrderActiveModel {
            status: Set(ORDER_STATUS_FULFILLMENT_IN_PROGRESS.to_string()),
            updated_at: Set(now),
            failure_problem: Set(None),
            ..Default::default()
        })
        .filter(ShopOrderColumn::Id.eq(order.id))
        .filter(
            Condition::any()
                .add(ShopOrderColumn::Status.eq(ORDER_STATUS_PAID))
                .add(
                    Condition::all()
                        .add(ShopOrderColumn::Status.eq(ORDER_STATUS_FULFILLMENT_IN_PROGRESS))
                        .add(ShopOrderColumn::UpdatedAt.lte(reclaim_before)),
                ),
        )
        .exec(db)
        .await?;

    if update_result.rows_affected == 0 {
        return reload_order_in_tx(db, order.id).await;
    }

    reload_order_in_tx(db, order.id).await
}

async fn mark_order_payment_expired_in_tx(
    db: &impl ConnectionTrait,
    order: &ShopOrderModel,
) -> Result<ShopOrderModel> {
    if order.status == ORDER_STATUS_PAYMENT_EXPIRED {
        return Ok(order.clone());
    }
    if order.status != ORDER_STATUS_PENDING_PAYMENT {
        return Ok(order.clone());
    }

    let update_result = ShopOrder::update_many()
        .set(ShopOrderActiveModel {
            status: Set(ORDER_STATUS_PAYMENT_EXPIRED.to_string()),
            failure_problem: Set(Some(
                "Payment session expired before the shop order was paid".to_string(),
            )),
            updated_at: Set(chrono::Utc::now()),
            ..Default::default()
        })
        .filter(ShopOrderColumn::Id.eq(order.id))
        .filter(ShopOrderColumn::Status.eq(ORDER_STATUS_PENDING_PAYMENT))
        .exec(db)
        .await?;

    if update_result.rows_affected == 0 {
        return reload_order_in_tx(db, order.id).await;
    }

    reload_order_in_tx(db, order.id).await
}

async fn mark_order_payment_validation_failed_in_tx(
    db: &impl ConnectionTrait,
    order: &ShopOrderModel,
    problem: String,
) -> Result<ShopOrderModel> {
    if matches!(
        order.status.as_str(),
        ORDER_STATUS_FULFILLED
            | ORDER_STATUS_FULFILLMENT_IN_PROGRESS
            | ORDER_STATUS_FULFILLMENT_FAILED
    ) {
        return Ok(order.clone());
    }

    let update_result = ShopOrder::update_many()
        .set(ShopOrderActiveModel {
            status: Set(ORDER_STATUS_PAYMENT_VALIDATION_FAILED.to_string()),
            failure_problem: Set(Some(problem)),
            updated_at: Set(chrono::Utc::now()),
            ..Default::default()
        })
        .filter(ShopOrderColumn::Id.eq(order.id))
        .filter(ShopOrderColumn::Status.is_in([
            ORDER_STATUS_PENDING_PAYMENT,
            ORDER_STATUS_PAYMENT_EXPIRED,
            ORDER_STATUS_PAID,
        ]))
        .exec(db)
        .await?;

    if update_result.rows_affected == 0 {
        return reload_order_in_tx(db, order.id).await;
    }

    reload_order_in_tx(db, order.id).await
}

async fn reload_order_in_tx(db: &impl ConnectionTrait, order_id: Uuid) -> Result<ShopOrderModel> {
    ShopOrder::find_by_id(order_id)
        .one(db)
        .await?
        .ok_or_else(|| anyhow!("Shop order could not be reloaded"))
}

async fn latest_payment_attempt_for_order(
    db: &DatabaseConnection,
    order_id: Uuid,
) -> Result<Option<ShopPaymentAttemptModel>> {
    ShopPaymentAttempt::find()
        .filter(ShopPaymentAttemptColumn::OrderId.eq(order_id))
        .order_by_desc(ShopPaymentAttemptColumn::CreatedAt)
        .one(db)
        .await
        .map_err(Into::into)
}

fn validate_yookassa_payment_matches_order(
    order: &ShopOrderModel,
    payment: &yookassa::YooKassaPayment,
) -> Result<()> {
    if !payment.paid {
        return Err(bad_request(
            "YooKassa reported a succeeded payment, but the payment is not marked as paid",
        ));
    }

    let amount = payment
        .amount
        .as_ref()
        .ok_or_else(|| bad_request("YooKassa payment payload is missing amount"))?;
    let expected_value = format!("{}.00", order.total_price_rub);
    if amount.value != expected_value {
        return Err(bad_request(format!(
            "YooKassa payment amount mismatch: expected {expected_value} RUB, got {} {}",
            amount.value, amount.currency
        )));
    }
    if amount.currency != "RUB" {
        return Err(bad_request(format!(
            "YooKassa payment currency mismatch: expected RUB, got {}",
            amount.currency
        )));
    }
    if payment.metadata.get("orderId") != Some(&order.id.to_string()) {
        return Err(bad_request(format!(
            "YooKassa payment metadata mismatch: expected metadata.orderId='{}'",
            order.id
        )));
    }

    Ok(())
}

fn should_reconcile_order(order: &ShopOrderModel) -> bool {
    matches!(
        order.status.as_str(),
        ORDER_STATUS_PENDING_PAYMENT
            | ORDER_STATUS_PAYMENT_EXPIRED
            | ORDER_STATUS_PAID
            | ORDER_STATUS_FULFILLMENT_IN_PROGRESS
    )
}

fn should_expire_pending_order(
    order: &ShopOrderModel,
    shop_config: &ShopConfig,
    now: chrono::DateTime<chrono::Utc>,
) -> bool {
    order.status == ORDER_STATUS_PENDING_PAYMENT
        && effective_payment_expires_at(order, shop_config)
            .is_some_and(|expires_at| expires_at <= now)
}

fn effective_payment_expires_at(
    order: &ShopOrderModel,
    shop_config: &ShopConfig,
) -> Option<chrono::DateTime<chrono::Utc>> {
    order.payment_expires_at.or_else(|| {
        Some(order.created_at + chrono::Duration::seconds(shop_config.pending_payment_ttl_seconds))
    })
}

fn fulfillment_retry_after_seconds(shop_config: &ShopConfig) -> i64 {
    std::cmp::max(
        (shop_config
            .reconciliation_interval_seconds
            .saturating_mul(3)) as i64,
        FULFILLMENT_RETRY_AFTER_SECONDS,
    )
}

fn reconcilable_order_status_condition() -> Condition {
    Condition::all().add(ShopOrderColumn::Status.is_in([
        ORDER_STATUS_PENDING_PAYMENT,
        ORDER_STATUS_PAYMENT_EXPIRED,
        ORDER_STATUS_PAID,
        ORDER_STATUS_FULFILLMENT_IN_PROGRESS,
    ]))
}

async fn fulfill_order_in_tx(
    db: &impl ConnectionTrait,
    receipts_config: &ReceiptsConfig,
    order: &ShopOrderModel,
    actor: OwnershipActor,
) -> Result<()> {
    let ownership_model = OwnershipModel::parse(&order.ownership_model)
        .map_err(|error| bad_request(error.to_string()))?;
    let quantity = order.quantity;
    let context = OperationContext {
        reason_code: Some("shop_purchase".to_string()),
        reason_text: Some(format!("Purchased shop product '{}'", order.product_key)),
        metadata: json!({
            "orderId": order.id,
            "productKey": order.product_key,
            "paymentProvider": order.payment_provider,
        }),
    };

    match ownership_model {
        OwnershipModel::Stackable => {
            let unit_amount = order
                .stackable_amount_per_unit
                .ok_or_else(|| bad_request("Stackable order is missing stackable amount"))?;
            let granted_amount = unit_amount
                .checked_mul(quantity)
                .ok_or_else(|| bad_request("Granted stackable amount overflow"))?;
            recheck_stackable_limit(
                db,
                order.user_id,
                order.asset_definition_id,
                granted_amount,
                order.max_owned_amount_snapshot,
            )
            .await?;
            inventory::add_stackable_in_tx(
                db,
                StackableMutation {
                    user_id: order.user_id,
                    asset_key: order.asset_key.clone(),
                    amount: granted_amount,
                    actor,
                    context,
                },
            )
            .await?;
        }
        OwnershipModel::Entitlement => {
            ensure_entitlement_not_owned(db, order.user_id, order.asset_definition_id).await?;
            grant_entitlement_in_tx(
                db,
                EntitlementMutation {
                    user_id: order.user_id,
                    asset_key: order.asset_key.clone(),
                    actor,
                    context,
                },
            )
            .await?;
        }
        OwnershipModel::Expirable => {
            let unit_duration = order
                .expirable_duration_seconds_per_unit
                .ok_or_else(|| bad_request("Subscription order is missing duration"))?;
            let granted_duration = unit_duration
                .checked_mul(quantity)
                .ok_or_else(|| bad_request("Granted duration overflow"))?;
            inventory::prolong_expirable_in_tx(
                db,
                ProlongExpirableMutation {
                    user_id: order.user_id,
                    asset_key: order.asset_key.clone(),
                    duration_seconds: granted_duration,
                    actor,
                    context,
                },
            )
            .await?;
        }
    }

    let mut active: ShopOrderActiveModel = order.clone().into();
    active.status = Set(ORDER_STATUS_FULFILLED.to_string());
    active.fulfilled_at = Set(Some(chrono::Utc::now()));
    active.updated_at = Set(chrono::Utc::now());
    active.failure_problem = Set(None);
    let fulfilled_order = active.update(db).await?;
    receipts::ensure_receipt_for_order_in_tx(db, &fulfilled_order, receipts_config).await?;
    queue_shop_purchase_completed_notification(
        db,
        order.user_id,
        order.id,
        &order.product_key,
        &order.product_name,
        order.quantity,
    )
    .await?;
    Ok(())
}

async fn replace_product_locales(
    db: &impl ConnectionTrait,
    product_id: Uuid,
    locales: &[NormalizedLocaleInput],
    now: chrono::DateTime<chrono::Utc>,
) -> Result<()> {
    ShopProductLocale::delete_many()
        .filter(ShopProductLocaleColumn::ProductId.eq(product_id))
        .exec(db)
        .await?;

    for locale in locales {
        ShopProductLocaleActiveModel {
            product_id: Set(product_id),
            locale: Set(locale.locale.clone()),
            name: Set(locale.name.clone()),
            description: Set(locale.description.clone()),
            created_at: Set(now),
            updated_at: Set(now),
        }
        .insert(db)
        .await?;
    }
    Ok(())
}

async fn load_product_bundle_by_key(
    db: &impl ConnectionTrait,
    product_key: &str,
) -> Result<
    Option<(
        ShopProductModel,
        AssetDefinitionModel,
        Vec<ShopProductLocaleModel>,
    )>,
> {
    let product = ShopProduct::find()
        .filter(ShopProductColumn::Key.eq(product_key))
        .one(db)
        .await?;
    let Some(product) = product else {
        return Ok(None);
    };
    let asset = AssetDefinition::find_by_id(product.asset_definition_id)
        .one(db)
        .await?
        .ok_or_else(|| anyhow!("Missing asset definition for shop product"))?;
    let locales = ShopProductLocale::find()
        .filter(ShopProductLocaleColumn::ProductId.eq(product.id))
        .order_by_asc(ShopProductLocaleColumn::Locale)
        .all(db)
        .await?;
    Ok(Some((product, asset, locales)))
}

async fn map_orders_with_attempts(
    db: &DatabaseConnection,
    orders: Vec<ShopOrderModel>,
    shop_config: &ShopConfig,
) -> Result<Vec<ShopOrderView>> {
    let order_ids = orders.iter().map(|order| order.id).collect::<Vec<_>>();
    let attempts = if order_ids.is_empty() {
        Vec::new()
    } else {
        ShopPaymentAttempt::find()
            .filter(ShopPaymentAttemptColumn::OrderId.is_in(order_ids.clone()))
            .order_by_desc(ShopPaymentAttemptColumn::CreatedAt)
            .all(db)
            .await?
    };
    let receipts = if order_ids.is_empty() {
        Vec::new()
    } else {
        ShopReceipt::find()
            .filter(ShopReceiptColumn::OrderId.is_in(order_ids))
            .all(db)
            .await?
    };

    let mut result = Vec::with_capacity(orders.len());
    for order in orders {
        let payment = attempts
            .iter()
            .find(|attempt| attempt.order_id == order.id)
            .cloned()
            .map(map_payment_attempt_view);
        let receipt = receipts
            .iter()
            .find(|receipt| receipt.order_id == order.id)
            .cloned()
            .map(receipts::map_receipt_view);
        result.push(map_order_view(order, payment, receipt, shop_config)?);
    }
    Ok(result)
}

fn map_product_view(
    product: ShopProductModel,
    asset: AssetDefinitionModel,
    locales: Vec<ShopProductLocaleModel>,
    locale: Option<&str>,
    now: chrono::DateTime<chrono::Utc>,
) -> Result<ShopProductView> {
    let ownership_model = parse_ownership_model(&asset)?;
    let localized = localize_product(
        &locales,
        locale,
        &asset.display_name,
        asset.description.as_deref(),
    );
    let is_available_now = is_product_available_now(product.starts_at, product.ends_at, now);

    Ok(ShopProductView {
        id: product.id,
        key: product.key,
        asset_definition_id: asset.id,
        asset_key: asset.key,
        asset_display_name: asset.display_name,
        ownership_model,
        price_rub: product.price_rub,
        stackable_amount: product.stackable_amount,
        duration_seconds: product.expirable_duration_seconds,
        max_per_purchase: product.max_per_purchase,
        max_owned_amount: product.max_owned_amount,
        localized_name: localized.name,
        localized_description: localized.description,
        locales: locales
            .into_iter()
            .map(|locale| ShopProductLocaleView {
                locale: locale.locale,
                name: locale.name,
                description: locale.description,
            })
            .collect(),
        is_active: product.is_active,
        is_public: product.is_public,
        is_available_now,
        sort_order: product.sort_order,
        starts_at: product.starts_at,
        ends_at: product.ends_at,
        metadata: serde_json::from_str(&product.metadata)?,
        created_at: product.created_at,
        updated_at: product.updated_at,
    })
}

fn map_payment_attempt_view(item: ShopPaymentAttemptModel) -> ShopPaymentAttemptView {
    ShopPaymentAttemptView {
        id: item.id,
        provider: item.provider,
        provider_payment_id: item.provider_payment_id,
        checkout_token: item.checkout_token,
        checkout_url: item.checkout_url,
        status: item.status,
        created_at: item.created_at,
        updated_at: item.updated_at,
    }
}

fn map_order_view(
    order: ShopOrderModel,
    payment: Option<ShopPaymentAttemptView>,
    receipt: Option<ShopReceiptView>,
    shop_config: &ShopConfig,
) -> Result<ShopOrderView> {
    let ownership_model = OwnershipModel::parse(&order.ownership_model)
        .map_err(|error| bad_request(error.to_string()))?;
    let granted_amount = order
        .stackable_amount_per_unit
        .and_then(|amount| amount.checked_mul(order.quantity));
    let granted_duration_seconds = order
        .expirable_duration_seconds_per_unit
        .and_then(|duration| duration.checked_mul(order.quantity));
    let payment_expires_at = effective_payment_expires_at(&order, shop_config);

    Ok(ShopOrderView {
        id: order.id,
        user_id: order.user_id,
        product_id: order.product_id,
        product_key: order.product_key,
        product_name: order.product_name,
        product_description: order.product_description,
        product_locale: order.product_locale,
        asset_definition_id: order.asset_definition_id,
        asset_key: order.asset_key,
        ownership_model,
        quantity: order.quantity,
        unit_price_rub: order.unit_price_rub,
        total_price_rub: order.total_price_rub,
        granted_amount,
        granted_duration_seconds,
        max_owned_amount: order.max_owned_amount_snapshot,
        payment_provider: order.payment_provider,
        status: order.status,
        failure_problem: order.failure_problem,
        payment_expires_at,
        paid_at: order.paid_at,
        fulfilled_at: order.fulfilled_at,
        payment,
        receipt,
        metadata: serde_json::from_str(&order.metadata)?,
        created_at: order.created_at,
        updated_at: order.updated_at,
    })
}

async fn get_product_asset_by_key(
    db: &impl ConnectionTrait,
    asset_key: &str,
) -> Result<AssetDefinitionModel> {
    let asset = AssetDefinition::find()
        .filter(AssetDefinitionColumn::Key.eq(asset_key))
        .one(db)
        .await?
        .ok_or_else(|| not_found("Asset not found"))?;
    if !asset.is_active {
        return Err(unprocessable("Shop product asset must be active"));
    }
    if asset.is_currency {
        return Err(bad_request(
            "Currency assets cannot be sold through the shop product catalog",
        ));
    }
    Ok(asset)
}

async fn ensure_purchase_allowed(
    db: &impl ConnectionTrait,
    user_id: Uuid,
    product: &ShopProductModel,
    asset: &AssetDefinitionModel,
    quantity: i64,
    max_owned_amount: Option<i64>,
) -> Result<()> {
    match parse_ownership_model(asset)? {
        OwnershipModel::Stackable => {
            let per_unit = product
                .stackable_amount
                .ok_or_else(|| bad_request("Shop product is missing stackable amount"))?;
            let granted_amount = per_unit
                .checked_mul(quantity)
                .ok_or_else(|| bad_request("Granted stackable amount overflow"))?;
            recheck_stackable_limit(db, user_id, asset.id, granted_amount, max_owned_amount)
                .await?;
        }
        OwnershipModel::Entitlement => {
            ensure_entitlement_not_owned(db, user_id, asset.id).await?;
        }
        OwnershipModel::Expirable => {}
    }
    Ok(())
}

async fn recheck_stackable_limit(
    db: &impl ConnectionTrait,
    user_id: Uuid,
    asset_definition_id: Uuid,
    granted_amount: i64,
    max_owned_amount: Option<i64>,
) -> Result<()> {
    if let Some(limit) = max_owned_amount {
        let current_amount = UserStackableAsset::find_by_id((user_id, asset_definition_id))
            .one(db)
            .await?
            .map(|holding| holding.amount)
            .unwrap_or(0);
        let new_total = current_amount
            .checked_add(granted_amount)
            .ok_or_else(|| bad_request("Stackable ownership overflow"))?;
        if new_total > limit {
            return Err(unprocessable(format!(
                "Purchase would exceed the ownership limit: current amount is {}, granted amount is {}, and maximum allowed is {}",
                current_amount, granted_amount, limit
            )));
        }
    }
    Ok(())
}

async fn ensure_entitlement_not_owned(
    db: &impl ConnectionTrait,
    user_id: Uuid,
    asset_definition_id: Uuid,
) -> Result<()> {
    if UserEntitlement::find_by_id((user_id, asset_definition_id))
        .one(db)
        .await?
        .is_some()
    {
        return Err(conflict("User already owns this entitlement"));
    }
    Ok(())
}

fn validate_order_quantity(
    product: &ShopProductModel,
    asset: &AssetDefinitionModel,
    quantity: i64,
) -> Result<()> {
    let ownership_model = parse_ownership_model(asset)?;
    match ownership_model {
        OwnershipModel::Stackable => {
            if let Some(max_per_purchase) = product.max_per_purchase
                && quantity > max_per_purchase
            {
                return Err(unprocessable(format!(
                    "Quantity exceeds max_per_purchase: requested {}, allowed {}",
                    quantity, max_per_purchase
                )));
            }
        }
        OwnershipModel::Entitlement | OwnershipModel::Expirable => {
            if quantity != 1 {
                return Err(bad_request(
                    "Quantity must be exactly 1 for entitlement and subscription products",
                ));
            }
        }
    }
    Ok(())
}

fn validate_reward_configuration(
    ownership_model: OwnershipModel,
    stackable_amount: Option<i64>,
    duration_seconds: Option<i64>,
    max_per_purchase: Option<i64>,
    max_owned_amount: Option<i64>,
) -> Result<()> {
    match ownership_model {
        OwnershipModel::Stackable => {
            let stackable_amount = stackable_amount.ok_or_else(|| {
                bad_request("stackable_amount is required for stackable products")
            })?;
            if stackable_amount <= 0 {
                return Err(bad_request("stackable_amount must be positive"));
            }
            if duration_seconds.is_some() {
                return Err(bad_request(
                    "durationSeconds is not allowed for stackable products",
                ));
            }
            if let Some(max_per_purchase) = max_per_purchase
                && max_per_purchase <= 0
            {
                return Err(bad_request("max_per_purchase must be positive"));
            }
            if let Some(max_owned_amount) = max_owned_amount {
                if max_owned_amount <= 0 {
                    return Err(bad_request("max_owned_amount must be positive"));
                }
                if max_owned_amount < stackable_amount {
                    return Err(bad_request(
                        "max_owned_amount cannot be lower than stackable_amount",
                    ));
                }
            }
        }
        OwnershipModel::Entitlement => {
            if stackable_amount.is_some()
                || duration_seconds.is_some()
                || max_per_purchase.is_some()
                || max_owned_amount.is_some()
            {
                return Err(bad_request(
                    "Entitlement products must not define stackable amount, duration, or ownership limits",
                ));
            }
        }
        OwnershipModel::Expirable => {
            let duration_seconds = duration_seconds.ok_or_else(|| {
                bad_request("durationSeconds is required for subscription products")
            })?;
            if duration_seconds <= 0 {
                return Err(bad_request("durationSeconds must be positive"));
            }
            if stackable_amount.is_some()
                || max_per_purchase.is_some()
                || max_owned_amount.is_some()
            {
                return Err(bad_request(
                    "Subscription products must not define stackable amount or stackable ownership limits",
                ));
            }
        }
    }
    Ok(())
}

fn validate_price_rub(price_rub: i64) -> Result<i64> {
    if price_rub <= 0 {
        return Err(bad_request(
            "price_rub must be a positive integer amount in rubles",
        ));
    }
    Ok(price_rub)
}

fn validate_availability_window(
    starts_at: Option<chrono::DateTime<chrono::Utc>>,
    ends_at: Option<chrono::DateTime<chrono::Utc>>,
) -> Result<()> {
    if let (Some(starts_at), Some(ends_at)) = (starts_at, ends_at)
        && starts_at > ends_at
    {
        return Err(bad_request("starts_at cannot be later than ends_at"));
    }
    Ok(())
}

fn ensure_product_is_sellable(
    product: &ShopProductModel,
    now: chrono::DateTime<chrono::Utc>,
) -> Result<()> {
    if !product.is_active || !product.is_public {
        return Err(not_found("Shop product not found"));
    }
    if !is_product_available_now(product.starts_at, product.ends_at, now) {
        return Err(unprocessable(
            "Shop product is not available for purchase right now",
        ));
    }
    Ok(())
}

fn is_product_available_now(
    starts_at: Option<chrono::DateTime<chrono::Utc>>,
    ends_at: Option<chrono::DateTime<chrono::Utc>>,
    now: chrono::DateTime<chrono::Utc>,
) -> bool {
    let after_start = starts_at.is_none_or(|starts_at| starts_at <= now);
    let before_end = ends_at.is_none_or(|ends_at| ends_at >= now);
    after_start && before_end
}

fn locales_for_product(
    locales: &[ShopProductLocaleModel],
    product_id: Uuid,
) -> Vec<ShopProductLocaleModel> {
    locales
        .iter()
        .filter(|locale| locale.product_id == product_id)
        .cloned()
        .collect()
}

fn localize_product(
    locales: &[ShopProductLocaleModel],
    requested_locale: Option<&str>,
    fallback_name: &str,
    fallback_description: Option<&str>,
) -> LocalizedProductContent {
    if let Some(locale) = requested_locale {
        if let Some(found) = locales.iter().find(|item| item.locale == locale) {
            return LocalizedProductContent {
                name: found.name.clone(),
                description: found.description.clone(),
            };
        }
        let primary = locale.split('-').next().unwrap_or(locale);
        if let Some(found) = locales.iter().find(|item| item.locale == primary) {
            return LocalizedProductContent {
                name: found.name.clone(),
                description: found.description.clone(),
            };
        }
    }
    if let Some(found) = locales.iter().find(|item| item.locale == "en") {
        return LocalizedProductContent {
            name: found.name.clone(),
            description: found.description.clone(),
        };
    }
    if let Some(found) = locales.first() {
        return LocalizedProductContent {
            name: found.name.clone(),
            description: found.description.clone(),
        };
    }
    LocalizedProductContent {
        name: fallback_name.to_string(),
        description: fallback_description.map(str::to_string),
    }
}

fn normalize_locale_inputs(locales: Vec<ProductLocaleInput>) -> Result<Vec<NormalizedLocaleInput>> {
    if locales.is_empty() {
        return Err(bad_request(
            "At least one localized shop product title is required",
        ));
    }

    let mut normalized = Vec::with_capacity(locales.len());
    let mut seen = std::collections::HashSet::new();
    for locale in locales {
        let locale_key = normalize_required_locale(locale.locale)?;
        if !seen.insert(locale_key.clone()) {
            return Err(bad_request(format!(
                "Duplicate locale '{}' in shop product locales",
                locale_key
            )));
        }
        let name = locale.name.trim().to_string();
        if name.is_empty() {
            return Err(bad_request(format!(
                "Localized product name for '{}' cannot be empty",
                locale_key
            )));
        }
        normalized.push(NormalizedLocaleInput {
            locale: locale_key,
            name,
            description: locale
                .description
                .map(|description| description.trim().to_string())
                .filter(|description| !description.is_empty()),
        });
    }

    Ok(normalized)
}

fn normalize_optional_locale(locale: Option<String>) -> Result<Option<String>> {
    locale.map(normalize_required_locale).transpose()
}

fn normalize_required_locale(locale: String) -> Result<String> {
    let normalized = locale.trim().to_lowercase();
    if normalized.is_empty() {
        return Err(bad_request("locale cannot be empty"));
    }
    if !normalized
        .chars()
        .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-')
    {
        return Err(bad_request(
            "locale must contain only lowercase latin letters, numbers, and hyphens",
        ));
    }
    Ok(normalized)
}

fn validate_shop_key(key: &str) -> Result<String> {
    validate_asset_key(key).map_err(|error| bad_request(error.to_string()))
}

fn parse_ownership_model(asset: &AssetDefinitionModel) -> Result<OwnershipModel> {
    OwnershipModel::parse(&asset.ownership_model).map_err(|error| bad_request(error.to_string()))
}

async fn ensure_user_exists(db: &impl ConnectionTrait, user_id: Uuid) -> Result<()> {
    if User::find_by_id(user_id).one(db).await?.is_none() {
        return Err(not_found("User not found"));
    }
    Ok(())
}

fn bad_request(message: impl Into<String>) -> anyhow::Error {
    anyhow!("bad_request: {}", message.into())
}

fn not_found(message: impl Into<String>) -> anyhow::Error {
    anyhow!("not_found: {}", message.into())
}

fn conflict(message: impl Into<String>) -> anyhow::Error {
    anyhow!("conflict: {}", message.into())
}

fn unprocessable(message: impl Into<String>) -> anyhow::Error {
    anyhow!("unprocessable: {}", message.into())
}

fn unavailable(message: impl Into<String>) -> anyhow::Error {
    anyhow!("unavailable: {}", message.into())
}

pub fn strip_problem_prefix(message: String) -> String {
    for prefix in [
        "bad_request: ",
        "not_found: ",
        "conflict: ",
        "unprocessable: ",
        "unavailable: ",
    ] {
        if let Some(stripped) = message.strip_prefix(prefix) {
            return stripped.to_string();
        }
    }
    message
}

#[derive(Clone)]
struct LocalizedProductContent {
    name: String,
    description: Option<String>,
}

#[derive(Clone)]
struct NormalizedLocaleInput {
    locale: String,
    name: String,
    description: Option<String>,
}
