use anyhow::{Result, anyhow, bail};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter,
    QueryOrder, TransactionTrait,
};
use serde::Serialize;
use uuid::Uuid;

use crate::entities::{
    AssetDefinition, AssetDefinitionColumn, AssetDefinitionModel, User, UserEntitlement,
    UserExpirableAsset, UserSelectedGunskin, UserSelectedGunskinActiveModel,
    UserSelectedGunskinModel, UserStackableAsset,
};
use crate::services::ownership::types::{
    AssetKind, OperationContext, OwnershipActor, OwnershipModel, SkinRarity, validate_asset_key,
    validate_weapon_key,
};

#[derive(Clone, Debug, Serialize)]
pub struct GunskinAssetView {
    pub id: Uuid,
    pub key: String,
    pub display_name: String,
    pub description: Option<String>,
    pub weapon_key: String,
    pub rarity: SkinRarity,
    pub ownership_model: OwnershipModel,
}

#[derive(Clone, Debug, Serialize)]
pub struct SelectedGunskinView {
    pub weapon_key: String,
    pub asset_definition_id: Uuid,
    pub asset_key: String,
    pub display_name: String,
    pub description: Option<String>,
    pub rarity: SkinRarity,
    pub selected_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Clone, Debug, Serialize)]
pub struct GunskinCollectionView {
    pub weapon_key: String,
    pub selected: Option<SelectedGunskinView>,
    pub available: Vec<GunskinAssetView>,
}

#[derive(Clone, Debug, Serialize)]
pub struct GunskinSelectionListItemView {
    pub weapon_key: String,
    pub selected: SelectedGunskinView,
}

#[derive(Clone, Debug)]
pub struct SelectGunskinMutation {
    pub user_id: Uuid,
    pub weapon_key: String,
    pub asset_key: String,
    pub actor: OwnershipActor,
    pub context: OperationContext,
}

pub async fn list_weapon_keys(db: &sea_orm::DatabaseConnection) -> Result<Vec<String>> {
    let mut items: Vec<String> = AssetDefinition::find()
        .filter(AssetDefinitionColumn::AssetKind.eq(AssetKind::Skin.as_str()))
        .filter(AssetDefinitionColumn::IsActive.eq(true))
        .all(db)
        .await?
        .into_iter()
        .filter_map(|asset| asset.weapon_key)
        .collect();
    items.sort();
    items.dedup();
    Ok(items)
}

pub async fn list_user_selections(
    db: &sea_orm::DatabaseConnection,
    user_id: Uuid,
) -> Result<Vec<GunskinSelectionListItemView>> {
    ensure_user_exists(db, user_id).await?;
    let selections = UserSelectedGunskin::find()
        .filter(crate::entities::UserSelectedGunskinColumn::UserId.eq(user_id))
        .order_by_asc(crate::entities::UserSelectedGunskinColumn::WeaponKey)
        .all(db)
        .await?;

    let mut items = Vec::new();
    for selection in selections {
        if let Some(selected) = get_selected_gunskin(db, user_id, &selection.weapon_key).await? {
            items.push(GunskinSelectionListItemView {
                weapon_key: selection.weapon_key,
                selected,
            });
        }
    }

    Ok(items)
}

pub async fn get_user_gunskin_collection(
    db: &sea_orm::DatabaseConnection,
    user_id: Uuid,
    weapon_key: &str,
) -> Result<GunskinCollectionView> {
    let weapon_key = validate_weapon_key(weapon_key)?;
    ensure_user_exists(db, user_id).await?;
    let available = list_available_gunskins_for_user(db, user_id, &weapon_key).await?;
    let selected = get_selected_gunskin(db, user_id, &weapon_key).await?;

    Ok(GunskinCollectionView {
        weapon_key,
        selected,
        available,
    })
}

pub async fn list_available_gunskins_for_user(
    db: &sea_orm::DatabaseConnection,
    user_id: Uuid,
    weapon_key: &str,
) -> Result<Vec<GunskinAssetView>> {
    let weapon_key = validate_weapon_key(weapon_key)?;
    ensure_user_exists(db, user_id).await?;
    let assets = load_skin_assets_for_weapon(db, &weapon_key).await?;
    let mut result = Vec::new();

    for asset in assets {
        if is_asset_owned_by_user(db, user_id, &asset).await? {
            result.push(map_gunskin_asset(asset)?);
        }
    }

    Ok(result)
}

