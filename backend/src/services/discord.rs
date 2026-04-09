use crate::app::config::DiscordConfig;
use anyhow::Result;
use reqwest::{Client, header::CONTENT_TYPE};
use serde::Deserialize;

const API_ENDPOINT: &str = "https://discord.com/api/v10";

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
    let client = Client::builder();
    let client = if let Some(p) = config.discord_proxy.clone() {
        client.proxy(p)
    } else {
        client
    };
    let client = client.build()?;

    let params = [
        ("grant_type", "authorization_code"),
        ("code", code),
        ("redirect_uri", &config.redirect_url),
    ];

    let res = client
        .post(&format!("{}/oauth2/token", API_ENDPOINT))
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
    let client = Client::builder();
    let client = if let Some(p) = config.discord_proxy.clone() {
        client.proxy(p)
    } else {
        client
    };
    let client = client.build()?;

    let res = client
        .get(&format!("{}/users/@me", API_ENDPOINT))
        .bearer_auth(access_token)
        .send()
        .await?
        .error_for_status()?;

    let json = res.json::<DiscordUserResponse>().await?;
    Ok(json)
}
