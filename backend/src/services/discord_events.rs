use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Result, anyhow};
use tokio::sync::Mutex;

use crate::app::config::DiscordConfig;
use crate::services::discord;

const EVENTS_CACHE_TTL: Duration = Duration::from_secs(300);

#[derive(Clone, Debug)]
pub struct DiscordGuildEvent {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub status: String,
    pub starts_at: chrono::DateTime<chrono::Utc>,
    pub ends_at: Option<chrono::DateTime<chrono::Utc>>,
    pub image_url: Option<String>,
    pub location: Option<String>,
    pub user_count: Option<u64>,
}

#[derive(Clone, Default)]
pub struct DiscordEventsRuntime {
    cache: Arc<Mutex<DiscordEventsCache>>,
}

#[derive(Default)]
struct DiscordEventsCache {
    fetched_at: Option<Instant>,
    events: Vec<DiscordGuildEvent>,
}

impl DiscordEventsRuntime {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn list_events(&self, config: &DiscordConfig) -> Result<Vec<DiscordGuildEvent>> {
        let mut guard = self.cache.lock().await;

        if let Some(fetched_at) = guard.fetched_at
            && fetched_at.elapsed() < EVENTS_CACHE_TTL
        {
            return Ok(guard.events.clone());
        }

        let bot_token = config
            .bot_token
            .as_deref()
            .ok_or_else(|| anyhow!("unavailable: Discord bot is not configured"))?;
        let guild_id = config
            .events_guild_id
            .as_deref()
            .ok_or_else(|| anyhow!("unavailable: Discord events guild is not configured"))?;

        match discord::fetch_guild_scheduled_events(config, bot_token, guild_id).await {
            Ok(events) => {
                let mapped = events.into_iter().map(map_event).collect::<Vec<_>>();
                guard.events = mapped.clone();
                guard.fetched_at = Some(Instant::now());
                Ok(mapped)
            }
            Err(error) => {
                if !guard.events.is_empty() {
                    Ok(guard.events.clone())
                } else {
                    Err(anyhow!("unavailable: Failed to fetch Discord events: {error:?}"))
                }
            }
        }
    }
}

fn map_event(event: discord::DiscordScheduledEvent) -> DiscordGuildEvent {
    DiscordGuildEvent {
        id: event.id.clone(),
        name: event.name,
        description: event.description,
        status: map_status(event.status).to_string(),
        starts_at: event.scheduled_start_time,
        ends_at: event.scheduled_end_time,
        image_url: event.image.map(|hash| {
            format!(
                "https://cdn.discordapp.com/guild-events/{}/{}.png",
                event.id, hash
            )
        }),
        location: event.entity_metadata.and_then(|metadata| metadata.location),
        user_count: event.user_count,
    }
}

fn map_status(status: u8) -> &'static str {
    match status {
        1 => "scheduled",
        2 => "active",
        3 => "completed",
        4 => "canceled",
        _ => "unknown",
    }
}
