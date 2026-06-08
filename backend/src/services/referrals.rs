use anyhow::{Result, anyhow, bail};
use chrono::{DateTime, Datelike, NaiveDate, TimeZone, Utc};
use regex::Regex;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, EntityTrait, PaginatorTrait,
    QueryFilter, QueryOrder, TransactionTrait,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::entities::{
    AssetDefinition, AssetDefinitionColumn, AssetDefinitionModel, ReferralCampaign,
    ReferralCampaignActiveModel, ReferralCampaignColumn, ReferralCampaignModel,
    ReferralCampaignReward, ReferralCampaignRewardActiveModel, ReferralCampaignRewardColumn,
    ReferralCampaignRewardModel, ReferralRegistration, ReferralRegistrationActiveModel,
    ReferralRegistrationColumn, ReferralRegistrationModel, User,
};
use crate::services::ownership::inventory::{
    EntitlementMutation, ProlongExpirableMutation, StackableMutation, add_stackable_in_tx,
    grant_entitlement_in_tx, prolong_expirable_in_tx,
};
use crate::services::ownership::types::{OperationContext, OwnershipActor, OwnershipModel};
use crate::services::ownership::wallet::{WalletMutation, credit_in_tx};

pub const CAMPAIGN_STATUS_ACTIVE: &str = "active";
pub const CAMPAIGN_STATUS_DRAFT: &str = "draft";
pub const CAMPAIGN_STATUS_REVOKED: &str = "revoked";
pub const REGISTRATION_SOURCE_LINK: &str = "link";
pub const REGISTRATION_SOURCE_MANUAL: &str = "manual";
pub const REWARD_STATUS_GRANTED: &str = "granted";
pub const REWARD_REASON_CODE: &str = "referral_welcome_reward";

const CODE_REGEX: &str = r"^[A-Z0-9_-]{3,32}$";
const DEFAULT_STATS_DAYS: i64 = 30;

#[derive(Clone, Debug, Serialize, ToSchema)]
pub struct ReferralRewardView {
    #[serde(rename = "assetKey")]
    pub asset_key: String,
    #[serde(rename = "assetDisplayName")]
    pub asset_display_name: String,
    #[serde(rename = "assetKind")]
    pub asset_kind: String,
    #[serde(rename = "ownershipModel")]
    pub ownership_model: String,
    #[serde(rename = "isCurrency")]
    pub is_currency: bool,
    pub amount: Option<i64>,
    #[serde(rename = "durationSeconds")]
    pub duration_seconds: Option<i64>,
    pub metadata: Value,
}

#[derive(Clone, Debug, Serialize, ToSchema)]
pub struct ReferralCampaignView {
    pub id: Uuid,
    pub code: String,
    pub title: String,
    #[serde(rename = "contentCreatorUserId")]
    pub content_creator_user_id: Option<Uuid>,
    pub status: String,
    #[serde(rename = "startsAt")]
    pub starts_at: Option<DateTime<Utc>>,
    #[serde(rename = "endsAt")]
    pub ends_at: Option<DateTime<Utc>>,
    #[serde(rename = "createdByUserId")]
    pub created_by_user_id: Uuid,
    #[serde(rename = "revokedAt")]
    pub revoked_at: Option<DateTime<Utc>>,
    #[serde(rename = "createdAt")]
    pub created_at: DateTime<Utc>,
    #[serde(rename = "updatedAt")]
    pub updated_at: DateTime<Utc>,
    pub rewards: Vec<ReferralRewardView>,
}

#[derive(Clone, Debug, Serialize, ToSchema)]
pub struct PublicReferralCampaignView {
    pub code: String,
    pub title: String,
    #[serde(rename = "contentCreatorUserId")]
    pub content_creator_user_id: Option<Uuid>,
    pub rewards: Vec<ReferralRewardView>,
}