pub async fn get_selected_gunskin(
    db: &sea_orm::DatabaseConnection,
    user_id: Uuid,
    weapon_key: &str,
) -> Result<Option<SelectedGunskinView>> {
    let weapon_key = validate_weapon_key(weapon_key)?;
    ensure_user_exists(db, user_id).await?;
    load_valid_selection(db, user_id, &weapon_key, true).await
}

pub async fn select_gunskin(
    db: &sea_orm::DatabaseConnection,
    mutation: SelectGunskinMutation,
) -> Result<SelectedGunskinView> {
    mutation.actor.validate()?;
    let asset_key = validate_asset_key(&mutation.asset_key)?;
    let weapon_key = validate_weapon_key(&mutation.weapon_key)?;
    let tx = db.begin().await?;
    ensure_user_exists(&tx, mutation.user_id).await?;
    let asset = get_skin_asset_by_key(&tx, &asset_key).await?;

    if asset.weapon_key.as_deref() != Some(weapon_key.as_str()) {
        bail!("Gunskin is not applicable to the requested weapon");
    }
    if !is_asset_owned_by_user(&tx, mutation.user_id, &asset).await? {
        bail!("User does not own this gunskin");
    }

    let now = chrono::Utc::now();
    let stored = if let Some(existing) = UserSelectedGunskin::find_by_id((mutation.user_id, weapon_key.clone()))
        .one(&tx)
        .await?
    {
        let mut active: UserSelectedGunskinActiveModel = existing.into();
        active.asset_definition_id = Set(asset.id);
        active.updated_at = Set(now);
        active.update(&tx).await?
    } else {
        UserSelectedGunskinActiveModel {
            user_id: Set(mutation.user_id),
            weapon_key: Set(weapon_key.clone()),
            asset_definition_id: Set(asset.id),
            selected_at: Set(now),
            updated_at: Set(now),
        }
        .insert(&tx)
        .await?
    };

    let selected = map_selected_gunskin(stored, asset)?;
    let _ = mutation.context;
    tx.commit().await?;
    Ok(selected)
}

