use crate::app::config::AppConfig;
use crate::entities::UserModel;
use anyhow::anyhow;
use hmac::{Hmac, Mac};
use jwt::{SignWithKey, VerifyWithKey};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::collections::BTreeMap;

#[derive(Serialize, Deserialize)]
pub struct JwtContent {
    #[serde(rename = "userId")]
    pub user_id: String,
    #[serde(rename = "authEpoch")]
    pub auth_epoch: i32,
}

pub fn sign_token(user: &UserModel, config: &AppConfig) -> anyhow::Result<String> {
    let key: Hmac<Sha256> = Hmac::new_from_slice(config.jwt_secret.as_bytes())?;
    let mut claims = BTreeMap::new();
    claims.insert("user_id", user.id.to_string());
    claims.insert("auth_epoch", user.auth_epoch.to_string());
    let token_str = claims.sign_with_key(&key)?;
    Ok(token_str)
}

pub fn verify_token(token: &str, config: &AppConfig) -> anyhow::Result<JwtContent> {
    let key: Hmac<Sha256> = Hmac::new_from_slice(config.jwt_secret.as_bytes())?;
    let claims: BTreeMap<String, String> = token.verify_with_key(&key)?;
    let user_id = claims
        .get("user_id")
        .ok_or(anyhow!("user_id not found in token"))?;
    let auth_epoch = claims
        .get("auth_epoch")
        .map(|it| it.parse::<i32>())
        .transpose()?
        .unwrap_or(0);

    Ok(JwtContent {
        user_id: user_id.clone(),
        auth_epoch,
    })
}
