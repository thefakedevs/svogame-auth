use anyhow::{Result, bail};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, Condition, EntityTrait, PaginatorTrait,
    QueryFilter, QueryOrder,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::entities::{
    AssetDefinition, AssetDefinitionActiveModel, AssetDefinitionColumn, AssetDefinitionModel,
};
use crate::services::ownership::types::{
    AssetKind, OwnershipModel, normalize_metadata, validate_asset_key,
};

#[derive(Clone, Debug, Serialize)]
pub struct AssetDefinitionView {
    pub id: Uuid,
    pub key: String,
    pub display_name: String,
    pub description: Option<String>,
    pub asset_kind: AssetKind,
    pub ownership_model: OwnershipModel,
    pub is_currency: bool,
    pub is_user_purchasable: bool,
    pub is_public: bool,
    pub is_active: bool,
    pub metadata: Value,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl TryFrom<AssetDefinitionModel> for AssetDefinitionView {
    type Error = anyhow::Error;

    fn try_from(value: AssetDefinitionModel) -> Result<Self> {
        Ok(Self {
            id: value.id,
            key: value.key,
            display_name: value.display_name,
            description: value.description,
            asset_kind: AssetKind::parse(&value.asset_kind)?,
            ownership_model: OwnershipModel::parse(&value.ownership_model)?,
            is_currency: value.is_currency,
            is_user_purchasable: value.is_user_purchasable,
            is_public: value.is_public,
            is_active: value.is_active,
            metadata: serde_json::from_str(&value.metadata)?,
            created_at: value.created_at,
            updated_at: value.updated_at,
        })
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateAssetDefinitionInput {
    pub key: String,
    pub display_name: String,
    pub description: Option<String>,
    pub asset_kind: AssetKind,
    pub ownership_model: OwnershipModel,
    pub is_currency: bool,
    pub is_user_purchasable: bool,
    pub is_public: bool,
    pub metadata: Option<Value>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateAssetDefinitionInput {
    pub display_name: Option<String>,
    pub description: Option<Option<String>>,
    pub is_user_purchasable: Option<bool>,
    pub is_public: Option<bool>,
    pub is_active: Option<bool>,
    pub metadata: Option<Value>,
}

#[derive(Debug, Deserialize, Default)]
pub struct AssetDefinitionQuery {
    pub q: Option<String>,
    pub asset_kind: Option<String>,
    pub ownership_model: Option<String>,
    pub is_currency: Option<bool>,
    pub is_public: Option<bool>,
    pub is_user_purchasable: Option<bool>,
    pub is_active: Option<bool>,
    pub page: Option<u64>,
    pub per_page: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct AssetDefinitionListResult {
    pub items: Vec<AssetDefinitionView>,
    pub total: u64,
    pub page: u64,
    pub per_page: u64,
    pub total_pages: u64,
}

const DEFAULT_PAGE: u64 = 1;
const DEFAULT_PER_PAGE: u64 = 20;
const MAX_PER_PAGE: u64 = 100;

pub async fn create_asset_definition(
    db: &sea_orm::DatabaseConnection,
    input: CreateAssetDefinitionInput,
) -> Result<AssetDefinitionView> {
    let key = validate_asset_key(&input.key)?;
    validate_asset_type_compatibility(input.is_currency, &input.asset_kind, &input.ownership_model)?;
    if input.display_name.trim().is_empty() {
        bail!("Display name cannot be empty");
    }

    let existing = AssetDefinition::find()
        .filter(AssetDefinitionColumn::Key.eq(&key))
        .one(db)
        .await?;
    if existing.is_some() {
        bail!("Asset key already exists");
    }

    let now = chrono::Utc::now();
    let model = AssetDefinitionActiveModel {
        id: Set(Uuid::new_v4()),
        key: Set(key),
        display_name: Set(input.display_name.trim().to_string()),
        description: Set(
            input
                .description
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty()),
        ),
        asset_kind: Set(input.asset_kind.as_str().to_string()),
        ownership_model: Set(input.ownership_model.as_str().to_string()),
        is_currency: Set(input.is_currency),
        is_user_purchasable: Set(input.is_user_purchasable),
        is_public: Set(input.is_public),
        is_active: Set(true),
        metadata: Set(normalize_metadata(input.metadata).to_string()),
        created_at: Set(now),
        updated_at: Set(now),
    };

    AssetDefinitionView::try_from(model.insert(db).await?)
}

pub async fn get_asset_definition_by_id(
    db: &sea_orm::DatabaseConnection,
    asset_id: Uuid,
) -> Result<Option<AssetDefinitionView>> {
    AssetDefinition::find_by_id(asset_id)
        .one(db)
        .await?
        .map(AssetDefinitionView::try_from)
        .transpose()
}

pub async fn get_asset_definition_by_key(
    db: &sea_orm::DatabaseConnection,
    key: &str,
) -> Result<Option<AssetDefinitionView>> {
    let key = validate_asset_key(key)?;
    AssetDefinition::find()
        .filter(AssetDefinitionColumn::Key.eq(key))
        .one(db)
        .await?
        .map(AssetDefinitionView::try_from)
        .transpose()
}

pub async fn list_asset_definitions(
    db: &sea_orm::DatabaseConnection,
    query: AssetDefinitionQuery,
    admin_view: bool,
) -> Result<AssetDefinitionListResult> {
    let page = query.page.unwrap_or(DEFAULT_PAGE).max(1);
    let per_page = query.per_page.unwrap_or(DEFAULT_PER_PAGE).clamp(1, MAX_PER_PAGE);
    let mut find = AssetDefinition::find().order_by_desc(AssetDefinitionColumn::CreatedAt);

    if !admin_view {
        find = find.filter(AssetDefinitionColumn::IsPublic.eq(true));
        find = find.filter(AssetDefinitionColumn::IsActive.eq(true));
    }

    if let Some(q) = query.q.as_ref().map(|value| value.trim()).filter(|value| !value.is_empty()) {
        find = find.filter(
            Condition::any()
                .add(AssetDefinitionColumn::Key.contains(q))
                .add(AssetDefinitionColumn::DisplayName.contains(q)),
        );
    }
    if let Some(asset_kind) = query.asset_kind.as_deref() {
        find = find.filter(AssetDefinitionColumn::AssetKind.eq(AssetKind::parse(asset_kind)?.as_str()));
    }
    if let Some(ownership_model) = query.ownership_model.as_deref() {
        find = find.filter(
            AssetDefinitionColumn::OwnershipModel.eq(OwnershipModel::parse(ownership_model)?.as_str()),
        );
    }
    if let Some(is_currency) = query.is_currency {
        find = find.filter(AssetDefinitionColumn::IsCurrency.eq(is_currency));
    }
    if let Some(is_public) = query.is_public {
        find = find.filter(AssetDefinitionColumn::IsPublic.eq(is_public));
    }
    if let Some(is_user_purchasable) = query.is_user_purchasable {
        find = find.filter(AssetDefinitionColumn::IsUserPurchasable.eq(is_user_purchasable));
    }
    if let Some(is_active) = query.is_active {
        find = find.filter(AssetDefinitionColumn::IsActive.eq(is_active));
    }

    let paginator = find.paginate(db, per_page);
    let total = paginator.num_items().await?;
    let items = paginator.fetch_page(page - 1).await?;
    let total_pages = if total == 0 { 0 } else { total.div_ceil(per_page) };

    Ok(AssetDefinitionListResult {
        items: items
            .into_iter()
            .map(AssetDefinitionView::try_from)
            .collect::<Result<Vec<_>>>()?,
        total,
        page,
        per_page,
        total_pages,
    })
}

pub async fn update_asset_definition(
    db: &sea_orm::DatabaseConnection,
    asset_id: Uuid,
    input: UpdateAssetDefinitionInput,
) -> Result<AssetDefinitionView> {
    let model = AssetDefinition::find_by_id(asset_id)
        .one(db)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Asset not found"))?;
    let mut active_model: AssetDefinitionActiveModel = model.into();

    if let Some(display_name) = input.display_name {
        let normalized = display_name.trim();
        if normalized.is_empty() {
            bail!("Display name cannot be empty");
        }
        active_model.display_name = Set(normalized.to_string());
    }
    if let Some(description) = input.description {
        active_model.description = Set(description.map(|value| value.trim().to_string()).filter(|value| !value.is_empty()));
    }
    if let Some(is_user_purchasable) = input.is_user_purchasable {
        active_model.is_user_purchasable = Set(is_user_purchasable);
    }
    if let Some(is_public) = input.is_public {
        active_model.is_public = Set(is_public);
    }
    if let Some(is_active) = input.is_active {
        active_model.is_active = Set(is_active);
    }
    if let Some(metadata) = input.metadata {
        active_model.metadata = Set(normalize_metadata(Some(metadata)).to_string());
    }
    active_model.updated_at = Set(chrono::Utc::now());

    AssetDefinitionView::try_from(active_model.update(db).await?)
}

fn validate_asset_type_compatibility(
    is_currency: bool,
    asset_kind: &AssetKind,
    ownership_model: &OwnershipModel,
) -> Result<()> {
    if is_currency && *ownership_model != OwnershipModel::Stackable {
        bail!("Currency assets must use stackable ownership");
    }
    if *asset_kind == AssetKind::Currency && !is_currency {
        bail!("Currency asset kind must set is_currency=true");
    }
    if is_currency && *asset_kind != AssetKind::Currency {
        bail!("Currency assets must use currency asset kind");
    }
    Ok(())
}