#[derive(Clone, Debug, Serialize, ToSchema)]
pub struct ReferralCampaignListResponse {
    pub items: Vec<ReferralCampaignView>,
    pub total: u64,
    pub page: u64,
    #[serde(rename = "perPage")]
    pub per_page: u64,
    #[serde(rename = "totalPages")]
    pub total_pages: u64,
}

#[derive(Clone, Debug, Deserialize, ToSchema)]
pub struct ReferralRewardInput {
    #[serde(rename = "assetKey")]
    pub asset_key: String,
    pub amount: Option<i64>,
    #[serde(rename = "durationSeconds")]
    pub duration_seconds: Option<i64>,
    #[serde(default)]
    pub metadata: Value,
}

#[derive(Clone, Debug, Deserialize, ToSchema)]
pub struct CreateReferralCampaignInput {
    pub code: String,
    pub title: String,
    #[serde(rename = "contentCreatorUserId")]
    pub content_creator_user_id: Option<Uuid>,
    pub status: Option<String>,
    #[serde(rename = "startsAt")]
    pub starts_at: Option<DateTime<Utc>>,
    #[serde(rename = "endsAt")]
    pub ends_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub rewards: Vec<ReferralRewardInput>,
}

#[derive(Clone, Debug, Deserialize, ToSchema)]
pub struct UpdateReferralCampaignInput {
    pub title: Option<String>,
    #[serde(rename = "contentCreatorUserId")]
    pub content_creator_user_id: Option<Option<Uuid>>,
    pub status: Option<String>,
    #[serde(rename = "startsAt")]
    pub starts_at: Option<Option<DateTime<Utc>>>,
    #[serde(rename = "endsAt")]
    pub ends_at: Option<Option<DateTime<Utc>>>,
    pub rewards: Option<Vec<ReferralRewardInput>>,
}

#[derive(Clone, Debug, Serialize, ToSchema)]
pub struct ReferralStatsBucket {
    pub date: String,
    pub registrations: u64,
}

#[derive(Clone, Debug, Serialize, ToSchema)]
pub struct ReferralStatsResponse {
    pub total: u64,
    pub buckets: Vec<ReferralStatsBucket>,
}

pub async fn list_campaigns(
    db: &sea_orm::DatabaseConnection,
    page: u64,
    per_page: u64,
) -> Result<ReferralCampaignListResponse> {
    let paginator = ReferralCampaign::find()
        .order_by_desc(ReferralCampaignColumn::CreatedAt)
        .paginate(db, per_page);
    let total = paginator.num_items().await?;
    let campaigns = paginator.fetch_page(page.saturating_sub(1)).await?;
    let rewards = ReferralCampaignReward::find().all(db).await?;

    let mut items = Vec::new();
    for campaign in campaigns {
        items.push(map_campaign_view(db, campaign, &rewards).await?);
    }

    let total_pages = if total == 0 {
        0
    } else {
        total.div_ceil(per_page)
    };
    Ok(ReferralCampaignListResponse {
        items,
        total,
        page,
        per_page,
        total_pages,
    })
}

pub async fn get_campaign(
    db: &sea_orm::DatabaseConnection,
    campaign_id: Uuid,
) -> Result<ReferralCampaignView> {
    let campaign = ReferralCampaign::find_by_id(campaign_id)
        .one(db)
        .await?
        .ok_or_else(|| anyhow!("Referral campaign not found"))?;
    let rewards = ReferralCampaignReward::find()
        .filter(ReferralCampaignRewardColumn::CampaignId.eq(campaign_id))
        .order_by_asc(ReferralCampaignRewardColumn::SortOrder)
        .all(db)
        .await?;
    map_campaign_view(db, campaign, &rewards).await
}

pub async fn get_public_campaign(
    db: &sea_orm::DatabaseConnection,
    code: &str,
) -> Result<PublicReferralCampaignView> {
    let normalized_code = normalize_referral_code(code)?;
    let campaign = get_active_campaign_by_code(db, &normalized_code).await?;
    let rewards = ReferralCampaignReward::find()
        .filter(ReferralCampaignRewardColumn::CampaignId.eq(campaign.id))
        .order_by_asc(ReferralCampaignRewardColumn::SortOrder)
        .all(db)
        .await?;
    let view = map_campaign_view(db, campaign, &rewards).await?;
    Ok(PublicReferralCampaignView {
        code: view.code,
        title: view.title,
        content_creator_user_id: view.content_creator_user_id,
        rewards: view.rewards,
    })
}

