use crate::app::config::DiscordConfig;
use anyhow::Result;
use reqwest::{
    Client,
    header::{AUTHORIZATION, CONTENT_TYPE},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Deserialize, Debug)]
pub struct DiscordTokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: u64,
    pub refresh_token: String,
    pub scope: String,
}

#[derive(Deserialize, Debug)]
pub struct DiscordUserResponse {
    pub id: String,
    pub username: String,
    pub global_name: Option<String>,
    pub avatar: Option<String>,
    pub email: Option<String>,
    pub verified: Option<bool>,
}

#[derive(Deserialize, Debug)]
pub struct DiscordScheduledEventEntityMetadata {
    pub location: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct DiscordScheduledEvent {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub status: u8,
    pub image: Option<String>,
    pub entity_metadata: Option<DiscordScheduledEventEntityMetadata>,
    pub scheduled_start_time: chrono::DateTime<chrono::Utc>,
    pub scheduled_end_time: Option<chrono::DateTime<chrono::Utc>>,
    pub user_count: Option<u64>,
}

#[derive(Serialize)]
struct CreateDmChannelRequest<'a> {
    recipient_id: &'a str,
}

#[derive(Deserialize)]
struct CreateDmChannelResponse {
    id: String,
}

#[derive(Serialize)]
struct SendMessageRequest<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<&'a str>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    embeds: Vec<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    nonce: Option<&'a str>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    enforce_nonce: bool,
}

#[derive(Serialize)]
pub struct DiscordEmbed<'a> {
    pub title: &'a str,
    pub description: &'a str,
    pub url: &'a str,
    pub color: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub footer: Option<DiscordEmbedFooter<'a>>,
}

#[derive(Serialize)]
pub struct DiscordEmbedFooter<'a> {
    pub text: &'a str,
}

#[derive(Deserialize)]
struct SendMessageResponse {
    id: String,
}

#[derive(Debug)]
pub struct DiscordBotMessageResult {
    pub channel_id: String,
    pub message_id: String,
}

#[derive(Debug)]
pub enum DiscordApiError {
    Timeout(String),
    Forbidden(String),
    NotFound(String),
    RateLimited(String),
    Http(String),
    Transport(String),
}

impl DiscordTokenResponse {
    pub fn build_avatar_url(&self, user: &DiscordUserResponse) -> Option<String> {
        user.avatar.as_ref().map(|avatar_hash| {
            format!(
                "https://cdn.discordapp.com/avatars/{}/{}.png",
                user.id, avatar_hash
            )
        })
    }
}

pub async fn exchange_code(config: &DiscordConfig, code: &str) -> Result<DiscordTokenResponse> {
    let client = build_client(config)?;

    let params = [
        ("grant_type", "authorization_code"),
        ("code", code),
        ("redirect_uri", &config.redirect_url),
    ];

    let res = client
        .post(format!("{}/oauth2/token", config.api_base_url))
        .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
        .basic_auth(&config.client_id, Some(&config.client_secret))
        .form(&params)
        .send()
        .await?
        .error_for_status()?;

    let json = res.json::<DiscordTokenResponse>().await?;
    Ok(json)
}

pub async fn get_user_info(
    config: &DiscordConfig,
    access_token: &str,
) -> Result<DiscordUserResponse> {
    let client = build_client(config)?;

    let res = client
        .get(format!("{}/users/@me", config.api_base_url))
        .bearer_auth(access_token)
        .send()
        .await?
        .error_for_status()?;

    let json = res.json::<DiscordUserResponse>().await?;
    Ok(json)
}

pub async fn create_direct_message_channel(
    config: &DiscordConfig,
    bot_token: &str,
    recipient_id: &str,
) -> std::result::Result<String, DiscordApiError> {
    let client =
        build_client(config).map_err(|error| DiscordApiError::Transport(error.to_string()))?;
    let response = client
        .post(format!("{}/users/@me/channels", config.api_base_url))
        .header(AUTHORIZATION, format!("Bot {bot_token}"))
        .json(&CreateDmChannelRequest { recipient_id })
        .send()
        .await
        .map_err(map_reqwest_error)?;
    let response = ensure_success(response).await?;
    let json = response
        .json::<CreateDmChannelResponse>()
        .await
        .map_err(map_reqwest_error)?;
    Ok(json.id)
}

pub async fn send_channel_message(
    config: &DiscordConfig,
    bot_token: &str,
    channel_id: &str,
    content: &str,
) -> std::result::Result<DiscordBotMessageResult, DiscordApiError> {
    let client =
        build_client(config).map_err(|error| DiscordApiError::Transport(error.to_string()))?;
    let response = client
        .post(format!(
            "{}/channels/{channel_id}/messages",
            config.api_base_url
        ))
        .header(AUTHORIZATION, format!("Bot {bot_token}"))
        .json(&SendMessageRequest {
            content: Some(content),
            embeds: Vec::new(),
            nonce: None,
            enforce_nonce: false,
        })
        .send()
        .await
        .map_err(map_reqwest_error)?;
    let response = ensure_success(response).await?;
    let json = response
        .json::<SendMessageResponse>()
        .await
        .map_err(map_reqwest_error)?;
    Ok(DiscordBotMessageResult {
        channel_id: channel_id.to_string(),
        message_id: json.id,
    })
}

