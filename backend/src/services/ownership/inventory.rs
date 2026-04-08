use anyhow::{Result, anyhow, bail};
use sea_orm::{
    ActiveModelTrait, ActiveValue::NotSet, ActiveValue::Set, ColumnTrait, ConnectionTrait,
    EntityTrait, QueryFilter, QueryOrder, TransactionTrait,
};
use serde::Serialize;
use serde_json::Value;
use uuid::Uuid;

use crate::entities::{
    AssetDefinition, AssetDefinitionColumn, AssetDefinitionModel, InventoryOperation,
    InventoryOperationActiveModel, InventoryOperationColumn, InventoryOperationModel, User,
    UserEntitlement, UserEntitlementActiveModel, UserEntitlementColumn, UserExpirableAsset,
    UserExpirableAssetActiveModel, UserExpirableAssetColumn,
    UserStackableAsset, UserStackableAssetActiveModel, UserStackableAssetColumn,
    UserStackableAssetModel,
};
use crate::services::ownership::types::{
    OperationContext, OwnershipActor, OwnershipModel, normalize_metadata, validate_asset_key,
};

#[derive(Clone, Debug, Serialize)]
pub struct StackableView {
    pub asset_key: String,
    pub asset_definition_id: Uuid,
    pub amount: i64,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Clone, Debug, Serialize)]
pub struct EntitlementView {
    pub asset_key: String,
    pub asset_definition_id: Uuid,
    pub granted_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ExpirableView {
    pub asset_key: String,
    pub asset_definition_id: Uuid,
    pub expires_at: chrono::DateTime<chrono::Utc>,
    pub granted_at: chrono::DateTime<chrono::Utc>,
    pub last_extended_at: Option<chrono::DateTime<chrono::Utc>>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub is_active: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct InventoryView {
    pub user_id: Uuid,
    pub stackables: Vec<StackableView>,
    pub entitlements: Vec<EntitlementView>,
    pub expirables: Vec<ExpirableView>,
}

#[derive(Clone, Debug, Serialize)]
pub struct InventoryPresenceView {
    pub asset_key: String,
    pub ownership_model: OwnershipModel,
    pub exists: bool,
    pub amount: Option<i64>,
    pub is_active: Option<bool>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Clone, Debug, Serialize)]
pub struct InventoryOperationView {
    pub id: i64,
    pub asset_definition_id: Uuid,
    pub asset_key: String,
    pub ownership_model: String,
    pub operation_type: String,
    pub actor_kind: String,
    pub actor_user_id: Option<Uuid>,
    pub actor_service_name: Option<String>,
    pub delta_amount: Option<i64>,
    pub new_amount: Option<i64>,
    pub previous_expires_at: Option<chrono::DateTime<chrono::Utc>>,
    pub new_expires_at: Option<chrono::DateTime<chrono::Utc>>,
    pub reason_code: Option<String>,
    pub reason_text: Option<String>,
    pub metadata: Value,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Clone, Debug)]
pub struct StackableMutation {
    pub user_id: Uuid,
    pub asset_key: String,
    pub amount: i64,
    pub actor: OwnershipActor,
    pub context: OperationContext,
}

#[derive(Clone, Debug)]
pub struct EntitlementMutation {
    pub user_id: Uuid,
    pub asset_key: String,
    pub actor: OwnershipActor,
    pub context: OperationContext,
}

#[derive(Clone, Debug)]
pub struct ProlongExpirableMutation {
    pub user_id: Uuid,
    pub asset_key: String,
    pub duration_seconds: i64,
    pub actor: OwnershipActor,
    pub context: OperationContext,
}

#[derive(Clone, Debug)]
pub struct SetExpirationMutation {
    pub user_id: Uuid,
    pub asset_key: String,
    pub expires_at: chrono::DateTime<chrono::Utc>,
    pub actor: OwnershipActor,
    pub context: OperationContext,
}

pub async fn get_inventory(db: &sea_orm::DatabaseConnection, user_id: Uuid) -> Result<InventoryView> {
    ensure_user_exists(db, user_id).await?;
    let now = chrono::Utc::now();
    Ok(InventoryView {
        user_id,
        stackables: load_stackables(db, user_id).await?,
        entitlements: load_entitlements(db, user_id).await?,
        expirables: load_expirables(db, user_id)
            .await?
            .into_iter()
            .filter(|item| item.expires_at > now)
            .collect(),
    })
}