pub async fn create_campaign(
    db: &sea_orm::DatabaseConnection,
    actor_user_id: Uuid,
    input: CreateReferralCampaignInput,
) -> Result<ReferralCampaignView> {
    let code = normalize_referral_code(&input.code)?;
    let title = normalize_title(&input.title)?;
    let status =
        normalize_campaign_status(input.status.as_deref().unwrap_or(CAMPAIGN_STATUS_ACTIVE))?;
    validate_time_window(input.starts_at.clone(), input.ends_at.clone())?;
    let tx = db.begin().await?;
    ensure_content_creator_exists(&tx, input.content_creator_user_id.clone()).await?;
    validate_rewards(&tx, &input.rewards).await?;

    let now = Utc::now();
    let campaign = ReferralCampaignActiveModel {
        id: Set(Uuid::new_v4()),
        code: Set(code),
        title: Set(title),
        content_creator_user_id: Set(input.content_creator_user_id),
        status: Set(status),
        starts_at: Set(input.starts_at),
        ends_at: Set(input.ends_at),
        created_by_user_id: Set(actor_user_id),
        revoked_at: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
    }
    .insert(&tx)
    .await?;

    replace_rewards(&tx, campaign.id, input.rewards).await?;
    tx.commit().await?;
    get_campaign(db, campaign.id).await
}

pub async fn update_campaign(
    db: &sea_orm::DatabaseConnection,
    campaign_id: Uuid,
    input: UpdateReferralCampaignInput,
) -> Result<ReferralCampaignView> {
    let tx = db.begin().await?;
    let campaign = ReferralCampaign::find_by_id(campaign_id)
        .one(&tx)
        .await?
        .ok_or_else(|| anyhow!("Referral campaign not found"))?;
    ensure_content_creator_exists(&tx, input.content_creator_user_id.clone().flatten()).await?;

    let has_starts_at = input.starts_at.is_some();
    let has_ends_at = input.ends_at.is_some();
    let next_starts_at = input
        .starts_at
        .clone()
        .unwrap_or_else(|| campaign.starts_at.clone());
    let next_ends_at = input
        .ends_at
        .clone()
        .unwrap_or_else(|| campaign.ends_at.clone());
    validate_time_window(next_starts_at, next_ends_at)?;

    if let Some(rewards) = input.rewards.as_ref() {
        validate_rewards(&tx, rewards).await?;
    }

    let mut active: ReferralCampaignActiveModel = campaign.into();
    if let Some(title) = input.title {
        active.title = Set(normalize_title(&title)?);
    }
    if let Some(content_creator_user_id) = input.content_creator_user_id {
        active.content_creator_user_id = Set(content_creator_user_id);
    }
    if let Some(status) = input.status {
        let status = normalize_campaign_status(&status)?;
        active.revoked_at = if status == CAMPAIGN_STATUS_REVOKED {
            Set(Some(Utc::now()))
        } else {
            Set(None)
        };
        active.status = Set(status);
    }
    if has_starts_at {
        active.starts_at = Set(next_starts_at);
    }
    if has_ends_at {
        active.ends_at = Set(next_ends_at);
    }
    active.updated_at = Set(Utc::now());
    let updated = active.update(&tx).await?;

    if let Some(rewards) = input.rewards {
        replace_rewards(&tx, campaign_id, rewards).await?;
    }

    tx.commit().await?;
    get_campaign(db, updated.id).await
}

