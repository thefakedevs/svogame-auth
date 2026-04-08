use anyhow::{Result, anyhow, bail};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::app::config::AppConfig;
use crate::entities::{
    ServiceToken, ServiceTokenActiveModel, ServiceTokenAudit, ServiceTokenAuditActiveModel,
    ServiceTokenAuditColumn, ServiceTokenColumn, ServiceTokenModel,
};
use crate::services::ownership::types::validate_asset_key;

pub const ACTION_SERVICE_TOKEN_CREATED: &str = "service_token.created";
pub const ACTION_SERVICE_TOKEN_ROTATED: &str = "service_token.rotated";
pub const ACTION_SERVICE_TOKEN_REVOKED: &str = "service_token.revoked";

const TOKEN_PREFIX_LEN: usize = 12;

#[derive(Clone, Debug)]
pub struct CreateServiceTokenInput {
    pub system_name: String,
    pub description: Option<String>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    pub actor_user_id: Uuid,
}

#[derive(Clone, Debug)]
pub struct RotateServiceTokenInput {
    pub token_id: Uuid,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    pub actor_user_id: Uuid,
    pub reason: Option<String>,
}

#[derive(Clone, Debug)]
pub struct RevokeServiceTokenInput {
    pub token_id: Uuid,
    pub actor_user_id: Uuid,
    pub reason: Option<String>,
}

#[derive(Clone, Debug)]
pub struct ServiceTokenSecret {
    pub model: ServiceTokenModel,
    pub plaintext_token: String,
}

#[derive(Clone, Debug)]
pub struct AuthenticatedServiceToken {
    pub id: Uuid,
    pub system_name: String,
}

pub async fn list_service_tokens(
    db: &sea_orm::DatabaseConnection,
) -> Result<Vec<ServiceTokenModel>> {
    ServiceToken::find()
        .order_by_asc(ServiceTokenColumn::SystemName)
        .all(db)
        .await
        .map_err(Into::into)
}

pub async fn get_service_token(
    db: &sea_orm::DatabaseConnection,
    token_id: Uuid,
) -> Result<ServiceTokenModel> {
    ServiceToken::find_by_id(token_id)
        .one(db)
        .await?
        .ok_or_else(|| anyhow!("Service token not found"))
}

pub async fn get_service_token_audit(
    db: &sea_orm::DatabaseConnection,
    token_id: Uuid,
) -> Result<Vec<crate::entities::ServiceTokenAuditModel>> {
    ServiceTokenAudit::find()
        .filter(ServiceTokenAuditColumn::ServiceTokenId.eq(token_id))
        .order_by_desc(ServiceTokenAuditColumn::CreatedAt)
        .all(db)
        .await
        .map_err(Into::into)
}

pub async fn create_service_token(
    db: &sea_orm::DatabaseConnection,
    config: &AppConfig,
    input: CreateServiceTokenInput,
) -> Result<ServiceTokenSecret> {
    let system_name = validate_system_name(&input.system_name)?;
    ensure_unique_system_name(db, &system_name, None).await?;

    let plaintext_token = generate_plaintext_token();
    let (token_prefix, secret) = parse_plaintext_token(&plaintext_token)?;
    let now = chrono::Utc::now();

    let model = ServiceTokenActiveModel {
        id: Set(Uuid::new_v4()),
        system_name: Set(system_name.clone()),
        token_prefix: Set(token_prefix.to_string()),
        token_hash: Set(hash_service_secret(secret, config)),
        is_active: Set(true),
        expires_at: Set(input.expires_at),
        last_used_at: Set(None),
        description: Set(normalize_optional_string(input.description)),
        created_by_user_id: Set(input.actor_user_id),
        rotated_from_id: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
    }
    .insert(db)
    .await?;

    write_service_token_audit(
        db,
        model.id,
        ACTION_SERVICE_TOKEN_CREATED,
        input.actor_user_id,
        None,
        Some(json!({ "systemName": model.system_name })),
    )
    .await?;

    Ok(ServiceTokenSecret {
        model,
        plaintext_token,
    })
}

pub async fn rotate_service_token(
    db: &sea_orm::DatabaseConnection,
    config: &AppConfig,
    input: RotateServiceTokenInput,
) -> Result<ServiceTokenSecret> {
    let existing = get_service_token(db, input.token_id).await?;
    if !existing.is_active {
        bail!("Service token is already inactive");
    }

    let now = chrono::Utc::now();
    let mut old_active: ServiceTokenActiveModel = existing.clone().into();
    old_active.is_active = Set(false);
    old_active.updated_at = Set(now);
    old_active.update(db).await?;

    let plaintext_token = generate_plaintext_token();
    let (token_prefix, secret) = parse_plaintext_token(&plaintext_token)?;
    let new_model = ServiceTokenActiveModel {
        id: Set(Uuid::new_v4()),
        system_name: Set(existing.system_name.clone()),
        token_prefix: Set(token_prefix.to_string()),
        token_hash: Set(hash_service_secret(secret, config)),
        is_active: Set(true),
        expires_at: Set(input.expires_at.or(existing.expires_at)),
        last_used_at: Set(None),
        description: Set(existing.description.clone()),
        created_by_user_id: Set(input.actor_user_id),
        rotated_from_id: Set(Some(existing.id)),
        created_at: Set(now),
        updated_at: Set(now),
    }
    .insert(db)
    .await?;

    write_service_token_audit(
        db,
        existing.id,
        ACTION_SERVICE_TOKEN_ROTATED,
        input.actor_user_id,
        input.reason,
        Some(json!({ "replacedByTokenId": new_model.id })),
    )
    .await?;
    write_service_token_audit(
        db,
        new_model.id,
        ACTION_SERVICE_TOKEN_CREATED,
        input.actor_user_id,
        None,
        Some(json!({ "rotatedFromTokenId": existing.id, "systemName": new_model.system_name })),
    )
    .await?;

    Ok(ServiceTokenSecret {
        model: new_model,
        plaintext_token,
    })
}

