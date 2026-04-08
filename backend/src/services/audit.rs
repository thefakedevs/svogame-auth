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
pub const ACTION_USER_SQUAD_CREATED: &str = "user.squad.created";
pub const ACTION_USER_SQUAD_DISBANDED: &str = "user.squad.disbanded";
pub const ACTION_USER_SQUAD_LEFT: &str = "user.squad.left";
pub const ACTION_USER_SQUAD_KICKED: &str = "user.squad.kicked";
pub const ACTION_USER_SQUAD_INVITE_CREATED: &str = "user.squad.invite_created";
pub const ACTION_USER_SQUAD_INVITE_ACCEPTED: &str = "user.squad.invite_accepted";
pub const ACTION_USER_SQUAD_INVITE_DECLINED: &str = "user.squad.invite_declined";
pub const ACTION_USER_SQUAD_INVITE_REVOKED: &str = "user.squad.invite_revoked";
pub const ACTION_USER_SQUAD_UPDATED: &str = "user.squad.updated";
pub const ACTION_USER_SQUAD_IMAGE_UPDATED: &str = "user.squad.image_updated";
pub const ACTION_USER_SQUAD_IMAGE_DELETED: &str = "user.squad.image_deleted";
pub const ACTION_ADMIN_SQUAD_UPDATED: &str = "admin.squad.updated";
pub const ACTION_ADMIN_SQUAD_RESTRICTED: &str = "admin.squad.restricted";
pub const ACTION_ADMIN_SQUAD_UNRESTRICTED: &str = "admin.squad.unrestricted";
pub const ACTION_ADMIN_SQUAD_DELETED: &str = "admin.squad.deleted";
pub const ACTION_ADMIN_SQUAD_MEMBER_KICKED: &str = "admin.squad.member_kicked";
pub const ACTION_ADMIN_SQUAD_IMAGE_UPDATED: &str = "admin.squad.image_updated";
pub const ACTION_ADMIN_SQUAD_IMAGE_DELETED: &str = "admin.squad.image_deleted";
pub const ACTION_ADMIN_USER_RESTRICTION_GRANTED: &str = "admin.user.restriction_granted";
pub const ACTION_ADMIN_USER_RESTRICTION_REVOKED: &str = "admin.user.restriction_revoked";
pub const ACTION_ADMIN_ASSET_CREATED: &str = "admin.asset.created";
pub const ACTION_ADMIN_ASSET_UPDATED: &str = "admin.asset.updated";
pub const ACTION_ADMIN_INVENTORY_ENTITLEMENT_GRANTED: &str = "admin.inventory.entitlement_granted";
pub const ACTION_ADMIN_INVENTORY_ENTITLEMENT_REVOKED: &str = "admin.inventory.entitlement_revoked";
pub const ACTION_ADMIN_INVENTORY_STACKABLE_ADDED: &str = "admin.inventory.stackable_added";
pub const ACTION_ADMIN_INVENTORY_STACKABLE_REMOVED: &str = "admin.inventory.stackable_removed";
pub const ACTION_ADMIN_INVENTORY_STACKABLE_SET: &str = "admin.inventory.stackable_set";
pub const ACTION_ADMIN_INVENTORY_EXPIRABLE_PROLONGED: &str = "admin.inventory.expirable.prolonged";
pub const ACTION_ADMIN_INVENTORY_EXPIRABLE_EXPIRATION_SET: &str = "admin.inventory.expirable.expiration_set";
pub const ACTION_ADMIN_INVENTORY_EXPIRABLE_REVOKED: &str = "admin.inventory.expirable.revoked";
pub const ACTION_ADMIN_WALLET_CREDITED: &str = "admin.wallet.credited";
pub const ACTION_ADMIN_WALLET_DEBITED: &str = "admin.wallet.debited";
pub const ACTION_ADMIN_WALLET_ADJUSTED: &str = "admin.wallet.adjusted";
pub const ACTION_ADMIN_DEFAULT_SKIN_UPDATED: &str = "admin.default_skin.updated";

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