pub async fn revoke_campaign(
    db: &sea_orm::DatabaseConnection,
    campaign_id: Uuid,
) -> Result<ReferralCampaignView> {
    update_campaign(
        db,
        campaign_id,
        UpdateReferralCampaignInput {
            title: None,
            content_creator_user_id: None,
            status: Some(CAMPAIGN_STATUS_REVOKED.to_string()),
            starts_at: None,
            ends_at: None,
            rewards: None,
        },
    )
    .await
}

pub async fn apply_registration_referral(
    db: &impl ConnectionTrait,
    user_id: Uuid,
    code: &str,
    source: &str,
) -> Result<Option<ReferralRegistrationModel>> {
    let code = normalize_referral_code(code)?;
    let source = normalize_registration_source(source)?;
    let campaign = get_active_campaign_by_code(db, &code).await?;
    if campaign
        .content_creator_user_id
        .as_ref()
        .is_some_and(|creator_user_id| *creator_user_id == user_id)
    {
        return Ok(None);
    }

    let registration = ReferralRegistrationActiveModel {
        id: Set(Uuid::new_v4()),
        campaign_id: Set(campaign.id),
        user_id: Set(user_id),
        code_snapshot: Set(campaign.code.clone()),
        campaign_title_snapshot: Set(campaign.title.clone()),
        content_creator_user_id_snapshot: Set(campaign.content_creator_user_id),
        source: Set(source),
        reward_status: Set(REWARD_STATUS_GRANTED.to_string()),
        reward_error: Set(None),
        metadata: Set(json!({}).to_string()),
        registered_at: Set(Utc::now()),
    }
    .insert(db)
    .await?;

    let rewards = ReferralCampaignReward::find()
        .filter(ReferralCampaignRewardColumn::CampaignId.eq(campaign.id))
        .order_by_asc(ReferralCampaignRewardColumn::SortOrder)
        .all(db)
        .await?;
    for reward in rewards {
        grant_reward(db, user_id, &campaign, &registration, &reward).await?;
    }

    Ok(Some(registration))
}

pub async fn campaign_stats(
    db: &sea_orm::DatabaseConnection,
    campaign_id: Uuid,
    from: Option<DateTime<Utc>>,
    to: Option<DateTime<Utc>>,
) -> Result<ReferralStatsResponse> {
    if ReferralCampaign::find_by_id(campaign_id).one(db).await?.is_none() {
        bail!("Referral campaign not found");
    }
    let (from, to) = normalize_stats_window(from, to)?;
    let registrations = ReferralRegistration::find()
        .filter(ReferralRegistrationColumn::CampaignId.eq(campaign_id))
        .filter(ReferralRegistrationColumn::RegisteredAt.gte(from))
        .filter(ReferralRegistrationColumn::RegisteredAt.lte(to))
        .all(db)
        .await?;
    Ok(build_daily_stats(registrations, from, to))
}

pub async fn creator_stats(
    db: &sea_orm::DatabaseConnection,
    creator_user_id: Uuid,
    from: Option<DateTime<Utc>>,
    to: Option<DateTime<Utc>>,
) -> Result<ReferralStatsResponse> {
    let (from, to) = normalize_stats_window(from, to)?;
    let registrations = ReferralRegistration::find()
        .filter(ReferralRegistrationColumn::ContentCreatorUserIdSnapshot.eq(Some(creator_user_id)))
        .filter(ReferralRegistrationColumn::RegisteredAt.gte(from))
        .filter(ReferralRegistrationColumn::RegisteredAt.lte(to))
        .all(db)
        .await?;
    Ok(build_daily_stats(registrations, from, to))
}

pub fn normalize_referral_code(code: &str) -> Result<String> {
    let normalized = code.trim().to_uppercase();
    if normalized.is_empty() {
        bail!("Referral code cannot be empty");
    }
    let regex = Regex::new(CODE_REGEX)?;
    if !regex.is_match(&normalized) {
        bail!("Referral code must be 3-32 characters and contain only A-Z, 0-9, underscore, or hyphen");
    }
    Ok(normalized)
}

