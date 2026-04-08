use anyhow::{Result, anyhow, bail};
use sea_orm::{
    ActiveModelTrait, ActiveValue::NotSet, ActiveValue::Set, ColumnTrait, ConnectionTrait,
    EntityTrait, QueryFilter, QueryOrder, TransactionTrait,
};
use serde::Serialize;
use serde_json::Value;
use uuid::Uuid;

use crate::entities::{
    AssetDefinition, AssetDefinitionColumn, AssetDefinitionModel, User, WalletBalance,
    WalletBalanceActiveModel, WalletBalanceColumn, WalletBalanceModel, WalletTransaction,
    WalletTransactionActiveModel, WalletTransactionColumn, WalletTransactionModel,
};
use crate::services::ownership::types::{
    OperationContext, OwnershipActor, OwnershipModel, normalize_metadata, validate_asset_key,
};

#[derive(Clone, Debug, Serialize)]
pub struct WalletBalanceView {
    pub user_id: Uuid,
    pub currency_asset_definition_id: Uuid,
    pub currency_key: String,
    pub balance: i64,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Clone, Debug, Serialize)]
pub struct WalletTransactionView {
    pub id: i64,
    pub currency_asset_definition_id: Uuid,
    pub currency_key: String,
    pub operation_type: String,
    pub actor_kind: String,
    pub actor_user_id: Option<Uuid>,
    pub actor_service_name: Option<String>,
    pub delta: i64,
    pub balance_after: i64,
    pub reason_code: Option<String>,
    pub reason_text: Option<String>,
    pub metadata: Value,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Clone, Debug)]
pub struct WalletMutation {
    pub user_id: Uuid,
    pub currency_key: String,
    pub amount: i64,
    pub actor: OwnershipActor,
    pub context: OperationContext,
}

pub async fn get_wallet(db: &sea_orm::DatabaseConnection, user_id: Uuid) -> Result<Vec<WalletBalanceView>> {
    ensure_user_exists(db, user_id).await?;
    let balances = WalletBalance::find()
        .filter(WalletBalanceColumn::UserId.eq(user_id))
        .order_by_desc(WalletBalanceColumn::UpdatedAt)
        .all(db)
        .await?;
    let assets = AssetDefinition::find().all(db).await?;

    balances
        .into_iter()
        .map(|balance| {
            let asset = assets
                .iter()
                .find(|asset| asset.id == balance.currency_asset_definition_id)
                .ok_or_else(|| anyhow!("Missing asset definition for wallet balance"))?;
            Ok(WalletBalanceView {
                user_id: balance.user_id,
                currency_asset_definition_id: balance.currency_asset_definition_id,
                currency_key: asset.key.clone(),
                balance: balance.balance,
                updated_at: balance.updated_at,
            })
        })
        .collect()
}

pub async fn get_wallet_balance(
    db: &sea_orm::DatabaseConnection,
    user_id: Uuid,
    currency_key: &str,
) -> Result<WalletBalanceView> {
    ensure_user_exists(db, user_id).await?;
    let asset = get_currency_asset_by_key(db, currency_key).await?;
    let now = chrono::Utc::now();
    let balance = WalletBalance::find_by_id((user_id, asset.id)).one(db).await?;
    let balance = balance.unwrap_or(WalletBalanceModel {
        user_id,
        currency_asset_definition_id: asset.id,
        balance: 0,
        updated_at: now,
    });

    Ok(WalletBalanceView {
        user_id,
        currency_asset_definition_id: asset.id,
        currency_key: asset.key,
        balance: balance.balance,
        updated_at: balance.updated_at,
    })
}

pub async fn get_wallet_transactions(
    db: &sea_orm::DatabaseConnection,
    user_id: Uuid,
    currency_key: &str,
) -> Result<Vec<WalletTransactionView>> {
    ensure_user_exists(db, user_id).await?;
    let asset = get_currency_asset_by_key(db, currency_key).await?;
    WalletTransaction::find()
        .filter(WalletTransactionColumn::UserId.eq(user_id))
        .filter(WalletTransactionColumn::CurrencyAssetDefinitionId.eq(asset.id))
        .order_by_desc(WalletTransactionColumn::CreatedAt)
        .all(db)
        .await?
        .into_iter()
        .map(|tx| map_wallet_transaction(tx, &asset.key))
        .collect()
}

pub async fn credit(db: &sea_orm::DatabaseConnection, mutation: WalletMutation) -> Result<WalletBalanceView> {
    mutate_balance(db, mutation, "credit", |current, amount| Ok(current + amount)).await
}

pub async fn debit(db: &sea_orm::DatabaseConnection, mutation: WalletMutation) -> Result<WalletBalanceView> {
    mutate_balance(db, mutation, "debit", |current, amount| {
        if current < amount {
            bail!("Insufficient wallet balance");
        }
        Ok(current - amount)
    })
    .await
}

pub async fn adjust_balance(
    db: &sea_orm::DatabaseConnection,
    mutation: WalletMutation,
) -> Result<WalletBalanceView> {
    mutation.actor.validate()?;
    if mutation.amount < 0 {
        bail!("Balance must be non-negative");
    }
    let currency_key = validate_asset_key(&mutation.currency_key)?;
    let tx = db.begin().await?;
    ensure_user_exists(&tx, mutation.user_id).await?;
    let asset = get_currency_asset_by_key(&tx, &currency_key).await?;
    let now = chrono::Utc::now();
    let existing = WalletBalance::find_by_id((mutation.user_id, asset.id)).one(&tx).await?;
    let current = existing.as_ref().map_or(0, |balance| balance.balance);
    let delta = mutation.amount - current;

    let balance = write_balance_state(&tx, mutation.user_id, asset.id, mutation.amount, now, existing).await?;
    write_wallet_transaction(
        &tx,
        mutation.user_id,
        asset.id,
        "adjustment",
        &mutation.actor,
        &mutation.context,
        delta,
        mutation.amount,
    )
    .await?;

    tx.commit().await?;
    Ok(WalletBalanceView {
        user_id: mutation.user_id,
        currency_asset_definition_id: asset.id,
        currency_key: asset.key,
        balance: balance.balance,
        updated_at: balance.updated_at,
    })
}