pub async fn get_stackables(db: &sea_orm::DatabaseConnection, user_id: Uuid) -> Result<Vec<StackableView>> {
    ensure_user_exists(db, user_id).await?;
    load_stackables(db, user_id).await
}

pub async fn get_entitlements(db: &sea_orm::DatabaseConnection, user_id: Uuid) -> Result<Vec<EntitlementView>> {
    ensure_user_exists(db, user_id).await?;
    load_entitlements(db, user_id).await
}

pub async fn get_active_expirables(
    db: &sea_orm::DatabaseConnection,
    user_id: Uuid,
) -> Result<Vec<ExpirableView>> {
    ensure_user_exists(db, user_id).await?;
    let now = chrono::Utc::now();
    Ok(load_expirables(db, user_id)
        .await?
        .into_iter()
        .filter(|item| item.expires_at > now)
        .collect())
}

pub async fn check_presence(
    db: &sea_orm::DatabaseConnection,
    user_id: Uuid,
    asset_key: &str,
) -> Result<InventoryPresenceView> {
    ensure_user_exists(db, user_id).await?;
    let asset = get_non_currency_asset_by_key(db, asset_key).await?;
    let now = chrono::Utc::now();

    match parse_ownership_model(&asset)? {
        OwnershipModel::Stackable => {
            let holding = UserStackableAsset::find_by_id((user_id, asset.id)).one(db).await?;
            Ok(InventoryPresenceView {
                asset_key: asset.key,
                ownership_model: OwnershipModel::Stackable,
                exists: holding.as_ref().is_some_and(|holding| holding.amount > 0),
                amount: holding.map(|holding| holding.amount),
                is_active: None,
                expires_at: None,
            })
        }
        OwnershipModel::Entitlement => {
            let holding = UserEntitlement::find_by_id((user_id, asset.id)).one(db).await?;
            Ok(InventoryPresenceView {
                asset_key: asset.key,
                ownership_model: OwnershipModel::Entitlement,
                exists: holding.is_some(),
                amount: None,
                is_active: None,
                expires_at: None,
            })
        }
        OwnershipModel::Expirable => {
            let holding = UserExpirableAsset::find_by_id((user_id, asset.id)).one(db).await?;
            let is_active = holding.as_ref().is_some_and(|holding| holding.expires_at > now);
            Ok(InventoryPresenceView {
                asset_key: asset.key,
                ownership_model: OwnershipModel::Expirable,
                exists: is_active,
                amount: None,
                is_active: Some(is_active),
                expires_at: holding.map(|holding| holding.expires_at),
            })
        }
    }
}

pub async fn grant_entitlement(
    db: &sea_orm::DatabaseConnection,
    mutation: EntitlementMutation,
) -> Result<EntitlementView> {
    mutation.actor.validate()?;
    let asset_key = validate_asset_key(&mutation.asset_key)?;
    let tx = db.begin().await?;
    ensure_user_exists(&tx, mutation.user_id).await?;
    let asset = get_non_currency_asset_by_key(&tx, &asset_key).await?;
    ensure_ownership_model(&asset, OwnershipModel::Entitlement)?;
    let now = chrono::Utc::now();

    let entitlement = if let Some(existing) = UserEntitlement::find_by_id((mutation.user_id, asset.id))
        .one(&tx)
        .await?
    {
        existing
    } else {
        let created = UserEntitlementActiveModel {
            user_id: Set(mutation.user_id),
            asset_definition_id: Set(asset.id),
            granted_at: Set(now),
            granted_by_actor: Set(serde_json::to_string(&mutation.actor)?),
            updated_at: Set(now),
        }
        .insert(&tx)
        .await?;
        write_inventory_operation(
            &tx,
            mutation.user_id,
            asset.id,
            OwnershipModel::Entitlement,
            "entitlement_granted",
            &mutation.actor,
            &mutation.context,
            None,
            None,
            None,
            None,
        )
        .await?;
        created
    };

    tx.commit().await?;
    Ok(EntitlementView {
        asset_key: asset.key,
        asset_definition_id: asset.id,
        granted_at: entitlement.granted_at,
        updated_at: entitlement.updated_at,
    })
}

