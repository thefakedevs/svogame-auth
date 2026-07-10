use chrono::{Duration, Utc};
use sea_orm::sea_query::Expr;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, Condition, DatabaseConnection, EntityTrait,
    QueryFilter, QueryOrder,
};

use crate::app::config::{DiscordConfig, MetricsConfig};
use crate::entities::{
    MetricDiscordOutbox, MetricDiscordOutboxActiveModel, MetricDiscordOutboxColumn,
};
use crate::services::discord::{DiscordApiError, send_channel_embeds_idempotent};

pub async fn process_next_delivery(
    db: &DatabaseConnection,
    discord_config: &DiscordConfig,
    metrics_config: &MetricsConfig,
) -> anyhow::Result<bool> {
    let (Some(bot_token), Some(channel_id)) = (
        discord_config.bot_token.as_deref(),
        metrics_config.discord_channel_id.as_deref(),
    ) else {
        return Ok(false);
    };
    let now = Utc::now();
    let Some(candidate) = MetricDiscordOutbox::find()
        .filter(
            Condition::any()
                .add(MetricDiscordOutboxColumn::Status.eq("pending"))
                .add(MetricDiscordOutboxColumn::Status.eq("retry"))
                .add(MetricDiscordOutboxColumn::Status.eq("processing")),
        )
        .filter(MetricDiscordOutboxColumn::NextAttemptAt.lte(now))
        .order_by_asc(MetricDiscordOutboxColumn::NextAttemptAt)
        .one(db)
        .await?
    else {
        return Ok(false);
    };

    let next_attempt = now + Duration::seconds(metrics_config.discord_retry_interval_seconds);
    let claim = MetricDiscordOutbox::update_many()
        .col_expr(MetricDiscordOutboxColumn::Status, Expr::value("processing"))
        .col_expr(
            MetricDiscordOutboxColumn::AttemptCount,
            Expr::col(MetricDiscordOutboxColumn::AttemptCount).add(1),
        )
        .col_expr(
            MetricDiscordOutboxColumn::LastAttemptAt,
            Expr::value(Some(now)),
        )
        .col_expr(
            MetricDiscordOutboxColumn::NextAttemptAt,
            Expr::value(next_attempt),
        )
        .col_expr(MetricDiscordOutboxColumn::UpdatedAt, Expr::value(now))
        .filter(MetricDiscordOutboxColumn::GameId.eq(candidate.game_id))
        .filter(MetricDiscordOutboxColumn::Status.eq(candidate.status))
        .exec(db)
        .await?;
    if claim.rows_affected == 0 {
        return Ok(false);
    }

    let Some(claimed) = MetricDiscordOutbox::find_by_id(candidate.game_id)
        .one(db)
        .await?
    else {
        return Ok(false);
    };
    let compact_id = claimed.game_id.as_simple().to_string();
    let nonce = &compact_id[..25];
    let embeds = serde_json::from_str(&claimed.description)
        .map_err(|error| anyhow::anyhow!("stored metric Discord payload is invalid: {error}"))?;
    let result =
        send_channel_embeds_idempotent(discord_config, bot_token, channel_id, embeds, nonce).await;
    match result {
        Ok(sent) => {
            let attempt_count = claimed.attempt_count;
            let mut active: MetricDiscordOutboxActiveModel = claimed.into();
            active.status = Set("delivered".to_string());
            active.last_error = Set(None);
            active.discord_channel_id = Set(Some(sent.channel_id));
            active.discord_message_id = Set(Some(sent.message_id));
            active.delivered_at = Set(Some(Utc::now()));
            active.updated_at = Set(Utc::now());
            active.update(db).await?;
            tracing::info!(
                action = "metrics.discord.delivered",
                game_id = %candidate.game_id,
                attempt = attempt_count,
                "Metric match summary delivered to Discord"
            );
        }
        Err(error) => {
            let error_message = truncate_error(discord_error_message(error));
            let mut active: MetricDiscordOutboxActiveModel = claimed.clone().into();
            active.status = Set(
                if claimed.attempt_count >= metrics_config.discord_max_attempts {
                    "failed".to_string()
                } else {
                    "retry".to_string()
                },
            );
            active.last_error = Set(Some(error_message.clone()));
            active.updated_at = Set(Utc::now());
            active.update(db).await?;
            tracing::warn!(
                action = "metrics.discord.failed",
                game_id = %candidate.game_id,
                attempt = claimed.attempt_count,
                error = %error_message,
                "Metric match summary delivery failed"
            );
        }
    }
    Ok(true)
}

fn discord_error_message(error: DiscordApiError) -> String {
    match error {
        DiscordApiError::Timeout(message)
        | DiscordApiError::Forbidden(message)
        | DiscordApiError::NotFound(message)
        | DiscordApiError::RateLimited(message)
        | DiscordApiError::Http(message)
        | DiscordApiError::Transport(message) => message,
    }
}

fn truncate_error(value: String) -> String {
    value.chars().take(1000).collect()
}
