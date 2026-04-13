use std::time::Duration;

use anyhow::{Result, anyhow};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, DatabaseConnection,
    EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
};
use serde::Serialize;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::app::config::DiscordConfig;
use crate::entities::{
    DiscordBroadcast, DiscordBroadcastActiveModel, DiscordBroadcastColumn, DiscordBroadcastModel,
    DiscordDelivery, DiscordDeliveryActiveModel, DiscordDeliveryColumn, DiscordDeliveryModel, User,
};
use crate::services::discord;
use crate::services::discord_templates;

pub const DELIVERY_STATUS_PENDING: &str = "pending";
pub const DELIVERY_STATUS_PROCESSING: &str = "processing";
pub const DELIVERY_STATUS_DELIVERED: &str = "delivered";
pub const DELIVERY_STATUS_TIMEOUT: &str = "timeout";
pub const DELIVERY_STATUS_FORBIDDEN: &str = "forbidden";
pub const DELIVERY_STATUS_ERROR: &str = "error";

pub const BROADCAST_STATUS_PENDING: &str = "pending";
pub const BROADCAST_STATUS_RUNNING: &str = "running";
pub const BROADCAST_STATUS_COMPLETED: &str = "completed";

const TEMPLATE_ADMIN_CUSTOM: &str = "admin.custom";
const TEMPLATE_SQUAD_INVITE_RECEIVED: &str = "squad.invite_received";
const TEMPLATE_SQUAD_KICKED: &str = "squad.kicked";
const TEMPLATE_SHOP_PURCHASE_COMPLETED: &str = "shop.purchase_completed";

#[derive(Clone, Debug, Serialize)]
pub struct BroadcastProgress {
    pub delivered_count: u64,
    pub timeout_count: u64,
    pub forbidden_count: u64,
    pub error_count: u64,
}

pub async fn create_single_delivery(
    db: &impl ConnectionTrait,
    requested_by_user_id: Option<Uuid>,
    user_id: Uuid,
    message: String,
) -> Result<DiscordDeliveryModel> {
    insert_delivery(
        db,
        NewDelivery {
            broadcast_id: None,
            requested_by_user_id,
            user_id,
            template_key: TEMPLATE_ADMIN_CUSTOM.to_string(),
            message,
            metadata: json!({ "source": "admin.notify" }),
        },
    )
    .await
}

pub async fn create_broadcast(
    db: &DatabaseConnection,
    requested_by_user_id: Uuid,
    message: String,
) -> Result<DiscordBroadcastModel> {
    let users = User::find()
        .filter(crate::entities::UserColumn::IsActive.eq(true))
        .order_by_asc(crate::entities::UserColumn::CreatedAt)
        .all(db)
        .await?;
    let now = chrono::Utc::now();
    let broadcast = DiscordBroadcastActiveModel {
        id: Set(Uuid::new_v4()),
        requested_by_user_id: Set(Some(requested_by_user_id)),
        template_key: Set(TEMPLATE_ADMIN_CUSTOM.to_string()),
        message: Set(message.clone()),
        status: Set(if users.is_empty() {
            BROADCAST_STATUS_COMPLETED.to_string()
        } else {
            BROADCAST_STATUS_PENDING.to_string()
        }),
        total_count: Set(users.len() as i64),
        created_at: Set(now),
        updated_at: Set(now),
        started_at: Set(None),
        finished_at: Set(if users.is_empty() { Some(now) } else { None }),
    }
    .insert(db)
    .await?;

    for user in users {
        insert_delivery(
            db,
            NewDelivery {
                broadcast_id: Some(broadcast.id),
                requested_by_user_id: Some(requested_by_user_id),
                user_id: user.id,
                template_key: TEMPLATE_ADMIN_CUSTOM.to_string(),
                message: message.clone(),
                metadata: json!({
                    "source": "admin.broadcast",
                    "broadcastId": broadcast.id
                }),
            },
        )
        .await?;
    }

    Ok(broadcast)
}