async fn get_active_campaign_by_code(
    db: &impl ConnectionTrait,
    code: &str,
) -> Result<ReferralCampaignModel> {
    let campaign = ReferralCampaign::find()
        .filter(ReferralCampaignColumn::Code.eq(code))
        .one(db)
        .await?
        .ok_or_else(|| anyhow!("Referral campaign not found"))?;
    if !campaign_is_active(&campaign, Utc::now()) {
        bail!("Referral campaign is not active");
    }
    Ok(campaign)
}

fn campaign_is_active(campaign: &ReferralCampaignModel, now: DateTime<Utc>) -> bool {
    if campaign.status != CAMPAIGN_STATUS_ACTIVE || campaign.revoked_at.is_some() {
        return false;
    }
    if campaign.starts_at.as_ref().is_some_and(|starts_at| *starts_at > now) {
        return false;
    }
    if campaign.ends_at.as_ref().is_some_and(|ends_at| *ends_at < now) {
        return false;
    }
    true
}

async fn map_campaign_view(
    db: &impl ConnectionTrait,
    campaign: ReferralCampaignModel,
    all_rewards: &[ReferralCampaignRewardModel],
) -> Result<ReferralCampaignView> {
    let rewards = all_rewards
        .iter()
        .filter(|reward| reward.campaign_id == campaign.id)
        .cloned()
        .collect::<Vec<_>>();
    let rewards = map_reward_views(db, rewards).await?;
    Ok(ReferralCampaignView {
        id: campaign.id,
        code: campaign.code,
        title: campaign.title,
        content_creator_user_id: campaign.content_creator_user_id,
        status: campaign.status,
        starts_at: campaign.starts_at,
        ends_at: campaign.ends_at,
        created_by_user_id: campaign.created_by_user_id,
        revoked_at: campaign.revoked_at,
        created_at: campaign.created_at,
        updated_at: campaign.updated_at,
        rewards,
    })
}

async fn map_reward_views(
    db: &impl ConnectionTrait,
    rewards: Vec<ReferralCampaignRewardModel>,
) -> Result<Vec<ReferralRewardView>> {
    let assets = AssetDefinition::find().all(db).await?;
    rewards
        .into_iter()
        .map(|reward| {
            let asset = assets
                .iter()
                .find(|asset| asset.key == reward.asset_key)
                .ok_or_else(|| anyhow!("Missing reward asset"))?;
            Ok(ReferralRewardView {
                asset_key: reward.asset_key,
                asset_display_name: asset.display_name.clone(),
                asset_kind: asset.asset_kind.clone(),
                ownership_model: asset.ownership_model.clone(),
                is_currency: asset.is_currency,
                amount: reward.amount,
                duration_seconds: reward.duration_seconds,
                metadata: serde_json::from_str(&reward.metadata)?,
            })
        })
        .collect()
}

async fn replace_rewards(
    db: &impl ConnectionTrait,
    campaign_id: Uuid,
    rewards: Vec<ReferralRewardInput>,
) -> Result<()> {
    ReferralCampaignReward::delete_many()
        .filter(ReferralCampaignRewardColumn::CampaignId.eq(campaign_id))
        .exec(db)
        .await?;

    let now = Utc::now();
    for (index, reward) in rewards.into_iter().enumerate() {
        ReferralCampaignRewardActiveModel {
            id: Set(Uuid::new_v4()),
            campaign_id: Set(campaign_id),
            asset_key: Set(normalize_asset_key(&reward.asset_key)?),
            amount: Set(reward.amount),
            duration_seconds: Set(reward.duration_seconds),
            sort_order: Set(index as i32),
            metadata: Set(normalize_metadata(reward.metadata).to_string()),
            created_at: Set(now),
            updated_at: Set(now),
        }
        .insert(db)
        .await?;
    }

    Ok(())
}

async fn validate_rewards(db: &impl ConnectionTrait, rewards: &[ReferralRewardInput]) -> Result<()> {
    for reward in rewards {
        let asset_key = normalize_asset_key(&reward.asset_key)?;
        let asset = get_asset_by_key(db, &asset_key).await?;
        if !asset.is_active {
            bail!("Reward asset is inactive");
        }
        validate_reward_for_asset(&asset, reward)?;
    }
    Ok(())
}