pub async fn revoke_service_token(
    db: &sea_orm::DatabaseConnection,
    input: RevokeServiceTokenInput,
) -> Result<ServiceTokenModel> {
    let existing = get_service_token(db, input.token_id).await?;
    if !existing.is_active {
        return Ok(existing);
    }

    let mut active: ServiceTokenActiveModel = existing.into();
    active.is_active = Set(false);
    active.updated_at = Set(chrono::Utc::now());
    let updated = active.update(db).await?;

    write_service_token_audit(
        db,
        updated.id,
        ACTION_SERVICE_TOKEN_REVOKED,
        input.actor_user_id,
        input.reason,
        None,
    )
    .await?;

    Ok(updated)
}

pub async fn authenticate_service_token(
    db: &sea_orm::DatabaseConnection,
    config: &AppConfig,
    token: &str,
) -> Result<AuthenticatedServiceToken> {
    let (token_prefix, secret) = parse_plaintext_token(token)?;
    let model = ServiceToken::find()
        .filter(ServiceTokenColumn::TokenPrefix.eq(token_prefix))
        .one(db)
        .await?
        .ok_or_else(|| anyhow!("Service token not found"))?;

    if !model.is_active {
        bail!("Service token is inactive");
    }
    if let Some(expires_at) = model.expires_at {
        if expires_at <= chrono::Utc::now() {
            bail!("Service token expired");
        }
    }

    let expected_hash = hash_service_secret(secret, config);
    if expected_hash != model.token_hash {
        bail!("Service token invalid");
    }

    let mut active: ServiceTokenActiveModel = model.clone().into();
    active.last_used_at = Set(Some(chrono::Utc::now()));
    active.updated_at = Set(chrono::Utc::now());
    let _ = active.update(db).await;

    Ok(AuthenticatedServiceToken {
        id: model.id,
        system_name: model.system_name,
    })
}

pub fn validate_system_name(value: &str) -> Result<String> {
    validate_asset_key(value)
}

fn hash_service_secret(secret: &str, config: &AppConfig) -> String {
    let mut hasher = Sha256::new();
    hasher.update(config.jwt_secret.as_bytes());
    hasher.update(b":service_token:");
    hasher.update(secret.as_bytes());
    let digest = hasher.finalize();
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn generate_plaintext_token() -> String {
    let prefix = Uuid::new_v4().simple().to_string()[..TOKEN_PREFIX_LEN].to_string();
    let secret = format!(
        "{}{}{}",
        Uuid::new_v4().simple(),
        Uuid::new_v4().simple(),
        Uuid::new_v4().simple()
    );
    format!("svcs_{prefix}_{secret}")
}

fn parse_plaintext_token(token: &str) -> Result<(&str, &str)> {
    let mut parts = token.split('_');
    let kind = parts.next().ok_or_else(|| anyhow!("Invalid service token"))?;
    let prefix = parts.next().ok_or_else(|| anyhow!("Invalid service token"))?;
    let secret = parts.next().ok_or_else(|| anyhow!("Invalid service token"))?;
    if kind != "svcs" || prefix.len() != TOKEN_PREFIX_LEN || secret.trim().is_empty() || parts.next().is_some() {
        bail!("Invalid service token");
    }
    Ok((prefix, secret))
}

async fn ensure_unique_system_name(
    db: &sea_orm::DatabaseConnection,
    system_name: &str,
    exclude_token_id: Option<Uuid>,
) -> Result<()> {
    let existing = ServiceToken::find()
        .filter(ServiceTokenColumn::SystemName.eq(system_name))
        .one(db)
        .await?;

    if let Some(existing) = existing {
        if Some(existing.id) != exclude_token_id {
            bail!("System name is already taken");
        }
    }
    Ok(())
}

async fn write_service_token_audit(
    db: &sea_orm::DatabaseConnection,
    service_token_id: Uuid,
    action: &str,
    actor_user_id: Uuid,
    reason: Option<String>,
    metadata: Option<Value>,
) -> Result<()> {
    ServiceTokenAuditActiveModel {
        id: sea_orm::ActiveValue::NotSet,
        service_token_id: Set(service_token_id),
        action: Set(action.to_string()),
        actor_user_id: Set(actor_user_id),
        reason: Set(reason),
        metadata: Set(metadata.map(|value| value.to_string())),
        created_at: Set(chrono::Utc::now()),
    }
    .insert(db)
    .await?;
    Ok(())
}

fn normalize_optional_string(value: Option<String>) -> Option<String> {
    value.and_then(|text| {
        let trimmed = text.trim().to_string();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    })
}