pub async fn reset_gunskin_selection(
    db: &sea_orm::DatabaseConnection,
    user_id: Uuid,
    weapon_key: &str,
) -> Result<()> {
    let weapon_key = validate_weapon_key(weapon_key)?;
    let tx = db.begin().await?;
    ensure_user_exists(&tx, user_id).await?;
    UserSelectedGunskin::delete_by_id((user_id, weapon_key))
        .exec(&tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

pub async fn clear_selection_if_asset_no_longer_owned(
    db: &impl ConnectionTrait,
    user_id: Uuid,
    asset_definition_id: Uuid,
) -> Result<()> {
    let Some(asset) = AssetDefinition::find_by_id(asset_definition_id).one(db).await? else {
        return Ok(());
    };
    if asset.asset_kind != AssetKind::Skin.as_str() {
        return Ok(());
    }
    let Some(weapon_key) = asset.weapon_key.clone() else {
        return Ok(());
    };
    let Some(selection) = UserSelectedGunskin::find_by_id((user_id, weapon_key.clone()))
        .one(db)
        .await?
    else {
        return Ok(());
    };
    if selection.asset_definition_id != asset_definition_id {
        return Ok(());
    }
    if is_asset_owned_by_user(db, user_id, &asset).await? {
        return Ok(());
    }

    UserSelectedGunskin::delete_by_id((user_id, weapon_key))
        .exec(db)
        .await?;
    Ok(())
}

async fn load_valid_selection(
    db: &sea_orm::DatabaseConnection,
    user_id: Uuid,
    weapon_key: &str,
    cleanup_invalid: bool,
) -> Result<Option<SelectedGunskinView>> {
    let Some(selection) = UserSelectedGunskin::find_by_id((user_id, weapon_key.to_string()))
        .one(db)
        .await?
    else {
        return Ok(None);
    };

    let Some(asset) = AssetDefinition::find_by_id(selection.asset_definition_id).one(db).await? else {
        if cleanup_invalid {
            UserSelectedGunskin::delete_by_id((user_id, weapon_key.to_string()))
                .exec(db)
                .await?;
        }
        return Ok(None);
    };

    if asset.asset_kind != AssetKind::Skin.as_str()
        || asset.weapon_key.as_deref() != Some(weapon_key)
        || !asset.is_active
        || !is_asset_owned_by_user(db, user_id, &asset).await?
    {
        if cleanup_invalid {
            UserSelectedGunskin::delete_by_id((user_id, weapon_key.to_string()))
                .exec(db)
                .await?;
        }
        return Ok(None);
    }

    Ok(Some(map_selected_gunskin(selection, asset)?))
}

async fn ensure_user_exists(db: &impl ConnectionTrait, user_id: Uuid) -> Result<()> {
    if User::find_by_id(user_id).one(db).await?.is_none() {
        bail!("User not found");
    }
    Ok(())
}

async fn load_skin_assets_for_weapon(
    db: &sea_orm::DatabaseConnection,
    weapon_key: &str,
) -> Result<Vec<AssetDefinitionModel>> {
    AssetDefinition::find()
        .filter(AssetDefinitionColumn::AssetKind.eq(AssetKind::Skin.as_str()))
        .filter(AssetDefinitionColumn::IsActive.eq(true))
        .filter(AssetDefinitionColumn::WeaponKey.eq(weapon_key))
        .order_by_asc(AssetDefinitionColumn::DisplayName)
        .all(db)
        .await
        .map_err(Into::into)
}

async fn get_skin_asset_by_key(
    db: &impl ConnectionTrait,
    asset_key: &str,
) -> Result<AssetDefinitionModel> {
    let asset = AssetDefinition::find()
        .filter(AssetDefinitionColumn::Key.eq(asset_key))
        .one(db)
        .await?
        .ok_or_else(|| anyhow!("Gunskin not found"))?;
    if !asset.is_active {
        bail!("Gunskin is inactive");
    }
    if asset.asset_kind != AssetKind::Skin.as_str() {
        bail!("Asset is not a gunskin");
    }
    if asset.is_currency {
        bail!("Gunskin cannot be a currency asset");
    }
    if asset.weapon_key.as_deref().is_none_or(|value| value.trim().is_empty()) {
        bail!("Gunskin is missing weapon key");
    }
    if asset.rarity.as_deref().is_none_or(|value| value.trim().is_empty()) {
        bail!("Gunskin is missing rarity");
    }
    Ok(asset)
}

async fn is_asset_owned_by_user(
    db: &impl ConnectionTrait,
    user_id: Uuid,
    asset: &AssetDefinitionModel,
) -> Result<bool> {
    match OwnershipModel::parse(&asset.ownership_model)? {
        OwnershipModel::Stackable => Ok(UserStackableAsset::find_by_id((user_id, asset.id))
            .one(db)
            .await?
            .is_some_and(|holding| holding.amount > 0)),
        OwnershipModel::Entitlement => Ok(UserEntitlement::find_by_id((user_id, asset.id))
            .one(db)
            .await?
            .is_some()),
        OwnershipModel::Expirable => Ok(UserExpirableAsset::find_by_id((user_id, asset.id))
            .one(db)
            .await?
            .is_some_and(|holding| holding.expires_at > chrono::Utc::now())),
    }
}

fn map_gunskin_asset(asset: AssetDefinitionModel) -> Result<GunskinAssetView> {
    Ok(GunskinAssetView {
        id: asset.id,
        key: asset.key,
        display_name: asset.display_name,
        description: asset.description,
        weapon_key: asset
            .weapon_key
            .ok_or_else(|| anyhow!("Gunskin is missing weapon key"))?,
        rarity: SkinRarity::parse(
            asset.rarity.as_deref().ok_or_else(|| anyhow!("Gunskin is missing rarity"))?,
        )?,
        ownership_model: OwnershipModel::parse(&asset.ownership_model)?,
    })
}

fn map_selected_gunskin(
    selection: UserSelectedGunskinModel,
    asset: AssetDefinitionModel,
) -> Result<SelectedGunskinView> {
    Ok(SelectedGunskinView {
        weapon_key: selection.weapon_key,
        asset_definition_id: asset.id,
        asset_key: asset.key,
        display_name: asset.display_name,
        description: asset.description,
        rarity: SkinRarity::parse(
            asset.rarity.as_deref().ok_or_else(|| anyhow!("Gunskin is missing rarity"))?,
        )?,
        selected_at: selection.selected_at,
        updated_at: selection.updated_at,
    })
}