fn validate_reward_for_asset(asset: &AssetDefinitionModel, reward: &ReferralRewardInput) -> Result<()> {
    let ownership_model = OwnershipModel::parse(&asset.ownership_model)?;
    if asset.is_currency {
        if ownership_model != OwnershipModel::Stackable {
            bail!("Currency reward must use stackable ownership");
        }
        if reward.amount.unwrap_or(0) <= 0 || reward.duration_seconds.is_some() {
            bail!("Currency reward requires positive amount and no duration");
        }
        return Ok(());
    }

    match ownership_model {
        OwnershipModel::Stackable => {
            if reward.amount.unwrap_or(0) <= 0 || reward.duration_seconds.is_some() {
                bail!("Stackable reward requires positive amount and no duration");
            }
        }
        OwnershipModel::Entitlement => {
            if reward.amount.is_some() || reward.duration_seconds.is_some() {
                bail!("Entitlement reward must not include amount or duration");
            }
        }
        OwnershipModel::Expirable => {
            if reward.duration_seconds.unwrap_or(0) <= 0 || reward.amount.is_some() {
                bail!("Expirable reward requires positive duration and no amount");
            }
        }
    }
    Ok(())
}

async fn grant_reward(
    db: &impl ConnectionTrait,
    user_id: Uuid,
    campaign: &ReferralCampaignModel,
    registration: &crate::entities::ReferralRegistrationModel,
    reward: &ReferralCampaignRewardModel,
) -> Result<()> {
    let asset = get_asset_by_key(db, &reward.asset_key).await?;
    if !asset.is_active {
        bail!("Reward asset is inactive");
    }
    let metadata = json!({
        "campaignId": campaign.id,
        "referralRegistrationId": registration.id,
        "code": registration.code_snapshot.clone(),
    });
    let context = OperationContext {
        reason_code: Some(REWARD_REASON_CODE.to_string()),
        reason_text: Some(format!("Referral campaign {}", campaign.code)),
        metadata,
    };
    let actor = OwnershipActor::system("referrals");

    if asset.is_currency {
        credit_in_tx(
            db,
            WalletMutation {
                user_id,
                currency_key: asset.key,
                amount: reward.amount.unwrap_or(0),
                actor,
                context,
            },
        )
        .await?;
        return Ok(());
    }

    match OwnershipModel::parse(&asset.ownership_model)? {
        OwnershipModel::Stackable => {
            add_stackable_in_tx(
                db,
                StackableMutation {
                    user_id,
                    asset_key: asset.key,
                    amount: reward.amount.unwrap_or(0),
                    actor,
                    context,
                },
            )
            .await?;
        }
        OwnershipModel::Entitlement => {
            grant_entitlement_in_tx(
                db,
                EntitlementMutation {
                    user_id,
                    asset_key: asset.key,
                    actor,
                    context,
                },
            )
            .await?;
        }
        OwnershipModel::Expirable => {
            prolong_expirable_in_tx(
                db,
                ProlongExpirableMutation {
                    user_id,
                    asset_key: asset.key,
                    duration_seconds: reward.duration_seconds.unwrap_or(0),
                    actor,
                    context,
                },
            )
            .await?;
        }
    }

    Ok(())
}

async fn get_asset_by_key(db: &impl ConnectionTrait, asset_key: &str) -> Result<AssetDefinitionModel> {
    AssetDefinition::find()
        .filter(AssetDefinitionColumn::Key.eq(asset_key))
        .one(db)
        .await?
        .ok_or_else(|| anyhow!("Reward asset not found"))
}

async fn ensure_content_creator_exists(
    db: &impl ConnectionTrait,
    user_id: Option<Uuid>,
) -> Result<()> {
    if let Some(user_id) = user_id {
        if User::find_by_id(user_id).one(db).await?.is_none() {
            bail!("Content creator user not found");
        }
    }
    Ok(())
}