async fn mutate_balance<F>(
    db: &sea_orm::DatabaseConnection,
    mutation: WalletMutation,
    operation_type: &str,
    compute_new_balance: F,
) -> Result<WalletBalanceView>
where
    F: Fn(i64, i64) -> Result<i64>,
{
    mutation.actor.validate()?;
    if mutation.amount <= 0 {
        bail!("Amount must be positive");
    }
    let currency_key = validate_asset_key(&mutation.currency_key)?;
    let tx = db.begin().await?;
    ensure_user_exists(&tx, mutation.user_id).await?;
    let asset = get_currency_asset_by_key(&tx, &currency_key).await?;
    let now = chrono::Utc::now();
    let existing = WalletBalance::find_by_id((mutation.user_id, asset.id)).one(&tx).await?;
    let current = existing.as_ref().map_or(0, |balance| balance.balance);
    let new_balance = compute_new_balance(current, mutation.amount)?;
    if new_balance < 0 {
        bail!("Balance must not become negative");
    }

    let balance = write_balance_state(&tx, mutation.user_id, asset.id, new_balance, now, existing).await?;
    let delta = if operation_type == "debit" { -mutation.amount } else { mutation.amount };
    write_wallet_transaction(
        &tx,
        mutation.user_id,
        asset.id,
        operation_type,
        &mutation.actor,
        &mutation.context,
        delta,
        new_balance,
    )
    .await?;

    tx.commit().await?;
    Ok(WalletBalanceView {
        user_id: mutation.user_id,
        currency_asset_definition_id: asset.id,
        currency_key: asset.key,
        balance: balance.balance,
        updated_at: balance.updated_at,
    })
}

async fn write_balance_state(
    db: &impl ConnectionTrait,
    user_id: Uuid,
    asset_id: Uuid,
    balance: i64,
    now: chrono::DateTime<chrono::Utc>,
    existing: Option<WalletBalanceModel>,
) -> Result<WalletBalanceModel> {
    Ok(if let Some(existing) = existing {
        let mut active: WalletBalanceActiveModel = existing.into();
        active.balance = Set(balance);
        active.updated_at = Set(now);
        active.update(db).await?
    } else {
        WalletBalanceActiveModel {
            user_id: Set(user_id),
            currency_asset_definition_id: Set(asset_id),
            balance: Set(balance),
            updated_at: Set(now),
        }
        .insert(db)
        .await?
    })
}

async fn get_currency_asset_by_key(db: &impl ConnectionTrait, key: &str) -> Result<AssetDefinitionModel> {
    let asset = AssetDefinition::find()
        .filter(AssetDefinitionColumn::Key.eq(key))
        .one(db)
        .await?
        .ok_or_else(|| anyhow!("Asset not found"))?;
    if !asset.is_active {
        bail!("Asset is inactive");
    }
    if !asset.is_currency {
        bail!("Asset is not a currency");
    }
    if OwnershipModel::parse(&asset.ownership_model)? != OwnershipModel::Stackable {
        bail!("Currency asset must use stackable ownership");
    }
    Ok(asset)
}

async fn ensure_user_exists(db: &impl ConnectionTrait, user_id: Uuid) -> Result<()> {
    if User::find_by_id(user_id).one(db).await?.is_none() {
        bail!("User not found");
    }
    Ok(())
}

async fn write_wallet_transaction(
    db: &impl ConnectionTrait,
    user_id: Uuid,
    currency_asset_definition_id: Uuid,
    operation_type: &str,
    actor: &OwnershipActor,
    context: &OperationContext,
    delta: i64,
    balance_after: i64,
) -> Result<WalletTransactionModel> {
    Ok(WalletTransactionActiveModel {
        id: NotSet,
        user_id: Set(user_id),
        currency_asset_definition_id: Set(currency_asset_definition_id),
        operation_type: Set(operation_type.to_string()),
        actor_kind: Set(actor.kind.as_str().to_string()),
        actor_user_id: Set(actor.user_id),
        actor_service_name: Set(actor.service_name.clone()),
        delta: Set(delta),
        balance_after: Set(balance_after),
        reason_code: Set(context.reason_code.clone()),
        reason_text: Set(context.reason_text.clone()),
        metadata: Set(normalize_metadata(Some(context.metadata.clone())).to_string()),
        created_at: Set(chrono::Utc::now()),
    }
    .insert(db)
    .await?)
}

fn map_wallet_transaction(transaction: WalletTransactionModel, currency_key: &str) -> Result<WalletTransactionView> {
    Ok(WalletTransactionView {
        id: transaction.id,
        currency_asset_definition_id: transaction.currency_asset_definition_id,
        currency_key: currency_key.to_string(),
        operation_type: transaction.operation_type,
        actor_kind: transaction.actor_kind,
        actor_user_id: transaction.actor_user_id,
        actor_service_name: transaction.actor_service_name,
        delta: transaction.delta,
        balance_after: transaction.balance_after,
        reason_code: transaction.reason_code,
        reason_text: transaction.reason_text,
        metadata: serde_json::from_str(&transaction.metadata)?,
        created_at: transaction.created_at,
    })
}
