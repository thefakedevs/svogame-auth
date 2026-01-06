use anyhow::Result;
use crate::state::config::GamerviiCompatConfig;
use reqwest::Client;
use serde_json::json;
use tracing::{info, error};

pub async fn remove_player_from_gml(config: &GamerviiCompatConfig, user_uuid: &str) -> Result<()> {
    info!("Attempting to remove player {} from GML at {}", user_uuid, config.endpoint);
    let client = Client::new();
    let url = format!("{}/api/v1/players/remove", config.endpoint.trim_end_matches('/'));

    let res = client.post(&url)
        .header("Authorization", format!("Bearer {}", config.token))
        .json(&json!([user_uuid]))
        .send()
        .await?;

    if !res.status().is_success() {
        let status = res.status();
        let text = res.text().await.unwrap_or_default();
        error!("GML request failed for user {} with status {}: {}", user_uuid, status, text);
        return Err(anyhow::anyhow!("GML request failed with status {}: {}", status, text));
    }

    info!("Successfully removed player {} from GML", user_uuid);
    Ok(())
}
