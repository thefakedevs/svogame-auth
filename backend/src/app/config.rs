use anyhow::Context;
use anyhow::Result;
use reqwest::Proxy;
use tracing::warn;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub binding_address: String,
    pub discord: DiscordConfig,
    pub email: EmailConfig,
    pub database: DatabaseConfig,
    pub s3: S3Config,
    pub littlemice: LittlemiceConfig,
    pub shop: ShopConfig,
    pub receipts: ReceiptsConfig,
    pub pow_complexity: i16,
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
    pub bot_token: Option<String>,
    pub events_guild_id: Option<String>,
    pub http_timeout_ms: u64,
}

#[derive(Debug, Clone)]
pub struct EmailConfig {
    pub enabled: bool,
    pub provider: EmailProviderKind,
    pub xyecoc: Option<XyecocMailConfig>,
    pub retry_interval_seconds: i64,
    pub failure_after_seconds: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EmailProviderKind {
    Xyecoc,
}

#[derive(Debug, Clone)]
pub struct XyecocMailConfig {
    pub login: String,
    pub password: String,
    pub api_base_url: String,
    pub current_lang: String,
    pub auth_local_part_only: bool,
    pub http_timeout_ms: u64,
}

#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    pub db_url: String,
}

#[derive(Debug, Clone)]
pub struct S3Config {
    pub endpoint: Option<String>,
    pub region: String,
    pub bucket: String,
    pub access_key_id: String,
    pub secret_access_key: String,
    pub force_path_style: bool,
}