pub async fn queue_squad_invite_notification(
    db: &impl ConnectionTrait,
    invited_user_id: Uuid,
    squad_id: Uuid,
    invite_id: Uuid,
    squad_name: &str,
    inviter_username: &str,
) -> Result<()> {
    let message = discord_templates::squad_invite_received(squad_name, inviter_username);
    insert_delivery(
        db,
        NewDelivery {
            broadcast_id: None,
            requested_by_user_id: None,
            user_id: invited_user_id,
            template_key: TEMPLATE_SQUAD_INVITE_RECEIVED.to_string(),
            message: message.description,
            metadata: json!({
                "source": "squads.create_invite",
                "squadId": squad_id,
                "squadName": squad_name,
                "inviteId": invite_id,
                "inviterUsername": inviter_username
            }),
        },
    )
    .await?;
    Ok(())
}

pub async fn queue_squad_kicked_notification(
    db: &impl ConnectionTrait,
    target_user_id: Uuid,
    squad_id: Uuid,
    squad_name: &str,
    leader_username: &str,
) -> Result<()> {
    let message = discord_templates::squad_kicked(squad_name, leader_username);
    insert_delivery(
        db,
        NewDelivery {
            broadcast_id: None,
            requested_by_user_id: None,
            user_id: target_user_id,
            template_key: TEMPLATE_SQUAD_KICKED.to_string(),
            message: message.description,
            metadata: json!({
                "source": "squads.kick_member",
                "squadId": squad_id,
                "squadName": squad_name,
                "leaderUsername": leader_username
            }),
        },
    )
    .await?;
    Ok(())
}

pub async fn queue_shop_purchase_completed_notification(
    db: &impl ConnectionTrait,
    user_id: Uuid,
    order_id: Uuid,
    product_key: &str,
    product_name: &str,
    quantity: i64,
) -> Result<()> {
    let message = discord_templates::shop_purchase_completed(product_name, quantity);
    insert_delivery(
        db,
        NewDelivery {
            broadcast_id: None,
            requested_by_user_id: Some(user_id),
            user_id,
            template_key: TEMPLATE_SHOP_PURCHASE_COMPLETED.to_string(),
            message: message.description,
            metadata: json!({
                "source": "shop.fulfill_order",
                "orderId": order_id,
                "productKey": product_key,
                "productName": product_name,
                "quantity": quantity
            }),
        },
    )
    .await?;
    Ok(())
}

pub async fn list_broadcasts(db: &DatabaseConnection) -> Result<Vec<DiscordBroadcastModel>> {
    DiscordBroadcast::find()
        .order_by_desc(DiscordBroadcastColumn::CreatedAt)
        .all(db)
        .await
        .map_err(Into::into)
}

pub async fn get_broadcast(db: &DatabaseConnection, broadcast_id: Uuid) -> Result<DiscordBroadcastModel> {
    DiscordBroadcast::find_by_id(broadcast_id)
        .one(db)
        .await?
        .ok_or_else(|| anyhow!("not_found: Broadcast not found"))
}

pub async fn get_broadcast_progress(
    db: &DatabaseConnection,
    broadcast_id: Uuid,
) -> Result<BroadcastProgress> {
    Ok(BroadcastProgress {
        delivered_count: count_deliveries(db, broadcast_id, DELIVERY_STATUS_DELIVERED).await?,
        timeout_count: count_deliveries(db, broadcast_id, DELIVERY_STATUS_TIMEOUT).await?,
        forbidden_count: count_deliveries(db, broadcast_id, DELIVERY_STATUS_FORBIDDEN).await?,
        error_count: count_deliveries(db, broadcast_id, DELIVERY_STATUS_ERROR).await?,
    })
}

pub async fn process_next_delivery(
    db: &DatabaseConnection,
    discord_config: &DiscordConfig,
) -> Result<bool> {
    if let Some(delivery) = next_pending_delivery(db, Some(false)).await? {
        attempt_delivery(db, discord_config, delivery, false).await?;
        return Ok(true);
    }

    if let Some(delivery) = next_pending_delivery(db, Some(true)).await? {
        attempt_delivery(db, discord_config, delivery, true).await?;
        tokio::time::sleep(Duration::from_millis(500)).await;
        return Ok(true);
    }

    Ok(false)
}

pub async fn send_delivery_now(
    db: &DatabaseConnection,
    discord_config: &DiscordConfig,
    delivery_id: Uuid,
) -> Result<DiscordDeliveryModel> {
    let delivery = DiscordDelivery::find_by_id(delivery_id)
        .one(db)
        .await?
        .ok_or_else(|| anyhow!("not_found: Delivery not found"))?;
    attempt_delivery(db, discord_config, delivery, false).await
}

