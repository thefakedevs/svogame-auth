use anyhow::{Result, anyhow, bail};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;
use uuid::Uuid;

pub const ASSET_KEY_REGEX: &str = r"^[a-z0-9_-]+$";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AssetKind {
    Item,
    Skin,
    Subscription,
    Cosmetic,
    Lootbox,
    Currency,
    Ticket,
    Token,
    Kit,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SkinRarity {
    Common,
    Rare,
    Legendary,
}

impl SkinRarity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Common => "common",
            Self::Rare => "rare",
            Self::Legendary => "legendary",
        }
    }

    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "common" => Ok(Self::Common),
            "rare" => Ok(Self::Rare),
            "legendary" => Ok(Self::Legendary),
            _ => bail!("Unsupported skin rarity"),
        }
    }
}

impl AssetKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Item => "item",
            Self::Skin => "skin",
            Self::Subscription => "subscription",
            Self::Cosmetic => "cosmetic",
            Self::Lootbox => "lootbox",
            Self::Currency => "currency",
            Self::Ticket => "ticket",
            Self::Token => "token",
            Self::Kit => "kit",
        }
    }

    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "item" => Ok(Self::Item),
            "skin" => Ok(Self::Skin),
            "subscription" => Ok(Self::Subscription),
            "cosmetic" => Ok(Self::Cosmetic),
            "lootbox" => Ok(Self::Lootbox),
            "currency" => Ok(Self::Currency),
            "ticket" => Ok(Self::Ticket),
            "token" => Ok(Self::Token),
            "kit" => Ok(Self::Kit),
            _ => bail!("Unsupported asset kind"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum OwnershipModel {
    Stackable,
    Entitlement,
    Expirable,
}

impl OwnershipModel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Stackable => "stackable",
            Self::Entitlement => "entitlement",
            Self::Expirable => "expirable",
        }
    }

    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "stackable" => Ok(Self::Stackable),
            "entitlement" => Ok(Self::Entitlement),
            "expirable" => Ok(Self::Expirable),
            _ => bail!("Unsupported ownership model"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OwnershipActorKind {
    System,
    User,
    Admin,
}

impl OwnershipActorKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::System => "system",
            Self::User => "user",
            Self::Admin => "admin",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OwnershipActor {
    pub kind: OwnershipActorKind,
    #[serde(rename = "userId")]
    pub user_id: Option<Uuid>,
    #[serde(rename = "serviceName")]
    pub service_name: Option<String>,
}

impl OwnershipActor {
    pub fn admin(user_id: Uuid) -> Self {
        Self {
            kind: OwnershipActorKind::Admin,
            user_id: Some(user_id),
            service_name: None,
        }
    }

    pub fn user(user_id: Uuid) -> Self {
        Self {
            kind: OwnershipActorKind::User,
            user_id: Some(user_id),
            service_name: None,
        }
    }

    pub fn system(service_name: impl Into<String>) -> Self {
        Self {
            kind: OwnershipActorKind::System,
            user_id: None,
            service_name: Some(service_name.into()),
        }
    }

    pub fn validate(&self) -> Result<()> {
        match self.kind {
            OwnershipActorKind::System => {
                if self
                    .service_name
                    .as_deref()
                    .unwrap_or_default()
                    .trim()
                    .is_empty()
                {
                    bail!("System actor must include service name");
                }
            }
            OwnershipActorKind::User | OwnershipActorKind::Admin => {
                if self.user_id.is_none() {
                    bail!("User actor must include user id");
                }
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OperationContext {
    #[serde(rename = "reasonCode")]
    pub reason_code: Option<String>,
    #[serde(rename = "reasonText")]
    pub reason_text: Option<String>,
    #[serde(default)]
    pub metadata: Value,
}

impl Default for OperationContext {
    fn default() -> Self {
        Self {
            reason_code: None,
            reason_text: None,
            metadata: Value::Object(Default::default()),
        }
    }
}

pub fn validate_asset_key(key: &str) -> Result<String> {
    let normalized = key.trim();
    if normalized.is_empty() {
        bail!("Asset key cannot be empty");
    }
    if normalized != normalized.to_lowercase() {
        bail!("Asset key must be lowercase");
    }

    let regex = Regex::new(ASSET_KEY_REGEX).map_err(|error| anyhow!(error))?;
    if !regex.is_match(normalized) {
        bail!(
            "Asset key must contain only lowercase latin letters, numbers, underscores, and hyphens"
        );
    }

    Ok(normalized.to_string())
}

pub fn normalize_metadata(metadata: Option<Value>) -> Value {
    match metadata {
        Some(Value::Null) | None => Value::Object(Default::default()),
        Some(value) => value,
    }
}

pub fn validate_weapon_key(value: &str) -> Result<String> {
    let normalized = value.trim();
    if normalized.is_empty() {
        bail!("Weapon key cannot be empty");
    }
    Ok(normalized.to_string())
}