pub async fn revoke_entitlement(
    db: &sea_orm::DatabaseConnection,
    mutation: EntitlementMutation,
) -> Result<()> {
    mutation.actor.validate()?;
    let asset_key = validate_asset_key(&mutation.asset_key)?;
    let tx = db.begin().await?;
    ensure_user_exists(&tx, mutation.user_id).await?;
    let asset = get_non_currency_asset_by_key(&tx, &asset_key).await?;
    ensure_ownership_model(&asset, OwnershipModel::Entitlement)?;

    if UserEntitlement::find_by_id((mutation.user_id, asset.id))
        .one(&tx)
        .await?
        .is_some()
    {
        UserEntitlement::delete_by_id((mutation.user_id, asset.id))
            .exec(&tx)
            .await?;
        write_inventory_operation(
            &tx,
            mutation.user_id,
            asset.id,
            OwnershipModel::Entitlement,
            "entitlement_revoked",
            &mutation.actor,
            &mutation.context,
            None,
            None,
            None,
            None,
        )
        .await?;
    }

    tx.commit().await?;
    Ok(())
}

pub async fn add_stackable(db: &sea_orm::DatabaseConnection, mutation: StackableMutation) -> Result<StackableView> {
    mutate_stackable(db, mutation, "stackable_added", |current, amount| Ok(current + amount)).await
}

pub async fn remove_stackable(
    db: &sea_orm::DatabaseConnection,
    mutation: StackableMutation,
) -> Result<StackableView> {
    mutate_stackable(db, mutation, "stackable_removed", |current, amount| {
        if current < amount {
            bail!("Insufficient stackable amount");
        }
        Ok(current - amount)
    })
    .await
}

pub async fn set_stackable(db: &sea_orm::DatabaseConnection, mutation: StackableMutation) -> Result<StackableView> {
    mutation.actor.validate()?;
    if mutation.amount < 0 {
        bail!("Amount must be non-negative");
    }
    let asset_key = validate_asset_key(&mutation.asset_key)?;
    let tx = db.begin().await?;
    ensure_user_exists(&tx, mutation.user_id).await?;
    let asset = get_non_currency_asset_by_key(&tx, &asset_key).await?;
    ensure_ownership_model(&asset, OwnershipModel::Stackable)?;
    let now = chrono::Utc::now();

    let existing = UserStackableAsset::find_by_id((mutation.user_id, asset.id)).one(&tx).await?;
    let model = write_stackable_state(&tx, mutation.user_id, asset.id, mutation.amount, now, existing).await?;

    write_inventory_operation(
        &tx,
        mutation.user_id,
        asset.id,
        OwnershipModel::Stackable,
        "stackable_set",
        &mutation.actor,
        &mutation.context,
        Some(mutation.amount),
        Some(mutation.amount),
        None,
        None,
    )
    .await?;

    tx.commit().await?;
    Ok(StackableView {
        asset_key: asset.key,
        asset_definition_id: asset.id,
        amount: model.amount,
        updated_at: model.updated_at,
    })
}

pub async fn prolong_expirable(
    db: &sea_orm::DatabaseConnection,
    mutation: ProlongExpirableMutation,
) -> Result<ExpirableView> {
    mutation.actor.validate()?;
    if mutation.duration_seconds <= 0 {
        bail!("Duration must be positive");
    }
    let asset_key = validate_asset_key(&mutation.asset_key)?;
    let tx = db.begin().await?;
    ensure_user_exists(&tx, mutation.user_id).await?;
    let asset = get_non_currency_asset_by_key(&tx, &asset_key).await?;
    ensure_ownership_model(&asset, OwnershipModel::Expirable)?;
    let now = chrono::Utc::now();
    let duration = chrono::Duration::seconds(mutation.duration_seconds);

    let existing = UserExpirableAsset::find_by_id((mutation.user_id, asset.id)).one(&tx).await?;
    let (previous_expires_at, model) = if let Some(existing) = existing {
        let base = if existing.expires_at > now { existing.expires_at } else { now };
        let mut active: UserExpirableAssetActiveModel = existing.clone().into();
        active.expires_at = Set(base + duration);
        active.last_extended_at = Set(Some(now));
        active.updated_at = Set(now);
        (Some(existing.expires_at), active.update(&tx).await?)
    } else {
        let expires_at = now + duration;
        let active = UserExpirableAssetActiveModel {
            user_id: Set(mutation.user_id),
            asset_definition_id: Set(asset.id),
            expires_at: Set(expires_at),
            granted_at: Set(now),
            last_extended_at: Set(Some(now)),
            granted_by_actor: Set(serde_json::to_string(&mutation.actor)?),
            updated_at: Set(now),
        };
        (None, active.insert(&tx).await?)
    };

    write_inventory_operation(
        &tx,
        mutation.user_id,
        asset.id,
        OwnershipModel::Expirable,
        "expirable_prolonged",
        &mutation.actor,
        &mutation.context,
        None,
        None,
        previous_expires_at,
        Some(model.expires_at),
        )
    .await?;

    tx.commit().await?;
    Ok(ExpirableView {
        asset_key: asset.key,
        asset_definition_id: asset.id,
        expires_at: model.expires_at,
        granted_at: model.granted_at,
        last_extended_at: model.last_extended_at,
        updated_at: model.updated_at,
        is_active: model.expires_at > now,
    })
}

