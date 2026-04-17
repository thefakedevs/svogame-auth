use anyhow::{Result, anyhow, bail};
use rand::Rng;
use sea_orm::{
    ActiveModelTrait, ActiveValue::NotSet, ActiveValue::Set, ColumnTrait, ConnectionTrait,
    DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, TransactionTrait, TryIntoModel,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::entities::{
    AssetDefinition, AssetDefinitionColumn, AssetDefinitionModel, LootboxDefinition,
    AssetDefinitionActiveModel, LootboxDefinitionActiveModel, LootboxDefinitionColumn,
    LootboxDefinitionModel, LootboxDropDefinition, LootboxDropDefinitionActiveModel,
    LootboxDropDefinitionColumn, LootboxDropDefinitionModel, LootboxOpenOperation,
    LootboxOpenOperationActiveModel, LootboxOpenOperationColumn, LootboxOpenOperationModel, User,
    UserEntitlement, UserStackableAsset, UserStackableAssetColumn,
};
use crate::services::ownership::catalog::DEFAULT_COIN_ASSET_KEY;
use crate::services::ownership::inventory::{
    self, EntitlementMutation, ProlongExpirableMutation, StackableMutation, StackableView,
};
use crate::services::ownership::types::{
    AssetKind, OperationContext, OwnershipActor, OwnershipModel, normalize_metadata,
    validate_asset_key,
};
use crate::services::ownership::wallet::{self, WalletMutation};

const DEFAULT_FEED_LENGTH: usize = 100;
const MAX_FEED_LENGTH: usize = 200;

#[derive(Clone, Debug, Serialize)]
pub struct LootboxDefinitionView {
    pub id: Uuid,
    pub asset_definition_id: Uuid,
    pub asset_key: String,
    pub asset_display_name: String,
    pub asset_description: Option<String>,
    pub is_public: bool,
    pub is_active: bool,
    pub metadata: Value,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Clone, Debug, Serialize)]
pub struct LootboxDropView {
    pub id: Uuid,
    pub reward_asset_definition_id: Uuid,
    pub reward_asset_key: String,
    pub reward_asset_display_name: String,
    pub reward_ownership_model: OwnershipModel,
    pub stackable_amount: Option<i64>,
    pub expirable_duration_seconds: Option<i64>,
    pub duplicate_compensation_amount: Option<i64>,
    pub weight: i64,
    pub total_weight: i64,
    pub title_i18n: Value,
    pub is_active: bool,
    pub sort_order: i32,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Clone, Debug, Serialize)]
pub struct LootboxDetailView {
    pub definition: LootboxDefinitionView,
    pub drops: Vec<LootboxDropView>,
}

#[derive(Clone, Debug, Serialize)]
pub struct OwnedLootboxView {
    pub lootbox_id: Uuid,
    pub asset_key: String,
    pub display_name: String,
    pub amount: i64,
    pub is_openable: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OpenFeedEntryView {
    pub asset_key: String,
    pub display_name: String,
    pub title: String,
    pub ownership_model: String,
    pub amount: Option<i64>,
    pub duration_seconds: Option<i64>,
}

#[derive(Clone, Debug, Serialize)]
pub struct OpenRewardView {
    pub asset_definition_id: Uuid,
    pub asset_key: String,
    pub display_name: String,
    pub title: String,
    pub ownership_model: String,
    pub amount: Option<i64>,
    pub duration_seconds: Option<i64>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Clone, Debug, Serialize)]
