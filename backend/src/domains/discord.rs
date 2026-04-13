use axum::Json;
use axum::extract::State;
use serde::Serialize;
use utoipa::ToSchema;

use crate::app::http::{HttpError, HttpResult, ProblemResponse};
use crate::app::state::AppStateExtractor;

#[derive(Debug, Serialize, ToSchema)]
pub struct DiscordEventResponse {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub status: String,
    #[serde(rename = "startsAt")]
    pub starts_at: chrono::DateTime<chrono::Utc>,
    #[serde(rename = "endsAt")]
    pub ends_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(rename = "imageUrl")]
    pub image_url: Option<String>,
    pub location: Option<String>,
    #[serde(rename = "userCount")]
    pub user_count: Option<u64>,
}

#[utoipa::path(
    get,
    path = "/api/discord/events",
    responses(
        (status = 200, description = "List scheduled Discord guild events for the public website.", body = [DiscordEventResponse]),
        (status = 503, description = "Discord events integration is unavailable.", body = ProblemResponse)
    ),
    tag = "discord"
)]
pub async fn list_events(State(state): AppStateExtractor) -> HttpResult<Json<Vec<DiscordEventResponse>>> {
    let (discord_runtime, discord_config) = {
        let state = state.read().await;
        (state.discord_events.clone(), state.config.discord.clone())
    };

    let events = discord_runtime
        .list_events(&discord_config)
        .await
        .map_err(map_events_error)?;

    Ok(Json(
        events
            .into_iter()
            .map(|event| DiscordEventResponse {
                id: event.id,
                name: event.name,
                description: event.description,
                status: event.status,
                starts_at: event.starts_at,
                ends_at: event.ends_at,
                image_url: event.image_url,
                location: event.location,
                user_count: event.user_count,
            })
            .collect(),
    ))
}

pub fn router() -> axum::Router<crate::app::state::SharedAppState> {
    axum::Router::new().route("/api/discord/events", axum::routing::get(list_events))
}

fn map_events_error(error: anyhow::Error) -> HttpError {
    let message = error.to_string();
    if let Some(stripped) = message.strip_prefix("unavailable: ") {
        HttpError::new(axum::http::StatusCode::SERVICE_UNAVAILABLE, stripped)
    } else {
        HttpError::internal_error(message)
    }
}