async fn next_pending_delivery(
    db: &DatabaseConnection,
    is_broadcast: Option<bool>,
) -> Result<Option<DiscordDeliveryModel>> {
    let mut query = DiscordDelivery::find()
        .filter(DiscordDeliveryColumn::Status.eq(DELIVERY_STATUS_PENDING))
        .order_by_asc(DiscordDeliveryColumn::CreatedAt);

    query = match is_broadcast {
        Some(true) => query.filter(DiscordDeliveryColumn::BroadcastId.is_not_null()),
        Some(false) => query.filter(DiscordDeliveryColumn::BroadcastId.is_null()),
        None => query,
    };

    query.one(db).await.map_err(Into::into)
}

async fn attempt_delivery(
    db: &DatabaseConnection,
    discord_config: &DiscordConfig,
    delivery: DiscordDeliveryModel,
    is_broadcast: bool,
) -> Result<DiscordDeliveryModel> {
    let processing = mark_processing(db, &delivery).await?;
    if is_broadcast
        && let Some(broadcast_id) = processing.broadcast_id
    {
        mark_broadcast_running_if_needed(db, broadcast_id).await?;
    }

    let user = User::find_by_id(processing.user_id)
        .one(db)
        .await?
        .ok_or_else(|| anyhow!("not_found: Delivery target user not found"))?;

    let outcome = if let Some(bot_token) = discord_config.bot_token.as_deref() {
        let metadata = serde_json::from_str::<Value>(&processing.metadata).unwrap_or_else(|_| json!({}));
        let template =
            discord_templates::render(&processing.template_key, &processing.message, &metadata);
        send_direct_message(
            discord_config,
            bot_token,
            &user.discord_id,
            &template,
        )
        .await
    } else {
        Err(discord::DiscordApiError::Http(
            "Discord bot is not configured".to_string(),
        ))
    };

    let updated = match outcome {
        Ok(result) => mark_delivered(db, &processing, result.channel_id, result.message_id).await?,
        Err(error) => {
            let (status, description) = map_error_to_status(error);
            mark_failed(db, &processing, status, description).await?
        }
    };

    if let Some(broadcast_id) = updated.broadcast_id {
        complete_broadcast_if_finished(db, broadcast_id).await?;
    }

    Ok(updated)
}

async fn send_direct_message(
    discord_config: &DiscordConfig,
    bot_token: &str,
    recipient_discord_id: &str,
    template: &discord_templates::DiscordTemplate,
) -> std::result::Result<discord::DiscordBotMessageResult, discord::DiscordApiError> {
    let channel_id =
        discord::create_direct_message_channel(discord_config, bot_token, recipient_discord_id)
            .await?;
    discord::send_channel_embed(
        discord_config,
        bot_token,
        &channel_id,
        discord::DiscordEmbed {
            title: &template.title,
            description: &template.description,
            url: &template.url,
            color: 0x4A90E2,
            footer: Some(discord::DiscordEmbedFooter {
                text: &template.footer_text,
            }),
        },
    )
    .await
}

fn map_error_to_status(error: discord::DiscordApiError) -> (&'static str, String) {
    match error {
        discord::DiscordApiError::Timeout(message) => (DELIVERY_STATUS_TIMEOUT, message),
        discord::DiscordApiError::Forbidden(message) => (DELIVERY_STATUS_FORBIDDEN, message),
        discord::DiscordApiError::NotFound(message)
        | discord::DiscordApiError::RateLimited(message)
        | discord::DiscordApiError::Http(message)
        | discord::DiscordApiError::Transport(message) => (DELIVERY_STATUS_ERROR, message),
    }
}

async fn insert_delivery(
    db: &impl ConnectionTrait,
    input: NewDelivery,
) -> Result<DiscordDeliveryModel> {
    let now = chrono::Utc::now();
    DiscordDeliveryActiveModel {
        id: Set(Uuid::new_v4()),
        broadcast_id: Set(input.broadcast_id),
        user_id: Set(input.user_id),
        requested_by_user_id: Set(input.requested_by_user_id),
        template_key: Set(input.template_key),
        message: Set(input.message),
        status: Set(DELIVERY_STATUS_PENDING.to_string()),
        error_message: Set(None),
        discord_channel_id: Set(None),
        discord_message_id: Set(None),
        attempt_count: Set(0),
        metadata: Set(input.metadata.to_string()),
        created_at: Set(now),
        updated_at: Set(now),
        last_attempt_at: Set(None),
        delivered_at: Set(None),
        finished_at: Set(None),
    }
    .insert(db)
    .await
    .map_err(Into::into)
}