pub struct LootboxOpenResultView {
    pub operation_id: i64,
    pub lootbox_asset_key: String,
    pub opened_at: chrono::DateTime<chrono::Utc>,
    pub selected_reward: OpenRewardView,
    pub reward: OpenRewardView,
    pub was_compensated: bool,
    pub feed: Vec<OpenFeedEntryView>,
    pub winner_index: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct LootboxOpenHistoryView {
    pub id: i64,
    pub user_id: Uuid,
    pub lootbox_asset_key: String,
    pub selected_reward: OpenRewardView,
    pub reward: OpenRewardView,
    pub was_compensated: bool,
    pub actor_kind: String,
    pub actor_user_id: Option<Uuid>,
    pub actor_service_name: Option<String>,
    pub feed_length: usize,
    pub winner_index: usize,
    pub opened_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateLootboxDefinitionInput {
    #[serde(alias = "assetKey")]
    pub asset_key: String,
    #[serde(alias = "isActive")]
    pub is_active: Option<bool>,
    pub metadata: Option<Value>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateLootboxDefinitionInput {
    #[serde(alias = "displayName")]
    pub display_name: Option<String>,
    pub description: Option<Option<String>>,
    #[serde(alias = "isPublic")]
    pub is_public: Option<bool>,
    #[serde(alias = "isActive")]
    pub is_active: Option<bool>,
    pub metadata: Option<Value>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateLootboxDropInput {
    #[serde(alias = "rewardAssetKey")]
    pub reward_asset_key: String,
    pub amount: Option<i64>,
    #[serde(alias = "durationSeconds")]
    pub duration_seconds: Option<i64>,
    #[serde(alias = "duplicateCompensationAmount")]
    pub duplicate_compensation_amount: Option<i64>,
    pub weight: i64,
    #[serde(alias = "titleI18n")]
    pub title_i18n: Option<Value>,
    #[serde(alias = "isActive")]
    pub is_active: Option<bool>,
    #[serde(alias = "sortOrder")]
    pub sort_order: Option<i32>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateLootboxDropInput {
    pub amount: Option<Option<i64>>,
    #[serde(alias = "durationSeconds")]
    pub duration_seconds: Option<Option<i64>>,
    #[serde(alias = "duplicateCompensationAmount")]
    pub duplicate_compensation_amount: Option<Option<i64>>,
    pub weight: Option<i64>,
    #[serde(alias = "titleI18n")]
    pub title_i18n: Option<Value>,
    #[serde(alias = "isActive")]
    pub is_active: Option<bool>,
    #[serde(alias = "sortOrder")]
    pub sort_order: Option<i32>,
}

pub async fn list_lootboxes(
    db: &DatabaseConnection,
    public_only: bool,
) -> Result<Vec<LootboxDefinitionView>> {
    let definitions = LootboxDefinition::find()
        .order_by_desc(LootboxDefinitionColumn::UpdatedAt)
        .all(db)
        .await?;
    let assets = AssetDefinition::find().all(db).await?;

    let items = definitions
        .into_iter()
        .filter_map(|definition| {
            let asset = assets
                .iter()
                .find(|asset| asset.id == definition.asset_definition_id)?;
            let asset_kind = AssetKind::parse(&asset.asset_kind).ok()?;
            let ownership_model = OwnershipModel::parse(&asset.ownership_model).ok()?;
            if asset_kind != AssetKind::Lootbox
                || ownership_model != OwnershipModel::Stackable
                || asset.is_currency
            {
                return None;
            }
            if public_only && (!definition.is_active || !asset.is_active || !asset.is_public) {
                return None;
            }
            Some(map_definition_view(definition, asset.clone()))
        })
        .collect();
    Ok(items)
}

pub async fn get_lootbox_by_id(
    db: &DatabaseConnection,
    lootbox_id: Uuid,
) -> Result<Option<LootboxDetailView>> {
    let definition = LootboxDefinition::find_by_id(lootbox_id).one(db).await?;
    match definition {
        Some(definition) => Ok(Some(load_lootbox_detail(db, definition).await?)),
        None => Ok(None),
    }
}

pub async fn get_public_lootbox_by_id(
    db: &DatabaseConnection,
    lootbox_id: Uuid,
) -> Result<Option<LootboxDetailView>> {
    let Some(definition) = LootboxDefinition::find_by_id(lootbox_id).one(db).await? else {
        return Ok(None);
    };
    let asset = AssetDefinition::find_by_id(definition.asset_definition_id)
        .one(db)
        .await?
        .ok_or_else(|| anyhow!("Asset not found"))?;
    if !definition.is_active || !asset.is_active || !asset.is_public {
        return Ok(None);
    }
    load_lootbox_detail_from_parts(db, definition, asset)
        .await
        .map(Some)
}

pub async fn create_lootbox_definition(
    db: &DatabaseConnection,
    input: CreateLootboxDefinitionInput,
) -> Result<LootboxDetailView> {
    let asset_key = validate_asset_key(&input.asset_key)?;
    let asset = get_valid_lootbox_asset_by_key(db, &asset_key).await?;
    if LootboxDefinition::find()
        .filter(LootboxDefinitionColumn::AssetDefinitionId.eq(asset.id))
        .one(db)
        .await?
        .is_some()
    {
        bail!("Lootbox definition already exists for this asset");
    }

    let now = chrono::Utc::now();
    let definition = LootboxDefinitionActiveModel {
        id: Set(Uuid::new_v4()),
        asset_definition_id: Set(asset.id),
        is_active: Set(input.is_active.unwrap_or(true)),
        metadata: Set(normalize_metadata(input.metadata).to_string()),
        created_at: Set(now),
        updated_at: Set(now),
    }
    .insert(db)
    .await?;

    load_lootbox_detail_from_parts(db, definition, asset).await
}

pub async fn update_lootbox_definition(
    db: &DatabaseConnection,
    lootbox_id: Uuid,
    input: UpdateLootboxDefinitionInput,
) -> Result<LootboxDetailView> {
    let definition = LootboxDefinition::find_by_id(lootbox_id)
        .one(db)
        .await?
        .ok_or_else(|| anyhow!("Lootbox not found"))?;
    let asset = AssetDefinition::find_by_id(definition.asset_definition_id)
        .one(db)
        .await?
        .ok_or_else(|| anyhow!("Asset not found"))?;

    let mut asset_active: AssetDefinitionActiveModel = asset.into();
    let mut should_update_asset = false;
    if let Some(display_name) = input.display_name {
        let normalized = display_name.trim();
        if normalized.is_empty() {
            bail!("Display name cannot be empty");
        }
        asset_active.display_name = Set(normalized.to_string());
        should_update_asset = true;
    }
    if let Some(description) = input.description {
        asset_active.description = Set(description
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty()));
        should_update_asset = true;
    }
    if let Some(is_public) = input.is_public {
        asset_active.is_public = Set(is_public);
        should_update_asset = true;
    }
    if should_update_asset {
        asset_active.updated_at = Set(chrono::Utc::now());
    }
    let asset = if should_update_asset {
        asset_active.update(db).await?
    } else {
        asset_active.try_into_model()?
    };

    let mut active: LootboxDefinitionActiveModel = definition.into();
    if let Some(is_active) = input.is_active {
        active.is_active = Set(is_active);
    }
    if let Some(metadata) = input.metadata {
        active.metadata = Set(normalize_metadata(Some(metadata)).to_string());
    }
    active.updated_at = Set(chrono::Utc::now());
    let updated = active.update(db).await?;

    load_lootbox_detail_from_parts(db, updated, asset).await
}

pub async fn create_lootbox_drop(
    db: &DatabaseConnection,
    lootbox_id: Uuid,
    input: CreateLootboxDropInput,
) -> Result<LootboxDetailView> {
    let definition = LootboxDefinition::find_by_id(lootbox_id)
        .one(db)
        .await?
        .ok_or_else(|| anyhow!("Lootbox not found"))?;
    let asset = AssetDefinition::find_by_id(definition.asset_definition_id)
        .one(db)
        .await?
        .ok_or_else(|| anyhow!("Asset not found"))?;
    let reward_asset_key = validate_asset_key(&input.reward_asset_key)?;
    let reward_asset = get_valid_lootbox_reward_asset_by_key(db, &reward_asset_key).await?;
    validate_drop_payload(
        &reward_asset,
        OwnershipModel::parse(&reward_asset.ownership_model)?,
        input.amount,
        input.duration_seconds,
        input.duplicate_compensation_amount,
        input.weight,
    )?;

    LootboxDropDefinitionActiveModel {
        id: Set(Uuid::new_v4()),
        lootbox_definition_id: Set(lootbox_id),
        reward_asset_definition_id: Set(reward_asset.id),
        stackable_amount: Set(input.amount),
        expirable_duration_seconds: Set(input.duration_seconds),
        duplicate_compensation_amount: Set(input.duplicate_compensation_amount),
        weight: Set(input.weight),
        title_i18n: Set(normalize_title_i18n(input.title_i18n)?.to_string()),
        is_active: Set(input.is_active.unwrap_or(true)),
        sort_order: Set(input.sort_order.unwrap_or(0)),
        created_at: Set(chrono::Utc::now()),
        updated_at: Set(chrono::Utc::now()),
    }
    .insert(db)
    .await?;

    load_lootbox_detail_from_parts(db, definition, asset).await
}

pub async fn update_lootbox_drop(
    db: &DatabaseConnection,
    lootbox_id: Uuid,
    drop_id: Uuid,
    input: UpdateLootboxDropInput,
) -> Result<LootboxDetailView> {
    let definition = LootboxDefinition::find_by_id(lootbox_id)
        .one(db)
        .await?
        .ok_or_else(|| anyhow!("Lootbox not found"))?;
    let asset = AssetDefinition::find_by_id(definition.asset_definition_id)
        .one(db)
        .await?
        .ok_or_else(|| anyhow!("Asset not found"))?;
    let drop = LootboxDropDefinition::find_by_id(drop_id)
        .one(db)
        .await?
        .ok_or_else(|| anyhow!("Lootbox drop not found"))?;
    if drop.lootbox_definition_id != lootbox_id {
        bail!("Lootbox drop does not belong to the requested lootbox");
    }

    let reward_asset = AssetDefinition::find_by_id(drop.reward_asset_definition_id)
        .one(db)
        .await?
        .ok_or_else(|| anyhow!("Reward asset not found"))?;
    let reward_ownership_model = OwnershipModel::parse(&reward_asset.ownership_model)?;
    let next_amount = input.amount.unwrap_or(drop.stackable_amount);
    let next_duration = input
        .duration_seconds
        .unwrap_or(drop.expirable_duration_seconds);
    let next_duplicate_compensation_amount = input
        .duplicate_compensation_amount
        .unwrap_or(drop.duplicate_compensation_amount);
    let next_weight = input.weight.unwrap_or(drop.weight);
    validate_drop_payload(
        &reward_asset,
        reward_ownership_model,
        next_amount,
        next_duration,
        next_duplicate_compensation_amount,
        next_weight,
    )?;

    let mut active: LootboxDropDefinitionActiveModel = drop.into();
    if let Some(amount) = input.amount {
        active.stackable_amount = Set(amount);
    }
    if let Some(duration_seconds) = input.duration_seconds {
        active.expirable_duration_seconds = Set(duration_seconds);
    }
    if let Some(duplicate_compensation_amount) = input.duplicate_compensation_amount {
        active.duplicate_compensation_amount = Set(duplicate_compensation_amount);
    }
    if let Some(weight) = input.weight {
        active.weight = Set(weight);
    }
    if let Some(title_i18n) = input.title_i18n {
        active.title_i18n = Set(normalize_title_i18n(Some(title_i18n))?.to_string());
    }
    if let Some(is_active) = input.is_active {
        active.is_active = Set(is_active);
    }
    if let Some(sort_order) = input.sort_order {
        active.sort_order = Set(sort_order);
    }
    active.updated_at = Set(chrono::Utc::now());
    active.update(db).await?;

    load_lootbox_detail_from_parts(db, definition, asset).await
}

pub async fn delete_lootbox_drop(
    db: &DatabaseConnection,
    lootbox_id: Uuid,
    drop_id: Uuid,
) -> Result<()> {
    let drop = LootboxDropDefinition::find_by_id(drop_id)
        .one(db)
        .await?
        .ok_or_else(|| anyhow!("Lootbox drop not found"))?;
    if drop.lootbox_definition_id != lootbox_id {
        bail!("Lootbox drop does not belong to the requested lootbox");
    }
    LootboxDropDefinition::delete_by_id(drop_id)
        .exec(db)
        .await?;
    Ok(())
}

pub async fn list_owned_lootboxes(
    db: &DatabaseConnection,
    user_id: Uuid,
) -> Result<Vec<OwnedLootboxView>> {
    ensure_user_exists(db, user_id).await?;
    let holdings = UserStackableAsset::find()
        .filter(UserStackableAssetColumn::UserId.eq(user_id))
        .filter(UserStackableAssetColumn::Amount.gt(0))
        .all(db)
        .await?;
    let assets = AssetDefinition::find().all(db).await?;
    let definitions = LootboxDefinition::find().all(db).await?;

    let mut result = Vec::new();
    for holding in holdings {
        let Some(asset) = assets
            .iter()
            .find(|asset| asset.id == holding.asset_definition_id)
        else {
            continue;
        };
        if AssetKind::parse(&asset.asset_kind)? != AssetKind::Lootbox {
            continue;
        }
        let Some(definition) = definitions
            .iter()
            .find(|definition| definition.asset_definition_id == asset.id)
        else {
            continue;
        };
        result.push(OwnedLootboxView {
            lootbox_id: definition.id,
            asset_key: asset.key.clone(),
            display_name: asset.display_name.clone(),
            amount: holding.amount,
            is_openable: definition.is_active && asset.is_active,
        });
    }
    result.sort_by(|a, b| a.asset_key.cmp(&b.asset_key));
    Ok(result)
}

pub async fn get_open_history(
    db: &DatabaseConnection,
    user_id: Uuid,
) -> Result<Vec<LootboxOpenHistoryView>> {
    ensure_user_exists(db, user_id).await?;
    let operations = LootboxOpenOperation::find()
        .filter(LootboxOpenOperationColumn::UserId.eq(user_id))
        .order_by_desc(LootboxOpenOperationColumn::CreatedAt)
        .all(db)
        .await?;
    map_open_history(db, operations).await
}

pub async fn get_all_open_history(db: &DatabaseConnection) -> Result<Vec<LootboxOpenHistoryView>> {
    let operations = LootboxOpenOperation::find()
        .order_by_desc(LootboxOpenOperationColumn::CreatedAt)
        .all(db)
        .await?;
    map_open_history(db, operations).await
}

pub async fn open_lootbox(
    db: &DatabaseConnection,
    user_id: Uuid,
    asset_key: &str,
    feed_length: Option<usize>,
    locale: Option<String>,
    actor: OwnershipActor,
) -> Result<LootboxOpenResultView> {
    actor.validate()?;
    let asset_key = validate_asset_key(asset_key)?;
    let feed_length = validate_feed_length(feed_length)?;

    let tx = db.begin().await?;
    ensure_user_exists(&tx, user_id).await?;
    let (definition, lootbox_asset) = get_openable_lootbox_by_asset_key(&tx, &asset_key).await?;
    let drops = load_active_drop_records(&tx, definition.id).await?;
    if drops.is_empty() {
        bail!("Lootbox has no active drops");
    }

    let weighted_drops = resolve_weighted_drops(&tx, &drops).await?;
    let winner = choose_drop(&weighted_drops)?;
    let locale = locale.filter(|locale| !locale.trim().is_empty());
    let reward_title = localize_title(
        &winner.title_i18n,
        locale.as_deref(),
        &winner.reward_asset.display_name,
    );

    let open_context = OperationContext {
        reason_code: Some("lootbox_open".to_string()),
        reason_text: Some(format!("Opened lootbox '{}'", lootbox_asset.key)),
        metadata: json!({ "lootboxAssetKey": lootbox_asset.key }),
    };
    inventory::remove_stackable_in_tx(
        &tx,
        StackableMutation {
            user_id,
            asset_key: lootbox_asset.key.clone(),
            amount: 1,
            actor: actor.clone(),
            context: open_context,
        },
    )
    .await?;

    let reward_application = apply_reward(
        &tx,
        user_id,
        &winner,
        reward_title.clone(),
        actor.clone(),
        &lootbox_asset.key,
    )
    .await?;

    let winner_index = choose_winner_index(feed_length);
    let mut feed = generate_feed(&weighted_drops, feed_length, locale.as_deref())?;
    feed[winner_index] = OpenFeedEntryView {
        asset_key: reward_application.selected.asset_key.clone(),
        display_name: reward_application.selected.display_name.clone(),
        title: reward_application.selected.title.clone(),
        ownership_model: reward_application.selected.ownership_model.clone(),
        amount: reward_application.selected.amount,
        duration_seconds: reward_application.selected.duration_seconds,
    };

    let operation = LootboxOpenOperationActiveModel {
        id: NotSet,
        user_id: Set(user_id),
        lootbox_definition_id: Set(definition.id),
        lootbox_asset_definition_id: Set(lootbox_asset.id),
        selected_drop_definition_id: Set(winner.drop.id),
        reward_asset_definition_id: Set(winner.reward_asset.id),
        reward_ownership_model: Set(reward_application.selected.ownership_model.clone()),
        reward_amount: Set(reward_application.selected.amount),
        reward_duration_seconds: Set(reward_application.selected.duration_seconds),
        reward_expires_at: Set(reward_application.selected.expires_at),
        reward_title: Set(reward_application.selected.title.clone()),
        granted_asset_definition_id: Set(reward_application.granted.asset_definition_id),
        granted_ownership_model: Set(reward_application.granted.ownership_model.clone()),
        granted_amount: Set(reward_application.granted.amount),
        granted_duration_seconds: Set(reward_application.granted.duration_seconds),
        granted_expires_at: Set(reward_application.granted.expires_at),
        granted_title: Set(reward_application.granted.title.clone()),
        was_compensated: Set(reward_application.was_compensated),
        locale: Set(locale.clone()),
        actor_kind: Set(actor.kind.as_str().to_string()),
        actor_user_id: Set(actor.user_id),
        actor_service_name: Set(actor.service_name.clone()),
        feed_length: Set(feed_length as i32),
        winner_index: Set(winner_index as i32),
        feed_json: Set(serde_json::to_string(&feed)?),
        created_at: Set(chrono::Utc::now()),
    }
    .insert(&tx)
    .await?;

    tx.commit().await?;

    Ok(LootboxOpenResultView {
        operation_id: operation.id,
        lootbox_asset_key: lootbox_asset.key,
        opened_at: operation.created_at,
        selected_reward: reward_application.selected,
        reward: reward_application.granted,
        was_compensated: reward_application.was_compensated,
        feed,
        winner_index,
    })
}

async fn map_open_history(
    db: &DatabaseConnection,
    operations: Vec<LootboxOpenOperationModel>,
) -> Result<Vec<LootboxOpenHistoryView>> {
    let assets = AssetDefinition::find().all(db).await?;
    let mut result = Vec::new();
    for item in operations {
        let lootbox_asset = assets
            .iter()
            .find(|asset| asset.id == item.lootbox_asset_definition_id)
            .ok_or_else(|| anyhow!("Lootbox asset not found"))?;
        let reward_asset = assets
            .iter()
            .find(|asset| asset.id == item.reward_asset_definition_id)
            .ok_or_else(|| anyhow!("Reward asset not found"))?;
        let granted_asset_id = if item.granted_asset_definition_id.is_nil() {
            item.reward_asset_definition_id
        } else {
            item.granted_asset_definition_id
        };
        let granted_asset = assets
            .iter()
            .find(|asset| asset.id == granted_asset_id)
            .unwrap_or(reward_asset);
        let selected_reward = OpenRewardView {
            asset_definition_id: reward_asset.id,
            asset_key: reward_asset.key.clone(),
            display_name: reward_asset.display_name.clone(),
            title: item.reward_title,
            ownership_model: item.reward_ownership_model,
            amount: item.reward_amount,
            duration_seconds: item.reward_duration_seconds,
            expires_at: item.reward_expires_at,
        };
        let granted_reward = OpenRewardView {
            asset_definition_id: granted_asset.id,
            asset_key: granted_asset.key.clone(),
            display_name: granted_asset.display_name.clone(),
            title: if item.granted_title.trim().is_empty() {
                selected_reward.title.clone()
            } else {
                item.granted_title
            },
            ownership_model: if item.granted_ownership_model.trim().is_empty() {
                selected_reward.ownership_model.clone()
            } else {
                item.granted_ownership_model
            },
            amount: item.granted_amount.or(selected_reward.amount),
            duration_seconds: item
                .granted_duration_seconds
                .or(selected_reward.duration_seconds),
            expires_at: item.granted_expires_at.or(selected_reward.expires_at),
        };
        result.push(LootboxOpenHistoryView {
            id: item.id,
            user_id: item.user_id,
            lootbox_asset_key: lootbox_asset.key.clone(),
            selected_reward,
            reward: granted_reward,
            was_compensated: item.was_compensated,
            actor_kind: item.actor_kind,
            actor_user_id: item.actor_user_id,
            actor_service_name: item.actor_service_name,
            feed_length: item.feed_length as usize,
            winner_index: item.winner_index as usize,
            opened_at: item.created_at,
        });
    }
    Ok(result)
}

async fn apply_reward(
    db: &impl ConnectionTrait,
    user_id: Uuid,
    winner: &ResolvedDrop,
    reward_title: String,
    actor: OwnershipActor,
    lootbox_asset_key: &str,
) -> Result<RewardApplication> {
    let selected_reward = OpenRewardView {
        asset_definition_id: winner.reward_asset.id,
        asset_key: winner.reward_asset.key.clone(),
        display_name: winner.reward_asset.display_name.clone(),
        title: reward_title.clone(),
        ownership_model: winner.reward_ownership_model.as_str().to_string(),
        amount: winner.drop.stackable_amount,
        duration_seconds: winner.drop.expirable_duration_seconds,
        expires_at: None,
    };
    let reward_context = OperationContext {
        reason_code: Some("lootbox_reward".to_string()),
        reason_text: Some(format!("Reward from lootbox '{}'", lootbox_asset_key)),
        metadata: json!({
            "lootboxAssetKey": lootbox_asset_key,
            "rewardAssetKey": winner.reward_asset.key,
        }),
    };

    if winner.reward_asset.is_currency {
        let amount = winner
            .drop
            .stackable_amount
            .ok_or_else(|| anyhow!("Missing currency amount"))?;
        wallet::credit_in_tx(
            db,
            WalletMutation {
                user_id,
                currency_key: winner.reward_asset.key.clone(),
                amount,
                actor,
                context: reward_context,
            },
        )
        .await?;

        return Ok(RewardApplication {
            granted: OpenRewardView {
                amount: Some(amount),
                ..selected_reward.clone()
            },
            selected: OpenRewardView {
                amount: Some(amount),
                ..selected_reward
            },
            was_compensated: false,
        });
    }

    match winner.reward_ownership_model {
        OwnershipModel::Stackable => {
            let amount = winner
                .drop
                .stackable_amount
                .ok_or_else(|| anyhow!("Missing stackable amount"))?;
            let StackableView { .. } = inventory::add_stackable_in_tx(
                db,
                StackableMutation {
                    user_id,
                    asset_key: winner.reward_asset.key.clone(),
                    amount,
                    actor,
                    context: reward_context,
                },
            )
            .await?;

            Ok(RewardApplication {
                selected: selected_reward.clone(),
                granted: OpenRewardView {
                    amount: Some(amount),
                    duration_seconds: None,
                    expires_at: None,
                    ..selected_reward
                },
                was_compensated: false,
            })
        }
        OwnershipModel::Expirable => {
            let duration_seconds = winner
                .drop
                .expirable_duration_seconds
                .ok_or_else(|| anyhow!("Missing expirable duration"))?;
            let expirable = inventory::prolong_expirable_in_tx(
                db,
                ProlongExpirableMutation {
                    user_id,
                    asset_key: winner.reward_asset.key.clone(),
                    duration_seconds,
                    actor,
                    context: reward_context,
                },
            )
            .await?;

            Ok(RewardApplication {
                selected: selected_reward.clone(),
                granted: OpenRewardView {
                    amount: None,
                    duration_seconds: Some(duration_seconds),
                    expires_at: Some(expirable.expires_at),
                    ..selected_reward
                },
                was_compensated: false,
            })
        }
        OwnershipModel::Entitlement => {
            let already_owned = UserEntitlement::find_by_id((user_id, winner.reward_asset.id))
                .one(db)
                .await?
                .is_some();
            if already_owned {
                let compensation_amount = winner
                    .drop
                    .duplicate_compensation_amount
                    .ok_or_else(|| anyhow!("Missing duplicate compensation amount"))?;
                let compensation_asset = AssetDefinition::find()
                    .filter(AssetDefinitionColumn::Key.eq(DEFAULT_COIN_ASSET_KEY))
                    .one(db)
                    .await?
                    .ok_or_else(|| anyhow!("Default currency asset not found"))?;
                wallet::credit_in_tx(
                    db,
                    WalletMutation {
                        user_id,
                        currency_key: DEFAULT_COIN_ASSET_KEY.to_string(),
                        amount: compensation_amount,
                        actor,
                        context: OperationContext {
                            reason_code: Some("lootbox_duplicate_compensation".to_string()),
                            reason_text: Some(format!(
                                "Duplicate reward '{}' from lootbox '{}'",
                                winner.reward_asset.key, lootbox_asset_key
                            )),
                            metadata: json!({
                                "lootboxAssetKey": lootbox_asset_key,
                                "selectedRewardAssetKey": winner.reward_asset.key,
                                "compensationCurrencyKey": DEFAULT_COIN_ASSET_KEY,
                            }),
                        },
                    },
                )
                .await?;
                return Ok(RewardApplication {
                    selected: selected_reward,
                    granted: OpenRewardView {
                        asset_definition_id: compensation_asset.id,
                        asset_key: compensation_asset.key,
                        display_name: compensation_asset.display_name,
                        title: "Duplicate compensation".to_string(),
                        ownership_model: OwnershipModel::Stackable.as_str().to_string(),
                        amount: Some(compensation_amount),
                        duration_seconds: None,
                        expires_at: None,
                    },
                    was_compensated: true,
                });
            }

            inventory::grant_entitlement_in_tx(
                db,
                EntitlementMutation {
                    user_id,
                    asset_key: winner.reward_asset.key.clone(),
                    actor,
                    context: reward_context,
                },
            )
            .await?;

            Ok(RewardApplication {
                selected: selected_reward.clone(),
                granted: selected_reward,
                was_compensated: false,
            })
        }
    }
}

fn generate_feed(
    weighted_drops: &[ResolvedDrop],
    feed_length: usize,
    locale: Option<&str>,
) -> Result<Vec<OpenFeedEntryView>> {
    let mut items = Vec::with_capacity(feed_length);
    for _ in 0..feed_length {
        let entry = choose_drop(weighted_drops)?;
        let title = localize_title(&entry.title_i18n, locale, &entry.reward_asset.display_name);
        items.push(OpenFeedEntryView {
            asset_key: entry.reward_asset.key.clone(),
            display_name: entry.reward_asset.display_name.clone(),
            title,
            ownership_model: entry.reward_ownership_model.as_str().to_string(),
            amount: entry.drop.stackable_amount,
            duration_seconds: entry.drop.expirable_duration_seconds,
        });
    }
    Ok(items)
}

fn choose_winner_index(feed_length: usize) -> usize {
    if feed_length <= 1 {
        return 0;
    }
    let min = ((feed_length as f64) * 0.65).floor() as usize;
    let max = feed_length - 1;
    rand::rng().random_range(min.min(max)..=max)
}

fn choose_drop(weighted_drops: &[ResolvedDrop]) -> Result<ResolvedDrop> {
    if weighted_drops.is_empty() {
        bail!("Lootbox has no active drops");
    }
    let total_weight: i64 = weighted_drops.iter().map(|drop| drop.drop.weight).sum();
    if total_weight <= 0 {
        bail!("Lootbox total weight must be positive");
    }

    let mut roll = rand::rng().random_range(0..total_weight);
    for drop in weighted_drops {
        if roll < drop.drop.weight {
            return Ok(drop.clone());
        }
        roll -= drop.drop.weight;
    }
    weighted_drops
        .last()
        .cloned()
        .ok_or_else(|| anyhow!("Lootbox has no active drops"))
}

async fn load_lootbox_detail(
    db: &DatabaseConnection,
    definition: LootboxDefinitionModel,
) -> Result<LootboxDetailView> {
    let asset = AssetDefinition::find_by_id(definition.asset_definition_id)
        .one(db)
        .await?
        .ok_or_else(|| anyhow!("Asset not found"))?;
    load_lootbox_detail_from_parts(db, definition, asset).await
}

async fn load_lootbox_detail_from_parts(
    db: &DatabaseConnection,
    definition: LootboxDefinitionModel,
    asset: AssetDefinitionModel,
) -> Result<LootboxDetailView> {
    let drops = LootboxDropDefinition::find()
        .filter(LootboxDropDefinitionColumn::LootboxDefinitionId.eq(definition.id))
        .order_by_asc(LootboxDropDefinitionColumn::SortOrder)
        .order_by_asc(LootboxDropDefinitionColumn::CreatedAt)
        .all(db)
        .await?;
    let reward_assets = AssetDefinition::find().all(db).await?;
    let total_weight = drops
        .iter()
        .filter(|drop| drop.is_active)
        .map(|drop| drop.weight)
        .sum::<i64>();
    let mapped_drops = drops
        .into_iter()
        .map(|drop| {
            let reward_asset = reward_assets
                .iter()
                .find(|asset| asset.id == drop.reward_asset_definition_id)
                .ok_or_else(|| anyhow!("Reward asset not found"))?;
            Ok(LootboxDropView {
                id: drop.id,
                reward_asset_definition_id: reward_asset.id,
                reward_asset_key: reward_asset.key.clone(),
                reward_asset_display_name: reward_asset.display_name.clone(),
                reward_ownership_model: OwnershipModel::parse(&reward_asset.ownership_model)?,
                stackable_amount: drop.stackable_amount,
                expirable_duration_seconds: drop.expirable_duration_seconds,
                duplicate_compensation_amount: drop.duplicate_compensation_amount,
                weight: drop.weight,
                total_weight,
                title_i18n: serde_json::from_str(&drop.title_i18n)?,
                is_active: drop.is_active,
                sort_order: drop.sort_order,
                created_at: drop.created_at,
                updated_at: drop.updated_at,
            })
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(LootboxDetailView {
        definition: map_definition_view(definition, asset),
        drops: mapped_drops,
    })
}

fn map_definition_view(
    definition: LootboxDefinitionModel,
    asset: AssetDefinitionModel,
) -> LootboxDefinitionView {
    LootboxDefinitionView {
        id: definition.id,
        asset_definition_id: asset.id,
        asset_key: asset.key,
        asset_display_name: asset.display_name,
        asset_description: asset.description,
        is_public: asset.is_public,
        is_active: definition.is_active && asset.is_active,
        metadata: serde_json::from_str(&definition.metadata).unwrap_or_else(|_| json!({})),
        created_at: definition.created_at,
        updated_at: definition.updated_at,
    }
}

async fn resolve_weighted_drops(
    db: &impl ConnectionTrait,
    drops: &[LootboxDropDefinitionModel],
) -> Result<Vec<ResolvedDrop>> {
    let assets = AssetDefinition::find().all(db).await?;
    let mut result = Vec::new();
    for drop in drops.iter().filter(|drop| drop.is_active) {
        let reward_asset = assets
            .iter()
            .find(|asset| asset.id == drop.reward_asset_definition_id)
            .ok_or_else(|| anyhow!("Reward asset not found"))?
            .clone();
        let reward_ownership_model = OwnershipModel::parse(&reward_asset.ownership_model)?;
        result.push(ResolvedDrop {
            drop: drop.clone(),
            reward_asset,
            reward_ownership_model,
            title_i18n: serde_json::from_str(&drop.title_i18n)?,
        });
    }
    Ok(result)
}

async fn load_active_drop_records(
    db: &impl ConnectionTrait,
    lootbox_definition_id: Uuid,
) -> Result<Vec<LootboxDropDefinitionModel>> {
    LootboxDropDefinition::find()
        .filter(LootboxDropDefinitionColumn::LootboxDefinitionId.eq(lootbox_definition_id))
        .filter(LootboxDropDefinitionColumn::IsActive.eq(true))
        .order_by_asc(LootboxDropDefinitionColumn::SortOrder)
        .order_by_asc(LootboxDropDefinitionColumn::CreatedAt)
        .all(db)
        .await
        .map_err(Into::into)
}

async fn get_openable_lootbox_by_asset_key(
    db: &impl ConnectionTrait,
    asset_key: &str,
) -> Result<(LootboxDefinitionModel, AssetDefinitionModel)> {
    let (definition, asset) = get_lootbox_and_asset_by_key(db, asset_key)
        .await?
        .ok_or_else(|| anyhow!("Lootbox not found"))?;
    if !definition.is_active || !asset.is_active {
        bail!("Lootbox is inactive");
    }
    Ok((definition, asset))
}

async fn get_lootbox_and_asset_by_key(
    db: &impl ConnectionTrait,
    asset_key: &str,
) -> Result<Option<(LootboxDefinitionModel, AssetDefinitionModel)>> {
    let asset = AssetDefinition::find()
        .filter(AssetDefinitionColumn::Key.eq(asset_key))
        .one(db)
        .await?;
    let Some(asset) = asset else {
        return Ok(None);
    };
    validate_lootbox_asset(&asset)?;
    let definition = LootboxDefinition::find()
        .filter(LootboxDefinitionColumn::AssetDefinitionId.eq(asset.id))
        .one(db)
        .await?;
    Ok(definition.map(|definition| (definition, asset)))
}

async fn get_valid_lootbox_asset_by_key(
    db: &impl ConnectionTrait,
    asset_key: &str,
) -> Result<AssetDefinitionModel> {
    let asset = AssetDefinition::find()
        .filter(AssetDefinitionColumn::Key.eq(asset_key))
        .one(db)
        .await?
        .ok_or_else(|| anyhow!("Asset not found"))?;
    validate_lootbox_asset(&asset)?;
    Ok(asset)
}

async fn get_valid_lootbox_reward_asset_by_key(
    db: &impl ConnectionTrait,
    asset_key: &str,
) -> Result<AssetDefinitionModel> {
    let asset = AssetDefinition::find()
        .filter(AssetDefinitionColumn::Key.eq(asset_key))
        .one(db)
        .await?
        .ok_or_else(|| anyhow!("Asset not found"))?;
    validate_reward_asset(&asset)?;
    Ok(asset)
}

fn validate_lootbox_asset(asset: &AssetDefinitionModel) -> Result<()> {
    if !asset.is_active {
        bail!("Asset is inactive");
    }
    if AssetKind::parse(&asset.asset_kind)? != AssetKind::Lootbox {
        bail!("Asset must be of lootbox kind");
    }
    if OwnershipModel::parse(&asset.ownership_model)? != OwnershipModel::Stackable {
        bail!("Lootbox asset must use stackable ownership");
    }
    if asset.is_currency {
        bail!("Lootbox asset cannot be currency");
    }
    Ok(())
}

fn validate_reward_asset(asset: &AssetDefinitionModel) -> Result<()> {
    if !asset.is_active {
        bail!("Reward asset is inactive");
    }
    match OwnershipModel::parse(&asset.ownership_model)? {
        OwnershipModel::Stackable | OwnershipModel::Expirable | OwnershipModel::Entitlement => {
            Ok(())
        }
    }
}

fn validate_drop_payload(
    reward_asset: &AssetDefinitionModel,
    reward_ownership_model: OwnershipModel,
    amount: Option<i64>,
    duration_seconds: Option<i64>,
    duplicate_compensation_amount: Option<i64>,
    weight: i64,
) -> Result<()> {
    if weight <= 0 {
        bail!("Drop weight must be positive");
    }
    match reward_ownership_model {
        OwnershipModel::Stackable => {
            let amount =
                amount.ok_or_else(|| anyhow!("amount is required for stackable rewards"))?;
            if amount <= 0 {
                bail!("amount must be positive");
            }
            if duration_seconds.is_some() {
                bail!("durationSeconds is not allowed for stackable rewards");
            }
            if duplicate_compensation_amount.is_some() {
                bail!("duplicateCompensationAmount is allowed only for entitlement rewards");
            }
            if reward_asset.is_currency && reward_asset.asset_kind != AssetKind::Currency.as_str() {
                bail!("Currency reward asset must use asset_kind = currency");
            }
        }
        OwnershipModel::Expirable => {
            let duration_seconds = duration_seconds
                .ok_or_else(|| anyhow!("durationSeconds is required for expirable rewards"))?;
            if duration_seconds <= 0 {
                bail!("durationSeconds must be positive");
            }
            if amount.is_some() {
                bail!("amount is not allowed for expirable rewards");
            }
            if duplicate_compensation_amount.is_some() {
                bail!("duplicateCompensationAmount is allowed only for entitlement rewards");
            }
        }
        OwnershipModel::Entitlement => {
            if amount.is_some() || duration_seconds.is_some() {
                bail!("amount and durationSeconds are not allowed for entitlement rewards");
            }
            let compensation = duplicate_compensation_amount.ok_or_else(|| {
                anyhow!("duplicateCompensationAmount is required for entitlement rewards")
            })?;
            if compensation <= 0 {
                bail!("duplicateCompensationAmount must be positive");
            }
        }
    }
    Ok(())
}

fn normalize_title_i18n(value: Option<Value>) -> Result<Value> {
    let value = value.unwrap_or_else(|| json!({}));
    match value {
        Value::Object(_) => Ok(value),
        _ => bail!("titleI18n must be a JSON object"),
    }
}

fn localize_title(title_i18n: &Value, locale: Option<&str>, fallback: &str) -> String {
    let Some(map) = title_i18n.as_object() else {
        return fallback.to_string();
    };
    if let Some(locale) = locale {
        if let Some(title) = map.get(locale).and_then(Value::as_str) {
            return title.to_string();
        }
        let primary = locale.split('-').next().unwrap_or(locale);
        if let Some(title) = map.get(primary).and_then(Value::as_str) {
            return title.to_string();
        }
    }
    if let Some(title) = map.get("en").and_then(Value::as_str) {
        return title.to_string();
    }
    map.values()
        .find_map(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| fallback.to_string())
}

fn validate_feed_length(feed_length: Option<usize>) -> Result<usize> {
    let feed_length = feed_length.unwrap_or(DEFAULT_FEED_LENGTH);
    if !(1..=MAX_FEED_LENGTH).contains(&feed_length) {
        bail!("feedLength must be between 1 and {}", MAX_FEED_LENGTH);
    }
    Ok(feed_length)
}

async fn ensure_user_exists(db: &impl ConnectionTrait, user_id: Uuid) -> Result<()> {
    if User::find_by_id(user_id).one(db).await?.is_none() {
        bail!("User not found");
    }
    Ok(())
}

#[derive(Clone)]
struct ResolvedDrop {
    drop: LootboxDropDefinitionModel,
    reward_asset: AssetDefinitionModel,
    reward_ownership_model: OwnershipModel,
    title_i18n: Value,
}

struct RewardApplication {
    selected: OpenRewardView,
    granted: OpenRewardView,
    was_compensated: bool,
}
