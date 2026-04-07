use anyhow::Result;
use sea_orm::{ActiveModelTrait, ActiveValue, ConnectionTrait};
use serde_json::Value;
use uuid::Uuid;

use crate::entities::AuditLogActiveModel;

pub const ACTION_USER_REGISTERED: &str = "user.registered";
pub const ACTION_ADMIN_USER_UPDATED: &str = "admin.user.updated";
pub const ACTION_ADMIN_USER_DEACTIVATED: &str = "admin.user.deactivated";
pub const ACTION_ADMIN_USER_ACTIVATED: &str = "admin.user.activated";
pub const ACTION_ADMIN_USER_AUTH_EPOCH_RESET: &str = "admin.user.auth_epoch_reset";
pub const ACTION_ADMIN_USER_SUPERUSER_GRANTED: &str = "admin.user.superuser_granted";
pub const ACTION_ADMIN_USER_SUPERUSER_REVOKED: &str = "admin.user.superuser_revoked";

pub async fn write_audit_log<C>(
    db: &C,
    action: &str,
    actor_user_id: Option<Uuid>,
    target_user_id: Option<Uuid>,
    reason: Option<String>,
    metadata: Option<Value>,
) -> Result<()>
where
    C: ConnectionTrait,
{
    let record = AuditLogActiveModel {
        id: ActiveValue::NotSet,
        action: ActiveValue::Set(action.to_string()),
        actor_user_id: ActiveValue::Set(actor_user_id),
        target_user_id: ActiveValue::Set(target_user_id),
        reason: ActiveValue::Set(reason),
        metadata: ActiveValue::Set(metadata.map(|it| it.to_string())),
        created_at: ActiveValue::Set(chrono::Utc::now()),
    };

    record.insert(db).await?;
    Ok(())
}