pub async fn set_expiration(
    db: &sea_orm::DatabaseConnection,
    mutation: SetExpirationMutation,
) -> Result<ExpirableView> {
    mutation.actor.validate()?;
    let asset_key = validate_asset_key(&mutation.asset_key)?;
    let tx = db.begin().await?;
    ensure_user_exists(&tx, mutation.user_id).await?;
    let asset = get_non_currency_asset_by_key(&tx, &asset_key).await?;
    ensure_ownership_model(&asset, OwnershipModel::Expirable)?;
    let now = chrono::Utc::now();

    let existing = UserExpirableAsset::find_by_id((mutation.user_id, asset.id)).one(&tx).await?;
    let (previous_expires_at, model) = if let Some(existing) = existing {
        let previous = existing.expires_at;
        let mut active: UserExpirableAssetActiveModel = existing.into();
        active.expires_at = Set(mutation.expires_at);
        active.updated_at = Set(now);
        (Some(previous), active.update(&tx).await?)
    } else {
        let active = UserExpirableAssetActiveModel {
            user_id: Set(mutation.user_id),
            asset_definition_id: Set(asset.id),
            expires_at: Set(mutation.expires_at),
            granted_at: Set(now),
            last_extended_at: Set(None),
            granted_by_actor: Set(serde_json::to_string(&mutation.actor)?),
            updated_at: Set(now),
        };
        (None, active.insert(&tx).await?)
    };

    write_inventory_operation(
        &tx,
        mutation.user_id,
        asset.id,
        OwnershipModel::Expirable,
        "expirable_expiration_set",
        &mutation.actor,
        &mutation.context,
        None,
        None,
        previous_expires_at,
        Some(mutation.expires_at),
    )
    .await?;

    tx.commit().await?;
    Ok(ExpirableView {
        asset_key: asset.key,
        asset_definition_id: asset.id,
        expires_at: model.expires_at,
        granted_at: model.granted_at,
        last_extended_at: model.last_extended_at,
        updated_at: model.updated_at,
        is_active: model.expires_at > now,
    })
}

pub async fn revoke_expirable(
    db: &sea_orm::DatabaseConnection,
    mutation: EntitlementMutation,
) -> Result<()> {
    mutation.actor.validate()?;
    let asset_key = validate_asset_key(&mutation.asset_key)?;
    let tx = db.begin().await?;
    ensure_user_exists(&tx, mutation.user_id).await?;
    let asset = get_non_currency_asset_by_key(&tx, &asset_key).await?;
    ensure_ownership_model(&asset, OwnershipModel::Expirable)?;

    if let Some(existing) = UserExpirableAsset::find_by_id((mutation.user_id, asset.id)).one(&tx).await? {
        UserExpirableAsset::delete_by_id((mutation.user_id, asset.id))
            .exec(&tx)
            .await?;
        write_inventory_operation(
            &tx,
            mutation.user_id,
            asset.id,
            OwnershipModel::Expirable,
            "expirable_revoked",
            &mutation.actor,
            &mutation.context,
            None,
            None,
            Some(existing.expires_at),
            None,
        )
        .await?;
    }

    tx.commit().await?;
    Ok(())
}

pub async fn get_inventory_history(
    db: &sea_orm::DatabaseConnection,
    user_id: Uuid,
) -> Result<Vec<InventoryOperationView>> {
    ensure_user_exists(db, user_id).await?;
    let operations = InventoryOperation::find()
        .filter(InventoryOperationColumn::UserId.eq(user_id))
        .order_by_desc(InventoryOperationColumn::CreatedAt)
        .all(db)
        .await?;
    let assets = AssetDefinition::find().all(db).await?;

    operations
        .into_iter()
        .map(|operation| {
            let asset = assets
                .iter()
                .find(|asset| asset.id == operation.asset_definition_id)
                .ok_or_else(|| anyhow!("Missing asset definition for inventory operation"))?;
            Ok(InventoryOperationView {
                id: operation.id,
                asset_definition_id: operation.asset_definition_id,
                asset_key: asset.key.clone(),
                ownership_model: operation.ownership_model,
                operation_type: operation.operation_type,
                actor_kind: operation.actor_kind,
                actor_user_id: operation.actor_user_id,
                actor_service_name: operation.actor_service_name,
                delta_amount: operation.delta_amount,
                new_amount: operation.new_amount,
                previous_expires_at: operation.previous_expires_at,
                new_expires_at: operation.new_expires_at,
                reason_code: operation.reason_code,
                reason_text: operation.reason_text,
                metadata: serde_json::from_str(&operation.metadata)?,
                created_at: operation.created_at,
            })
        })
        .collect()
}