pub async fn send_channel_embed(
    config: &DiscordConfig,
    bot_token: &str,
    channel_id: &str,
    embed: DiscordEmbed<'_>,
) -> std::result::Result<DiscordBotMessageResult, DiscordApiError> {
    send_channel_embed_with_nonce(config, bot_token, channel_id, embed, None).await
}

pub async fn send_channel_embed_idempotent(
    config: &DiscordConfig,
    bot_token: &str,
    channel_id: &str,
    embed: DiscordEmbed<'_>,
    nonce: &str,
) -> std::result::Result<DiscordBotMessageResult, DiscordApiError> {
    send_channel_embed_with_nonce(config, bot_token, channel_id, embed, Some(nonce)).await
}

pub async fn send_channel_embeds_idempotent(
    config: &DiscordConfig,
    bot_token: &str,
    channel_id: &str,
    embeds: Vec<Value>,
    nonce: &str,
) -> std::result::Result<DiscordBotMessageResult, DiscordApiError> {
    send_channel_embeds_with_nonce(config, bot_token, channel_id, embeds, Some(nonce)).await
}

async fn send_channel_embed_with_nonce(
    config: &DiscordConfig,
    bot_token: &str,
    channel_id: &str,
    embed: DiscordEmbed<'_>,
    nonce: Option<&str>,
) -> std::result::Result<DiscordBotMessageResult, DiscordApiError> {
    let embed = serde_json::to_value(embed)
        .map_err(|error| DiscordApiError::Transport(error.to_string()))?;
    send_channel_embeds_with_nonce(config, bot_token, channel_id, vec![embed], nonce).await
}

async fn send_channel_embeds_with_nonce(
    config: &DiscordConfig,
    bot_token: &str,
    channel_id: &str,
    embeds: Vec<Value>,
    nonce: Option<&str>,
) -> std::result::Result<DiscordBotMessageResult, DiscordApiError> {
    let client =
        build_client(config).map_err(|error| DiscordApiError::Transport(error.to_string()))?;
    let response = client
        .post(format!(
            "{}/channels/{channel_id}/messages",
            config.api_base_url
        ))
        .header(AUTHORIZATION, format!("Bot {bot_token}"))
        .json(&SendMessageRequest {
            content: None,
            embeds,
            nonce,
            enforce_nonce: nonce.is_some(),
        })
        .send()
        .await
        .map_err(map_reqwest_error)?;
    let response = ensure_success(response).await?;
    let json = response
        .json::<SendMessageResponse>()
        .await
        .map_err(map_reqwest_error)?;
    Ok(DiscordBotMessageResult {
        channel_id: channel_id.to_string(),
        message_id: json.id,
    })
}

pub async fn fetch_guild_scheduled_events(
    config: &DiscordConfig,
    bot_token: &str,
    guild_id: &str,
) -> std::result::Result<Vec<DiscordScheduledEvent>, DiscordApiError> {
    let client =
        build_client(config).map_err(|error| DiscordApiError::Transport(error.to_string()))?;
    let response = client
        .get(format!(
            "{}/guilds/{}/scheduled-events?with_user_count=true",
            config.api_base_url, guild_id
        ))
        .header(AUTHORIZATION, format!("Bot {bot_token}"))
        .send()
        .await
        .map_err(map_reqwest_error)?;
    let response = ensure_success(response).await?;
    response
        .json::<Vec<DiscordScheduledEvent>>()
        .await
        .map_err(map_reqwest_error)
}

fn build_client(config: &DiscordConfig) -> Result<Client> {
    let client =
        Client::builder().timeout(std::time::Duration::from_millis(config.http_timeout_ms));
    let client = if let Some(p) = config.discord_proxy.clone() {
        client.proxy(p)
    } else {
        client
    };
    client.build().map_err(Into::into)
}

fn map_reqwest_error(error: reqwest::Error) -> DiscordApiError {
    if error.is_timeout() {
        DiscordApiError::Timeout("Discord request timed out".to_string())
    } else {
        DiscordApiError::Transport(error.to_string())
    }
}

async fn ensure_success(
    response: reqwest::Response,
) -> std::result::Result<reqwest::Response, DiscordApiError> {
    let status = response.status();
    if status.is_success() {
        return Ok(response);
    }

    let body = response.text().await.unwrap_or_else(|_| String::new());
    let message = if body.trim().is_empty() {
        format!("Discord returned HTTP {}", status.as_u16())
    } else {
        format!("Discord returned HTTP {}: {}", status.as_u16(), body)
    };

    match status.as_u16() {
        403 => Err(DiscordApiError::Forbidden(message)),
        404 => Err(DiscordApiError::NotFound(message)),
        429 => Err(DiscordApiError::RateLimited(message)),
        _ => Err(DiscordApiError::Http(message)),
    }
}
