use anyhow::Context;
use anyhow::Result;
use reqwest::Proxy;
use tracing::warn;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub binding_address: String,
    pub discord: DiscordConfig,
    pub database: DatabaseConfig,
    pub pow_complexity: u8,
    pub jwt_secret: String,
    pub gamervii_compat: Option<GamerviiCompatConfig>,
}

#[derive(Debug, Clone)]
pub struct GamerviiCompatConfig {
    pub endpoint: String,
    pub token: String,
}

#[derive(Debug, Clone)]
pub struct DiscordConfig {
    pub oauth2_url: String,
    pub redirect_url: String,
    pub client_id: String,
    pub client_secret: String,
    pub required_scopes: Vec<String>,
    pub discord_proxy: Option<Proxy>,
}

#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    pub db_url: String,
}

impl AppConfig {
    pub(crate) fn from_env() -> Result<Self> {
        let binding_address =
            std::env::var("BINDING_ADDRESS").unwrap_or_else(|_| "0.0.0.0:3000".to_string());
        let discord = DiscordConfig::from_env()?;
        let database = DatabaseConfig::from_env()?;
        let pow_complexity = std::env::var("POW_COMPLEXITY")
            .unwrap_or_else(|_| "19".to_string())
            .parse::<u8>()
            .context("POW_COMPLEXITY must be a valid u8")?;
        let jwt_secret = std::env::var("JWT_SECRET").context("JWT_SECRET not set")?;
        let gamervii_compat = GamerviiCompatConfig::from_env();

        Ok(AppConfig {
            binding_address,
            discord,
            database,
            pow_complexity,
            jwt_secret,
            gamervii_compat,
        })
    }
}

impl GamerviiCompatConfig {
    fn from_env() -> Option<Self> {
        let enabled = std::env::var("GAMERVII_COMPAT_ENABLED")
            .unwrap_or_else(|_| "false".to_string())
            == "true";
        if !enabled {
            return None;
        }

        let endpoint = std::env::var("GAMERVII_COMPAT_ENDPOINT").ok()?;
        let token = std::env::var("GAMERVII_COMPAT_TOKEN").ok()?;

        Some(GamerviiCompatConfig { endpoint, token })
    }
}

impl DiscordConfig {
    fn from_env() -> Result<Self> {
        let client_id = std::env::var("DISCORD_CLIENT_ID").context("DISCORD_CLIENT_ID not set")?;
        let redirect_url =
            std::env::var("DISCORD_REDIRECT_URI").context("DISCORD_REDIRECT_URI not set")?;
        let scopes = "identify+email+openid";
        let oauth2_url = format!(
            "https://discord.com/oauth2/authorize?client_id={}&response_type=code&redirect_uri={}&scope={}",
            client_id,
            urlencoding::encode(&redirect_url),
            urlencoding::encode(scopes)
        );
        let client_secret =
            std::env::var("DISCORD_CLIENT_SECRET").context("DISCORD_CLIENT_SECRET not set")?;
        let discord_proxy = std::env::var("DISCORD_PROXY").ok();
        let discord_proxy = if let Some(p) = discord_proxy {
            Some(Proxy::all(p)?)
        } else {
            warn!("DISCORD_PROXY variable not set. Make sure service is hosting out of Russia.");
            None
        };

        Ok(DiscordConfig {
            oauth2_url,
            redirect_url,
            client_id,
            client_secret,
            required_scopes: scopes.split('+').map(|s| s.to_string()).collect(),
            discord_proxy,
        })
    }
}

impl DatabaseConfig {
    fn from_env() -> Result<Self> {
        let db_url = std::env::var("DATABASE_URL").context("DATABASE_URL not set")?;
        Ok(DatabaseConfig { db_url })
    }
}