async fn mutate_stackable<F>(
    db: &sea_orm::DatabaseConnection,
    mutation: StackableMutation,
    operation_type: &str,
    compute_new_amount: F,
) -> Result<StackableView>
where
    F: Fn(i64, i64) -> Result<i64>,
{
    mutation.actor.validate()?;
    if mutation.amount <= 0 {
        bail!("Amount must be positive");
    }
    let asset_key = validate_asset_key(&mutation.asset_key)?;
    let tx = db.begin().await?;
    ensure_user_exists(&tx, mutation.user_id).await?;
    let asset = get_non_currency_asset_by_key(&tx, &asset_key).await?;
    ensure_ownership_model(&asset, OwnershipModel::Stackable)?;
    let now = chrono::Utc::now();

    let existing = UserStackableAsset::find_by_id((mutation.user_id, asset.id)).one(&tx).await?;
    let current_amount = existing.as_ref().map_or(0, |holding| holding.amount);
    let new_amount = compute_new_amount(current_amount, mutation.amount)?;
    if new_amount < 0 {
        bail!("Amount must not become negative");
    }
    let model = write_stackable_state(&tx, mutation.user_id, asset.id, new_amount, now, existing).await?;

    let delta = if operation_type == "stackable_removed" { -mutation.amount } else { mutation.amount };
    write_inventory_operation(
        &tx,
        mutation.user_id,
        asset.id,
        OwnershipModel::Stackable,
        operation_type,
        &mutation.actor,
        &mutation.context,
        Some(delta),
        Some(new_amount),
        None,
        None,
    )
    .await?;

    tx.commit().await?;
    Ok(StackableView {
        asset_key: asset.key,
        asset_definition_id: asset.id,
        amount: model.amount,
        updated_at: model.updated_at,
    })
}

async fn write_stackable_state(
    db: &impl ConnectionTrait,
    user_id: Uuid,
    asset_id: Uuid,
    amount: i64,
    now: chrono::DateTime<chrono::Utc>,
    existing: Option<UserStackableAssetModel>,
) -> Result<UserStackableAssetModel> {
    Ok(if amount == 0 {
        if existing.is_some() {
            UserStackableAsset::delete_by_id((user_id, asset_id)).exec(db).await?;
        }
        UserStackableAssetModel {
            user_id,
            asset_definition_id: asset_id,
            amount: 0,
            updated_at: now,
        }
    } else if let Some(existing) = existing {
        let mut active: UserStackableAssetActiveModel = existing.into();
        active.amount = Set(amount);
        active.updated_at = Set(now);
        active.update(db).await?
    } else {
        UserStackableAssetActiveModel {
            user_id: Set(user_id),
            asset_definition_id: Set(asset_id),
            amount: Set(amount),
            updated_at: Set(now),
        }
        .insert(db)
        .await?
    })
}

async fn write_inventory_operation(
    db: &impl ConnectionTrait,
    user_id: Uuid,
    asset_definition_id: Uuid,
    ownership_model: OwnershipModel,
    operation_type: &str,
    actor: &OwnershipActor,
    context: &OperationContext,
    delta_amount: Option<i64>,
    new_amount: Option<i64>,
    previous_expires_at: Option<chrono::DateTime<chrono::Utc>>,
    new_expires_at: Option<chrono::DateTime<chrono::Utc>>,
) -> Result<InventoryOperationModel> {
    Ok(InventoryOperationActiveModel {
        id: NotSet,
        user_id: Set(user_id),
        asset_definition_id: Set(asset_definition_id),
        ownership_model: Set(ownership_model.as_str().to_string()),
        operation_type: Set(operation_type.to_string()),
        actor_kind: Set(actor.kind.as_str().to_string()),
        actor_user_id: Set(actor.user_id),
        actor_service_name: Set(actor.service_name.clone()),
        delta_amount: Set(delta_amount),
        new_amount: Set(new_amount),
        previous_expires_at: Set(previous_expires_at),
        new_expires_at: Set(new_expires_at),
        reason_code: Set(context.reason_code.clone()),
        reason_text: Set(context.reason_text.clone()),
        metadata: Set(normalize_metadata(Some(context.metadata.clone())).to_string()),
        created_at: Set(chrono::Utc::now()),
    }
    .insert(db)
    .await?)
}