fn normalize_title(title: &str) -> Result<String> {
    let title = title.trim();
    if title.is_empty() {
        bail!("Referral campaign title cannot be empty");
    }
    if title.chars().count() > 120 {
        bail!("Referral campaign title is too long");
    }
    Ok(title.to_string())
}

fn normalize_asset_key(asset_key: &str) -> Result<String> {
    let normalized = asset_key.trim().to_lowercase();
    if normalized.is_empty() {
        bail!("Reward asset key cannot be empty");
    }
    Ok(normalized)
}

fn normalize_campaign_status(status: &str) -> Result<String> {
    match status.trim() {
        CAMPAIGN_STATUS_ACTIVE => Ok(CAMPAIGN_STATUS_ACTIVE.to_string()),
        CAMPAIGN_STATUS_DRAFT => Ok(CAMPAIGN_STATUS_DRAFT.to_string()),
        CAMPAIGN_STATUS_REVOKED => Ok(CAMPAIGN_STATUS_REVOKED.to_string()),
        _ => bail!("Unsupported referral campaign status"),
    }
}

fn normalize_registration_source(source: &str) -> Result<String> {
    match source.trim() {
        REGISTRATION_SOURCE_LINK => Ok(REGISTRATION_SOURCE_LINK.to_string()),
        REGISTRATION_SOURCE_MANUAL => Ok(REGISTRATION_SOURCE_MANUAL.to_string()),
        _ => bail!("Unsupported referral source"),
    }
}

fn normalize_metadata(metadata: Value) -> Value {
    match metadata {
        Value::Null => Value::Object(Default::default()),
        value => value,
    }
}

fn validate_time_window(
    starts_at: Option<DateTime<Utc>>,
    ends_at: Option<DateTime<Utc>>,
) -> Result<()> {
    if let (Some(starts_at), Some(ends_at)) = (starts_at, ends_at)
        && ends_at <= starts_at
    {
        bail!("Referral campaign endsAt must be after startsAt");
    }
    Ok(())
}

fn normalize_stats_window(
    from: Option<DateTime<Utc>>,
    to: Option<DateTime<Utc>>,
) -> Result<(DateTime<Utc>, DateTime<Utc>)> {
    let to = to.unwrap_or_else(Utc::now);
    let from = from.unwrap_or_else(|| to - chrono::Duration::days(DEFAULT_STATS_DAYS));
    if from > to {
        bail!("Stats from must be before to");
    }
    Ok((start_of_day(from.date_naive())?, end_of_day(to.date_naive())?))
}

fn build_daily_stats(
    registrations: Vec<crate::entities::ReferralRegistrationModel>,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> ReferralStatsResponse {
    let mut counts = std::collections::BTreeMap::<NaiveDate, u64>::new();
    for registration in registrations {
        *counts
            .entry(registration.registered_at.date_naive())
            .or_insert(0) += 1;
    }

    let mut buckets = Vec::new();
    let mut date = from.date_naive();
    let end = to.date_naive();
    while date <= end {
        buckets.push(ReferralStatsBucket {
            date: date.to_string(),
            registrations: counts.get(&date).copied().unwrap_or(0),
        });
        date = date
            .succ_opt()
            .expect("next date should exist within supported range");
    }
    let total = buckets.iter().map(|bucket| bucket.registrations).sum();
    ReferralStatsResponse { total, buckets }
}

fn start_of_day(date: NaiveDate) -> Result<DateTime<Utc>> {
    Utc.with_ymd_and_hms(date.year(), date.month(), date.day(), 0, 0, 0)
        .single()
        .ok_or_else(|| anyhow!("Invalid stats start date"))
}

fn end_of_day(date: NaiveDate) -> Result<DateTime<Utc>> {
    Utc.with_ymd_and_hms(date.year(), date.month(), date.day(), 23, 59, 59)
        .single()
        .ok_or_else(|| anyhow!("Invalid stats end date"))
}