async fn mark_processing(
    db: &DatabaseConnection,
    delivery: &DiscordDeliveryModel,
) -> Result<DiscordDeliveryModel> {
    let now = chrono::Utc::now();
    let mut active: DiscordDeliveryActiveModel = delivery.clone().into();
    active.status = Set(DELIVERY_STATUS_PROCESSING.to_string());
    active.attempt_count = Set(delivery.attempt_count + 1);
    active.last_attempt_at = Set(Some(now));
    active.updated_at = Set(now);
    active.error_message = Set(None);
    active.update(db).await.map_err(Into::into)
}

async fn mark_delivered(
    db: &DatabaseConnection,
    delivery: &DiscordDeliveryModel,
    channel_id: String,
    message_id: String,
) -> Result<DiscordDeliveryModel> {
    let now = chrono::Utc::now();
    let mut active: DiscordDeliveryActiveModel = delivery.clone().into();
    active.status = Set(DELIVERY_STATUS_DELIVERED.to_string());
    active.error_message = Set(None);
    active.discord_channel_id = Set(Some(channel_id));
    active.discord_message_id = Set(Some(message_id));
    active.delivered_at = Set(Some(now));
    active.finished_at = Set(Some(now));
    active.updated_at = Set(now);
    active.update(db).await.map_err(Into::into)
}

async fn mark_failed(
    db: &DatabaseConnection,
    delivery: &DiscordDeliveryModel,
    status: &str,
    error_message: String,
) -> Result<DiscordDeliveryModel> {
    let now = chrono::Utc::now();
    let mut active: DiscordDeliveryActiveModel = delivery.clone().into();
    active.status = Set(status.to_string());
    active.error_message = Set(Some(error_message));
    active.finished_at = Set(Some(now));
    active.updated_at = Set(now);
    active.update(db).await.map_err(Into::into)
}

async fn mark_broadcast_running_if_needed(db: &DatabaseConnection, broadcast_id: Uuid) -> Result<()> {
    let Some(broadcast) = DiscordBroadcast::find_by_id(broadcast_id).one(db).await? else {
        return Ok(());
    };
    if broadcast.status != BROADCAST_STATUS_PENDING {
        return Ok(());
    }
    let now = chrono::Utc::now();
    let mut active: DiscordBroadcastActiveModel = broadcast.into();
    active.status = Set(BROADCAST_STATUS_RUNNING.to_string());
    active.started_at = Set(Some(now));
    active.updated_at = Set(now);
    active.update(db).await?;
    Ok(())
}

async fn complete_broadcast_if_finished(db: &DatabaseConnection, broadcast_id: Uuid) -> Result<()> {
    let Some(broadcast) = DiscordBroadcast::find_by_id(broadcast_id).one(db).await? else {
        return Ok(());
    };
    if broadcast.status == BROADCAST_STATUS_COMPLETED {
        return Ok(());
    }

    let remaining = DiscordDelivery::find()
        .filter(DiscordDeliveryColumn::BroadcastId.eq(broadcast_id))
        .filter(
            DiscordDeliveryColumn::Status
                .is_in([DELIVERY_STATUS_PENDING, DELIVERY_STATUS_PROCESSING]),
        )
        .count(db)
        .await?;
    if remaining > 0 {
        return Ok(());
    }

    let now = chrono::Utc::now();
    let mut active: DiscordBroadcastActiveModel = broadcast.into();
    active.status = Set(BROADCAST_STATUS_COMPLETED.to_string());
    active.finished_at = Set(Some(now));
    active.updated_at = Set(now);
    active.update(db).await?;
    Ok(())
}

async fn count_deliveries(
    db: &DatabaseConnection,
    broadcast_id: Uuid,
    status: &str,
) -> Result<u64> {
    DiscordDelivery::find()
        .filter(DiscordDeliveryColumn::BroadcastId.eq(broadcast_id))
        .filter(DiscordDeliveryColumn::Status.eq(status))
        .count(db)
        .await
        .map_err(Into::into)
}

struct NewDelivery {
    broadcast_id: Option<Uuid>,
    requested_by_user_id: Option<Uuid>,
    user_id: Uuid,
    template_key: String,
    message: String,
    metadata: Value,
}