async fn ensure_user_exists(db: &impl ConnectionTrait, user_id: Uuid) -> Result<()> {
    if User::find_by_id(user_id).one(db).await?.is_none() {
        bail!("User not found");
    }
    Ok(())
}

async fn get_non_currency_asset_by_key(db: &impl ConnectionTrait, key: &str) -> Result<AssetDefinitionModel> {
    let asset = AssetDefinition::find()
        .filter(AssetDefinitionColumn::Key.eq(key))
        .one(db)
        .await?
        .ok_or_else(|| anyhow!("Asset not found"))?;
    if !asset.is_active {
        bail!("Asset is inactive");
    }
    if asset.is_currency {
        bail!("Currency assets belong to wallet");
    }
    Ok(asset)
}

fn ensure_ownership_model(asset: &AssetDefinitionModel, expected: OwnershipModel) -> Result<()> {
    let actual = parse_ownership_model(asset)?;
    if actual != expected {
        bail!("Ownership model mismatch");
    }
    Ok(())
}

fn parse_ownership_model(asset: &AssetDefinitionModel) -> Result<OwnershipModel> {
    OwnershipModel::parse(&asset.ownership_model)
}

async fn load_stackables(db: &sea_orm::DatabaseConnection, user_id: Uuid) -> Result<Vec<StackableView>> {
    let holdings = UserStackableAsset::find()
        .filter(UserStackableAssetColumn::UserId.eq(user_id))
        .order_by_desc(UserStackableAssetColumn::UpdatedAt)
        .all(db)
        .await?;
    let assets = AssetDefinition::find().all(db).await?;

    holdings
        .into_iter()
        .filter(|holding| holding.amount > 0)
        .map(|holding| {
            let asset = assets
                .iter()
                .find(|asset| asset.id == holding.asset_definition_id)
                .ok_or_else(|| anyhow!("Missing asset definition"))?;
            Ok(StackableView {
                asset_key: asset.key.clone(),
                asset_definition_id: asset.id,
                amount: holding.amount,
                updated_at: holding.updated_at,
            })
        })
        .collect()
}

async fn load_entitlements(db: &sea_orm::DatabaseConnection, user_id: Uuid) -> Result<Vec<EntitlementView>> {
    let holdings = UserEntitlement::find()
        .filter(UserEntitlementColumn::UserId.eq(user_id))
        .order_by_desc(UserEntitlementColumn::GrantedAt)
        .all(db)
        .await?;
    let assets = AssetDefinition::find().all(db).await?;

    holdings
        .into_iter()
        .map(|holding| {
            let asset = assets
                .iter()
                .find(|asset| asset.id == holding.asset_definition_id)
                .ok_or_else(|| anyhow!("Missing asset definition"))?;
            Ok(EntitlementView {
                asset_key: asset.key.clone(),
                asset_definition_id: asset.id,
                granted_at: holding.granted_at,
                updated_at: holding.updated_at,
            })
        })
        .collect()
}

async fn load_expirables(db: &sea_orm::DatabaseConnection, user_id: Uuid) -> Result<Vec<ExpirableView>> {
    let holdings = UserExpirableAsset::find()
        .filter(UserExpirableAssetColumn::UserId.eq(user_id))
        .order_by_desc(UserExpirableAssetColumn::ExpiresAt)
        .all(db)
        .await?;
    let assets = AssetDefinition::find().all(db).await?;
    let now = chrono::Utc::now();

    holdings
        .into_iter()
        .map(|holding| {
            let asset = assets
                .iter()
                .find(|asset| asset.id == holding.asset_definition_id)
                .ok_or_else(|| anyhow!("Missing asset definition"))?;
            Ok(ExpirableView {
                asset_key: asset.key.clone(),
                asset_definition_id: asset.id,
                expires_at: holding.expires_at,
                granted_at: holding.granted_at,
                last_extended_at: holding.last_extended_at,
                updated_at: holding.updated_at,
                is_active: holding.expires_at > now,
            })
        })
        .collect()
}