#[derive(Debug, Clone)]
pub struct LittlemiceConfig {
    pub public_base_url: String,
    pub push_ttl_seconds: i64,
    pub screenshot_max_bytes: usize,
    pub log_max_bytes: usize,
    pub info_max_bytes: usize,
    pub expiry_check_interval_seconds: u64,
    pub cleanup_interval_seconds: u64,
    pub retention_days: i64,
    pub discord_log_channel_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ShopConfig {
    pub payment_provider: ShopPaymentProviderKind,
    pub yookassa: Option<YooKassaConfig>,
    pub pending_payment_ttl_seconds: i64,
    pub reconciliation_interval_seconds: u64,
}

#[derive(Debug, Clone)]
pub struct ReceiptsConfig {
    pub enabled: bool,
    pub provider: ReceiptProviderKind,
    pub retry_interval_seconds: i64,
    pub failure_after_seconds: i64,
    pub mytax: Option<MyTaxConfig>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReceiptProviderKind {
    MyTax,
}

#[derive(Debug, Clone)]
pub struct MyTaxConfig {
    pub inn: String,
    pub password: String,
    pub api_base_url: String,
    pub device_prefix: String,
    pub zone_offset: String,
}

#[derive(Debug, Clone)]
pub struct YooKassaConfig {
    pub shop_id: String,
    pub secret_key: String,
    pub api_base_url: String,
    pub return_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShopPaymentProviderKind {
    Mock,
    YooKassa,
    Disabled,
}

impl AppConfig {
    pub(crate) fn from_env() -> Result<Self> {
        let binding_address =
            std::env::var("BINDING_ADDRESS").unwrap_or_else(|_| "0.0.0.0:3000".to_string());
        let discord = DiscordConfig::from_env()?;
        let email = EmailConfig::from_env()?;
        let database = DatabaseConfig::from_env()?;
        let s3 = S3Config::from_env()?;
        let littlemice = LittlemiceConfig::from_env()?;
        let shop = ShopConfig::from_env()?;
        let receipts = ReceiptsConfig::from_env()?;
        let pow_complexity = std::env::var("POW_COMPLEXITY")
            .unwrap_or_else(|_| "19".to_string())
            .parse::<i16>()
            .context("POW_COMPLEXITY must be a valid u8")?;
        let jwt_secret = std::env::var("JWT_SECRET").context("JWT_SECRET not set")?;
        let gamervii_compat = GamerviiCompatConfig::from_env();

        Ok(AppConfig {
            binding_address,
            discord,
            email,
            database,
            s3,
            littlemice,
            shop,
            receipts,
            pow_complexity,
            jwt_secret,
            gamervii_compat,
        })
    }
}

impl EmailConfig {
    fn from_env() -> Result<Self> {
        let enabled = std::env::var("EMAIL_DELIVERY_ENABLED")
            .unwrap_or_else(|_| "false".to_string())
            == "true";
        let provider = EmailProviderKind::from_env()?;
        let retry_interval_seconds = std::env::var("EMAIL_RETRY_INTERVAL_SECONDS")
            .unwrap_or_else(|_| "300".to_string())
            .parse::<i64>()
            .context("EMAIL_RETRY_INTERVAL_SECONDS must be a valid integer")?;
        if retry_interval_seconds <= 0 {
            anyhow::bail!("EMAIL_RETRY_INTERVAL_SECONDS must be positive");
        }
        let failure_after_seconds = std::env::var("EMAIL_FAILURE_AFTER_SECONDS")
            .unwrap_or_else(|_| "86400".to_string())
            .parse::<i64>()
            .context("EMAIL_FAILURE_AFTER_SECONDS must be a valid integer")?;
        if failure_after_seconds <= 0 {
            anyhow::bail!("EMAIL_FAILURE_AFTER_SECONDS must be positive");
        }

        let xyecoc = if enabled && provider == EmailProviderKind::Xyecoc {
            Some(XyecocMailConfig::from_env()?)
        } else {
            None
        };

        Ok(Self {
            enabled,
            provider,
            xyecoc,
            retry_interval_seconds,
            failure_after_seconds,
        })
    }
}

impl EmailProviderKind {
    fn from_env() -> Result<Self> {
        let value = std::env::var("EMAIL_PROVIDER")
            .unwrap_or_else(|_| "xyecoc".to_string())
            .trim()
            .to_lowercase();
        match value.as_str() {
            "xyecoc" | "ксайкок" => Ok(Self::Xyecoc),
            _ => anyhow::bail!("EMAIL_PROVIDER must be one of: xyecoc"),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Xyecoc => "xyecoc",
        }
    }
}

impl XyecocMailConfig {
    fn from_env() -> Result<Self> {
        let login = std::env::var("XYECOC_MAIL_LOGIN").context("XYECOC_MAIL_LOGIN not set")?;
        let password =
            std::env::var("XYECOC_MAIL_PASSWORD").context("XYECOC_MAIL_PASSWORD not set")?;
        let api_base_url = std::env::var("XYECOC_API_BASE_URL")
            .unwrap_or_else(|_| "https://api.xyecoc.com".to_string())
            .trim()
            .trim_end_matches('/')
            .to_string();
        if api_base_url.is_empty() {
            anyhow::bail!("XYECOC_API_BASE_URL must not be empty");
        }
        let current_lang =
            std::env::var("XYECOC_MAIL_CURRENT_LANG").unwrap_or_else(|_| "mail".to_string());
        let auth_local_part_only = std::env::var("XYECOC_AUTH_LOCAL_PART_ONLY")
            .unwrap_or_else(|_| "true".to_string())
            == "true";
        let http_timeout_ms = std::env::var("XYECOC_HTTP_TIMEOUT_MS")
            .unwrap_or_else(|_| "10000".to_string())
            .parse::<u64>()
            .context("XYECOC_HTTP_TIMEOUT_MS must be a valid integer")?;

        Ok(Self {
            login,
            password,
            api_base_url,
            current_lang,
            auth_local_part_only,
            http_timeout_ms,
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
        let bot_token = std::env::var("DISCORD_BOT_TOKEN")
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        let events_guild_id = std::env::var("DISCORD_EVENTS_GUILD_ID")
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        let http_timeout_ms = std::env::var("DISCORD_HTTP_TIMEOUT_MS")
            .unwrap_or_else(|_| "10000".to_string())
            .parse::<u64>()
            .context("DISCORD_HTTP_TIMEOUT_MS must be a valid integer")?;

        Ok(DiscordConfig {
            oauth2_url,
            redirect_url,
            client_id,
            client_secret,
            required_scopes: scopes.split('+').map(|s| s.to_string()).collect(),
            discord_proxy,
            bot_token,
            events_guild_id,
            http_timeout_ms,
        })
    }
}

impl DatabaseConfig {
    fn from_env() -> Result<Self> {
        let db_url = std::env::var("DATABASE_URL").context("DATABASE_URL not set")?;
        Ok(DatabaseConfig { db_url })
    }
}

impl S3Config {
    fn from_env() -> Result<Self> {
        let endpoint = std::env::var("S3_ENDPOINT")
            .ok()
            .filter(|it| !it.trim().is_empty());
        let region = std::env::var("S3_REGION").context("S3_REGION not set")?;
        let bucket = std::env::var("S3_BUCKET").context("S3_BUCKET not set")?;
        let access_key_id =
            std::env::var("S3_ACCESS_KEY_ID").context("S3_ACCESS_KEY_ID not set")?;
        let secret_access_key =
            std::env::var("S3_SECRET_ACCESS_KEY").context("S3_SECRET_ACCESS_KEY not set")?;
        let force_path_style =
            std::env::var("S3_FORCE_PATH_STYLE").unwrap_or_else(|_| "true".to_string()) == "true";

        Ok(Self {
            endpoint,
            region,
            bucket,
            access_key_id,
            secret_access_key,
            force_path_style,
        })
    }
}

impl ShopConfig {
    fn from_env() -> Result<Self> {
        let payment_provider = ShopPaymentProviderKind::from_env()?;
        let yookassa = if payment_provider == ShopPaymentProviderKind::YooKassa {
            Some(YooKassaConfig::from_env()?)
        } else {
            None
        };
        let pending_payment_ttl_seconds = std::env::var("SHOP_PENDING_PAYMENT_TTL_SECONDS")
            .unwrap_or_else(|_| "900".to_string())
            .parse::<i64>()
            .context("SHOP_PENDING_PAYMENT_TTL_SECONDS must be a valid integer")?;
        if pending_payment_ttl_seconds <= 0 {
            anyhow::bail!("SHOP_PENDING_PAYMENT_TTL_SECONDS must be positive");
        }

        let reconciliation_interval_seconds = std::env::var("SHOP_RECONCILIATION_INTERVAL_SECONDS")
            .unwrap_or_else(|_| "30".to_string())
            .parse::<u64>()
            .context("SHOP_RECONCILIATION_INTERVAL_SECONDS must be a valid integer")?;
        if reconciliation_interval_seconds == 0 {
            anyhow::bail!("SHOP_RECONCILIATION_INTERVAL_SECONDS must be positive");
        }

        Ok(Self {
            payment_provider,
            yookassa,
            pending_payment_ttl_seconds,
            reconciliation_interval_seconds,
        })
    }
}

impl LittlemiceConfig {
    fn from_env() -> Result<Self> {
        let public_base_url = std::env::var("LITTLEMICE_PUBLIC_BASE_URL")
            .context("LITTLEMICE_PUBLIC_BASE_URL not set")?
            .trim()
            .trim_end_matches('/')
            .to_string();
        if public_base_url.is_empty() {
            anyhow::bail!("LITTLEMICE_PUBLIC_BASE_URL must not be empty");
        }
        let push_ttl_seconds = std::env::var("LITTLEMICE_PUSH_TTL_SECONDS")
            .unwrap_or_else(|_| "60".to_string())
            .parse::<i64>()
            .context("LITTLEMICE_PUSH_TTL_SECONDS must be a valid integer")?;
        if push_ttl_seconds <= 0 {
            anyhow::bail!("LITTLEMICE_PUSH_TTL_SECONDS must be positive");
        }

        let screenshot_max_bytes = std::env::var("LITTLEMICE_SCREENSHOT_MAX_BYTES")
            .unwrap_or_else(|_| (10 * 1024 * 1024).to_string())
            .parse::<usize>()
            .context("LITTLEMICE_SCREENSHOT_MAX_BYTES must be a valid integer")?;
        let log_max_bytes = std::env::var("LITTLEMICE_LOG_MAX_BYTES")
            .unwrap_or_else(|_| (1024 * 1024).to_string())
            .parse::<usize>()
            .context("LITTLEMICE_LOG_MAX_BYTES must be a valid integer")?;
        let info_max_bytes = std::env::var("LITTLEMICE_INFO_MAX_BYTES")
            .unwrap_or_else(|_| (1024 * 1024).to_string())
            .parse::<usize>()
            .context("LITTLEMICE_INFO_MAX_BYTES must be a valid integer")?;
        let expiry_check_interval_seconds = std::env::var("LITTLEMICE_EXPIRY_CHECK_INTERVAL_SECONDS")
            .unwrap_or_else(|_| "5".to_string())
            .parse::<u64>()
            .context("LITTLEMICE_EXPIRY_CHECK_INTERVAL_SECONDS must be a valid integer")?;
        let cleanup_interval_seconds = std::env::var("LITTLEMICE_CLEANUP_INTERVAL_SECONDS")
            .unwrap_or_else(|_| "3600".to_string())
            .parse::<u64>()
            .context("LITTLEMICE_CLEANUP_INTERVAL_SECONDS must be a valid integer")?;
        let retention_days = std::env::var("LITTLEMICE_RETENTION_DAYS")
            .unwrap_or_else(|_| "30".to_string())
            .parse::<i64>()
            .context("LITTLEMICE_RETENTION_DAYS must be a valid integer")?;
        if expiry_check_interval_seconds == 0 || cleanup_interval_seconds == 0 || retention_days <= 0
        {
            anyhow::bail!("Littlemice intervals and retention must be positive");
        }
        let discord_log_channel_id = std::env::var("LITTLEMICE_DISCORD_LOG_CHANNEL_ID")
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());

        Ok(Self {
            public_base_url,
            push_ttl_seconds,
            screenshot_max_bytes,
            log_max_bytes,
            info_max_bytes,
            expiry_check_interval_seconds,
            cleanup_interval_seconds,
            retention_days,
            discord_log_channel_id,
        })
    }
}

impl ReceiptsConfig {
    fn from_env() -> Result<Self> {
        let enabled = std::env::var("SHOP_RECEIPTS_ENABLED")
            .unwrap_or_else(|_| "false".to_string())
            == "true";
        let provider = ReceiptProviderKind::from_env()?;
        let retry_interval_seconds = std::env::var("MYTAX_RECEIPT_RETRY_INTERVAL_SECONDS")
            .unwrap_or_else(|_| "900".to_string())
            .parse::<i64>()
            .context("MYTAX_RECEIPT_RETRY_INTERVAL_SECONDS must be a valid integer")?;
        if retry_interval_seconds <= 0 {
            anyhow::bail!("MYTAX_RECEIPT_RETRY_INTERVAL_SECONDS must be positive");
        }
        let failure_after_seconds = std::env::var("MYTAX_RECEIPT_FAILURE_AFTER_SECONDS")
            .unwrap_or_else(|_| "604800".to_string())
            .parse::<i64>()
            .context("MYTAX_RECEIPT_FAILURE_AFTER_SECONDS must be a valid integer")?;
        if failure_after_seconds <= 0 {
            anyhow::bail!("MYTAX_RECEIPT_FAILURE_AFTER_SECONDS must be positive");
        }

        let mytax = if enabled && provider == ReceiptProviderKind::MyTax {
            Some(MyTaxConfig::from_env()?)
        } else {
            None
        };

        Ok(Self {
            enabled,
            provider,
            retry_interval_seconds,
            failure_after_seconds,
            mytax,
        })
    }
}

impl ReceiptProviderKind {
    fn from_env() -> Result<Self> {
        let value = std::env::var("SHOP_RECEIPTS_PROVIDER")
            .unwrap_or_else(|_| "mytax".to_string())
            .trim()
            .to_lowercase();
        match value.as_str() {
            "mytax" | "my_tax" | "my-tax" => Ok(Self::MyTax),
            _ => anyhow::bail!("SHOP_RECEIPTS_PROVIDER must be one of: mytax"),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::MyTax => "mytax",
        }
    }
}

impl MyTaxConfig {
    fn from_env() -> Result<Self> {
        let inn = std::env::var("MYTAX_INN").context("MYTAX_INN not set")?;
        let password = std::env::var("MYTAX_PASSWORD").context("MYTAX_PASSWORD not set")?;
        let api_base_url = std::env::var("MYTAX_API_BASE_URL")
            .unwrap_or_else(|_| "https://lknpd.nalog.ru/api/v1".to_string())
            .trim()
            .trim_end_matches('/')
            .to_string();
        if api_base_url.is_empty() {
            anyhow::bail!("MYTAX_API_BASE_URL must not be empty");
        }
        let device_prefix = std::env::var("MYTAX_DEVICE_PREFIX")
            .unwrap_or_else(|_| "svogame_".to_string())
            .trim()
            .to_string();
        if device_prefix.len() > 20 {
            anyhow::bail!("MYTAX_DEVICE_PREFIX must be at most 20 characters");
        }
        let zone_offset =
            std::env::var("MYTAX_ZONE_OFFSET").unwrap_or_else(|_| "+03:00".to_string());

        Ok(Self {
            inn,
            password,
            api_base_url,
            device_prefix,
            zone_offset,
        })
    }
}

impl ShopPaymentProviderKind {
    fn from_env() -> Result<Self> {
        let default_value = if cfg!(debug_assertions) {
            "mock"
        } else {
            "disabled"
        };
        let value =
            std::env::var("SHOP_PAYMENT_PROVIDER").unwrap_or_else(|_| default_value.to_string());
        match value.trim().to_lowercase().as_str() {
            "mock" => Ok(Self::Mock),
            "yookassa" | "yoo_kassa" | "yoo-kassa" => Ok(Self::YooKassa),
            "disabled" | "none" => Ok(Self::Disabled),
            _ => anyhow::bail!("SHOP_PAYMENT_PROVIDER must be one of: mock, yookassa, disabled"),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Mock => "mock",
            Self::YooKassa => "yookassa",
            Self::Disabled => "disabled",
        }
    }
}

impl YooKassaConfig {
    fn from_env() -> Result<Self> {
        let shop_id = std::env::var("YOOKASSA_SHOP_ID").context("YOOKASSA_SHOP_ID not set")?;
        let secret_key =
            std::env::var("YOOKASSA_SECRET_KEY").context("YOOKASSA_SECRET_KEY not set")?;
        let return_url = resolve_yookassa_return_url()?;
        let api_base_url = std::env::var("YOOKASSA_API_BASE_URL")
            .unwrap_or_else(|_| "https://api.yookassa.ru/v3".to_string());
        let api_base_url = api_base_url.trim().trim_end_matches('/').to_string();
        if api_base_url.is_empty() {
            anyhow::bail!("YOOKASSA_API_BASE_URL must not be empty");
        }

        Ok(Self {
            shop_id,
            secret_key,
            api_base_url,
            return_url,
        })
    }
}

fn resolve_yookassa_return_url() -> Result<String> {
    if let Ok(return_url) = std::env::var("YOOKASSA_RETURN_URL") {
        let return_url = return_url.trim();
        if !return_url.is_empty() {
            return Ok(return_url.to_string());
        }
    }

    if let Some(debug_port) = read_debug_port()? {
        return Ok(format!("http://127.0.0.1:{debug_port}/api/docs"));
    }

    Err(anyhow::anyhow!(
        "YOOKASSA_RETURN_URL not set and DEBUG_PORT is unavailable for local fallback"
    ))
}

fn read_debug_port() -> Result<Option<u16>> {
    let Some(raw_port) = std::env::var("DEBUG_PORT").ok() else {
        return Ok(None);
    };
    let raw_port = raw_port.trim();
    if raw_port.is_empty() {
        return Ok(None);
    }
    let port = raw_port
        .parse::<u16>()
        .context("DEBUG_PORT must be a valid TCP port")?;
    Ok(Some(port))
}
