use std::path::{Path, PathBuf};
use std::sync::Arc;

use auth::app::config::{
    AppConfig, DatabaseConfig, DiscordConfig, EmailConfig, EmailProviderKind, MetricsConfig,
    ReceiptProviderKind, ReceiptsConfig, S3Config, ShopConfig, ShopPaymentProviderKind,
    YooKassaConfig,
};
use auth::app::router::build_router;
use auth::app::state::{AppState, SharedAppState};
use auth::entities::{
    AssetDefinition, AssetDefinitionColumn, AuditLog, AuditLogColumn, InventoryOperation,
    InventoryOperationColumn, Squad, User, WalletTransaction, WalletTransactionColumn,
};
use auth::services::db::{connect_db, run_migrations};
use auth::services::ownership::catalog::ensure_system_assets;
use aws_credential_types::Credentials;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use reqwest::Client;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter};
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use uuid::Uuid;

pub struct TestApp {
    pub address: String,
    pub client: Client,
    pub db: DatabaseConnection,
    pub config: AppConfig,
    _db_path: PathBuf,
    _server_task: tokio::task::JoinHandle<()>,
    _mock_s3_server_task: tokio::task::JoinHandle<()>,
}

#[derive(Clone)]
pub struct IssuedUser {
    pub user_id: String,
    pub access_token: String,
}

impl TestApp {
    pub async fn spawn() -> Self {
        Self::spawn_with_shop_config(ShopConfig {
            payment_provider: ShopPaymentProviderKind::Mock,
            yookassa: None,
            pending_payment_ttl_seconds: 900,
            reconciliation_interval_seconds: 30,
        })
        .await
    }

    pub async fn spawn_with_shop_payment_provider(
        shop_payment_provider: ShopPaymentProviderKind,
    ) -> Self {
        Self::spawn_with_shop_config(ShopConfig {
            payment_provider: shop_payment_provider,
            yookassa: None,
            pending_payment_ttl_seconds: 900,
            reconciliation_interval_seconds: 30,
        })
        .await
    }

    pub async fn spawn_with_yookassa(api_base_url: String, return_url: String) -> Self {
        Self::spawn_with_shop_config(ShopConfig {
            payment_provider: ShopPaymentProviderKind::YooKassa,
            yookassa: Some(YooKassaConfig {
                shop_id: "test-shop".to_string(),
                secret_key: "test-secret".to_string(),
                api_base_url,
                return_url,
            }),
            pending_payment_ttl_seconds: 900,
            reconciliation_interval_seconds: 30,
        })
        .await
    }

    pub async fn spawn_with_shop_config(shop: ShopConfig) -> Self {
        let (s3_endpoint, mock_s3_server_task) = spawn_mock_s3_server().await;
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind test port");
        let address = format!("http://{}", listener.local_addr().expect("local addr"));

        let db_path = test_db_path();
        let s3_bucket = format!("test-bucket-{}", Uuid::new_v4().as_simple());
        let database = DatabaseConfig {
            db_url: format!("sqlite://{}?mode=rwc", normalize_sqlite_path(&db_path)),
        };
        let db = connect_db(&database).await.expect("connect test db");
        run_migrations(&db).await.expect("run migrations");
        ensure_system_assets(&db).await.expect("seed system assets");

        let config = AppConfig {
            binding_address: "127.0.0.1:0".to_string(),
            discord: DiscordConfig {
                api_base_url: "https://discord.test/api/v10".to_string(),
                oauth2_url: "https://discord.test/oauth".to_string(),
                redirect_url: "http://localhost:5173/auth/callback".to_string(),
                client_id: "test-client".to_string(),
                client_secret: "test-secret".to_string(),
                required_scopes: vec!["identify".to_string()],
                discord_proxy: None,
                bot_token: None,
                events_guild_id: None,
                http_timeout_ms: 10_000,
            },
            email: EmailConfig {
                enabled: false,
                provider: EmailProviderKind::Xyecoc,
                xyecoc: None,
                retry_interval_seconds: 300,
                failure_after_seconds: 86400,
            },
            database,
            s3: S3Config {
                endpoint: Some(s3_endpoint.clone()),
                region: "us-east-1".to_string(),
                bucket: s3_bucket,
                access_key_id: "test".to_string(),
                secret_access_key: "test".to_string(),
                force_path_style: true,
            },
            metrics: MetricsConfig {
                ingest_secret: Some("test-metrics-secret".to_string()),
                upload_max_bytes: 16 * 1024 * 1024,
                upload_timeout_seconds: 30,
                discord_channel_id: None,
                discord_retry_interval_seconds: 1,
                discord_max_attempts: 3,
                public_base_url: address.clone(),
            },
            littlemice: auth::app::config::LittlemiceConfig {
                public_base_url: address.clone(),
                push_ttl_seconds: 60,
                screenshot_max_bytes: 3840 * 2160 * 4,
                log_max_bytes: 1024 * 1024,
                info_max_bytes: 1024 * 1024,
                expiry_check_interval_seconds: 5,
                cleanup_interval_seconds: 3600,
                retention_days: 30,
                discord_log_channel_id: None,
            },
            shop,
            receipts: ReceiptsConfig {
                enabled: false,
                provider: ReceiptProviderKind::MyTax,
                retry_interval_seconds: 900,
                failure_after_seconds: 604800,
                mytax: None,
            },
            pow_complexity: 1,
            jwt_secret: "test-jwt-secret".to_string(),
            gamervii_compat: None,
        };

        let state: SharedAppState = Arc::new(RwLock::new(AppState::new(
            config.clone(),
            db.clone(),
            test_s3_client(&s3_endpoint).await,
        )));
        let app = build_router(state);
        let server_task = tokio::spawn(async move {
            if let Err(error) = axum::serve(listener, app.into_make_service()).await {
                panic!("serve test app: {error}");
            }
        });
        let test_app = Self {
            address,
            client: Client::new(),
            db,
            config,
            _db_path: db_path,
            _server_task: server_task,
            _mock_s3_server_task: mock_s3_server_task,
        };
        test_app.wait_until_ready().await;
        test_app
    }

    pub fn url(&self, path: &str) -> String {
        format!("{}{}", self.address, path)
    }

    pub async fn issue_user_token(
        &self,
        username: &str,
        is_superuser: bool,
        restrictions: &[&str],
    ) -> IssuedUser {
        let response = self
            .post_without_auth(
                "/api/test/issue-token",
                serde_json::json!({
                    "username": username,
                    "isSuperuser": is_superuser,
                    "restrictions": restrictions,
                }),
            )
            .await;
        assert!(
            response.status().is_success(),
            "issue token failed: {}",
            response.text().await.unwrap_or_default()
        );
        let body: serde_json::Value = response.json().await.expect("issue token json");
        IssuedUser {
            user_id: body["userId"].as_str().expect("userId").to_string(),
            access_token: body["accessToken"]
                .as_str()
                .expect("accessToken")
                .to_string(),
        }
    }

    pub async fn get_json(&self, path: &str, token: &str) -> reqwest::Response {
        self.send_with_retry(|| self.client.get(self.url(path)).bearer_auth(token))
            .await
    }

    pub async fn get_without_auth(&self, path: &str) -> reqwest::Response {
        self.send_with_retry(|| self.client.get(self.url(path)))
            .await
    }

    pub async fn get_json_value_without_auth(&self, path: &str) -> serde_json::Value {
        let response = self.get_without_auth(path).await;
        assert!(
            response.status().is_success(),
            "get without auth failed: {}",
            response.text().await.unwrap_or_default()
        );
        response.json().await.expect("json body")
    }

    pub async fn post_json(
        &self,
        path: &str,
        token: &str,
        body: serde_json::Value,
    ) -> reqwest::Response {
        self.send_with_retry(|| {
            self.client
                .post(self.url(path))
                .bearer_auth(token)
                .json(&body)
        })
        .await
    }

    pub async fn create_service_token(
        &self,
        admin: &IssuedUser,
        system_name: &str,
    ) -> String {
        let response = self
            .post_json(
                "/api/admin/service-tokens",
                &admin.access_token,
                serde_json::json!({
                    "systemName": system_name,
                    "description": format!("token for {system_name}")
                }),
            )
            .await;
        assert!(
            response.status().is_success(),
            "create service token failed: {}",
            response.text().await.unwrap_or_default()
        );
        let body: serde_json::Value = response.json().await.expect("service token json");
        body["plaintextToken"]
            .as_str()
            .expect("plaintextToken")
            .to_string()
    }

    pub async fn patch_json(
        &self,
        path: &str,
        token: &str,
        body: serde_json::Value,
    ) -> reqwest::Response {
        self.send_with_retry(|| {
            self.client
                .patch(self.url(path))
                .bearer_auth(token)
                .json(&body)
        })
        .await
    }

    pub async fn put_json(
        &self,
        path: &str,
        token: &str,
        body: serde_json::Value,
    ) -> reqwest::Response {
        self.send_with_retry(|| {
            self.client
                .put(self.url(path))
                .bearer_auth(token)
                .json(&body)
        })
        .await
    }

    pub async fn delete(&self, path: &str, token: &str) -> reqwest::Response {
        self.send_with_retry(|| self.client.delete(self.url(path)).bearer_auth(token))
            .await
    }

    pub async fn delete_with_json(
        &self,
        path: &str,
        token: &str,
        body: serde_json::Value,
    ) -> reqwest::Response {
        self.send_with_retry(|| {
            self.client
                .delete(self.url(path))
                .bearer_auth(token)
                .json(&body)
        })
        .await
    }

    pub async fn issue_invite(
        &self,
        leader: &IssuedUser,
        squad_id: &str,
        invited_user_id: &str,
    ) -> String {
        let response = self
            .post_json(
                &format!("/api/squads/{squad_id}/invites"),
                &leader.access_token,
                serde_json::json!({ "userId": invited_user_id }),
            )
            .await;
        assert!(response.status().is_success(), "invite failed");
        let body: serde_json::Value = response.json().await.expect("invite json");
        body["id"].as_str().expect("invite id").to_string()
    }

    pub async fn grant_user_restriction(
        &self,
        admin: &IssuedUser,
        user_id: &str,
        restriction_key: &str,
        reason: &str,
    ) -> reqwest::Response {
        self.post_json(
            &format!("/api/admin/users/{user_id}/restrictions/{restriction_key}"),
            &admin.access_token,
            serde_json::json!({ "reason": reason }),
        )
        .await
    }

    pub async fn revoke_user_restriction(
        &self,
        admin: &IssuedUser,
        user_id: &str,
        restriction_key: &str,
        reason: &str,
    ) -> reqwest::Response {
        self.delete_with_json(
            &format!("/api/admin/users/{user_id}/restrictions/{restriction_key}"),
            &admin.access_token,
            serde_json::json!({ "reason": reason }),
        )
        .await
    }

    pub async fn create_squad(&self, user: &IssuedUser, name: &str) -> serde_json::Value {
        let response = self
            .post_json(
                "/api/squads",
                &user.access_token,
                serde_json::json!({ "name": name }),
            )
            .await;
        assert!(
            response.status().is_success(),
            "create squad failed: {}",
            response.text().await.unwrap_or_default()
        );
        response.json().await.expect("create squad json")
    }

    pub async fn user_squad_id(&self, user_id: &str) -> Option<String> {
        let user = User::find_by_id(Uuid::parse_str(user_id).expect("uuid"))
            .one(&self.db)
            .await
            .expect("load user")
            .expect("user exists");
        user.squad_id.map(|id| id.to_string())
    }

    pub async fn invite_count_for_squad(&self, squad_id: &str) -> u64 {
        auth::entities::SquadInvite::find()
            .filter(
                auth::entities::SquadInviteColumn::SquadId
                    .eq(Uuid::parse_str(squad_id).expect("uuid")),
            )
            .count(&self.db)
            .await
            .expect("count invites")
    }

    pub async fn expire_invite(&self, invite_id: &str) {
        use auth::entities::SquadInviteActiveModel;
        use sea_orm::{ActiveModelTrait, Set};

        let invite =
            auth::entities::SquadInvite::find_by_id(Uuid::parse_str(invite_id).expect("uuid"))
                .one(&self.db)
                .await
                .expect("load invite")
                .expect("invite exists");
        let mut active: SquadInviteActiveModel = invite.into();
        active.expires_at = Set(chrono::Utc::now() - chrono::Duration::hours(1));
        active.update(&self.db).await.expect("update invite");
    }

    pub async fn audit_log_count(&self, action: &str) -> u64 {
        AuditLog::find()
            .filter(AuditLogColumn::Action.eq(action))
            .count(&self.db)
            .await
            .expect("count audit logs")
    }

    pub async fn my_restrictions(&self, user: &IssuedUser) -> Vec<serde_json::Value> {
        let response = self
            .get_json("/api/user/me/restrictions", &user.access_token)
            .await;
        assert!(
            response.status().is_success(),
            "get my restrictions failed: {}",
            response.text().await.unwrap_or_default()
        );
        response.json().await.expect("my restrictions json")
    }

    pub async fn my_invites(&self, user: &IssuedUser) -> Vec<serde_json::Value> {
        let response = self
            .get_json("/api/user/me/squad-invites", &user.access_token)
            .await;
        assert!(
            response.status().is_success(),
            "get my invites failed: {}",
            response.text().await.unwrap_or_default()
        );
        response.json().await.expect("my invites json")
    }

    pub async fn squad_exists(&self, squad_id: &str) -> bool {
        Squad::find_by_id(Uuid::parse_str(squad_id).expect("uuid"))
            .one(&self.db)
            .await
            .expect("load squad")
            .is_some()
    }

    pub async fn create_asset(
        &self,
        admin: &IssuedUser,
        body: serde_json::Value,
    ) -> serde_json::Value {
        let response = self
            .post_json("/api/admin/assets", &admin.access_token, body)
            .await;
        assert!(
            response.status().is_success(),
            "create asset failed: {}",
            response.text().await.unwrap_or_default()
        );
        response.json().await.expect("asset json")
    }

    pub async fn patch_asset(
        &self,
        admin: &IssuedUser,
        asset_id: &str,
        body: serde_json::Value,
    ) -> serde_json::Value {
        let response = self
            .patch_json(
                &format!("/api/admin/assets/{asset_id}"),
                &admin.access_token,
                body,
            )
            .await;
        assert!(
            response.status().is_success(),
            "patch asset failed: {}",
            response.text().await.unwrap_or_default()
        );
        response.json().await.expect("patched asset json")
    }

    pub async fn asset_by_key(&self, key: &str) -> Option<auth::entities::AssetDefinitionModel> {
        AssetDefinition::find()
            .filter(AssetDefinitionColumn::Key.eq(key))
            .one(&self.db)
            .await
            .expect("load asset by key")
    }

    pub async fn inventory_operation_count(&self, operation_type: &str) -> u64 {
        InventoryOperation::find()
            .filter(InventoryOperationColumn::OperationType.eq(operation_type))
            .count(&self.db)
            .await
            .expect("count inventory operations")
    }

    pub async fn wallet_transaction_count(&self, operation_type: &str) -> u64 {
        WalletTransaction::find()
            .filter(WalletTransactionColumn::OperationType.eq(operation_type))
            .count(&self.db)
            .await
            .expect("count wallet transactions")
    }

    async fn wait_until_ready(&self) {
        for _ in 0..50 {
            if let Ok(response) = self.client.get(self.url("/api/health")).send().await {
                if response.status().is_success() {
                    return;
                }
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }

        panic!("test server did not become ready in time");
    }

    pub async fn post_without_auth(
        &self,
        path: &str,
        body: serde_json::Value,
    ) -> reqwest::Response {
        self.send_with_retry(|| self.client.post(self.url(path)).json(&body))
            .await
    }

    pub async fn post_raw_without_auth(
        &self,
        path: &str,
        body: impl Into<String>,
    ) -> reqwest::Response {
        let body = body.into();
        self.send_with_retry(|| {
            self.client
                .post(self.url(path))
                .header(reqwest::header::CONTENT_TYPE, "application/json")
                .body(body.clone())
        })
        .await
    }

    pub async fn post_multipart(
        &self,
        path: &str,
        token: &str,
        file_name: &str,
        content_type: &str,
        bytes: Vec<u8>,
    ) -> reqwest::Response {
        let url = self.url(path);
        let token = token.to_string();
        let file_name = file_name.to_string();
        let content_type = content_type.to_string();
        self.send_with_retry(move || {
            let boundary = "----codex-form-boundary";
            let mut body = Vec::new();
            body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
            body.extend_from_slice(
                format!(
                    "Content-Disposition: form-data; name=\"file\"; filename=\"{}\"\r\n",
                    file_name
                )
                .as_bytes(),
            );
            body.extend_from_slice(format!("Content-Type: {}\r\n\r\n", content_type).as_bytes());
            body.extend_from_slice(&bytes);
            body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
            self.client
                .post(url.clone())
                .bearer_auth(token.clone())
                .header(
                    reqwest::header::CONTENT_TYPE,
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .body(body.clone())
        })
        .await
    }

    pub async fn post_raw_json(
        &self,
        path: &str,
        token: &str,
        body: impl Into<String>,
    ) -> reqwest::Response {
        let body = body.into();
        self.send_with_retry(|| {
            self.client
                .post(self.url(path))
                .bearer_auth(token)
                .header(reqwest::header::CONTENT_TYPE, "application/json")
                .body(body.clone())
        })
        .await
    }

    pub async fn post_multipart_without_auth_fields(
        &self,
        path: &str,
        parts: Vec<MultipartPart>,
    ) -> reqwest::Response {
        let url = self.url(path);
        self.send_with_retry(move || {
            let boundary = "----codex-form-boundary";
            let body = build_multipart_body(boundary, &parts);
            self.client
                .post(url.clone())
                .header(
                    reqwest::header::CONTENT_TYPE,
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .body(body)
        })
        .await
    }

    pub async fn get_bytes_without_auth(&self, path: &str) -> reqwest::Response {
        self.send_with_retry(|| self.client.get(self.url(path)))
            .await
    }

    async fn send_with_retry(
        &self,
        make_request: impl Fn() -> reqwest::RequestBuilder,
    ) -> reqwest::Response {
        let mut last_error = None;

        for _ in 0..10 {
            match make_request().send().await {
                Ok(response) => return response,
                Err(error) => {
                    last_error = Some(error);
                    tokio::time::sleep(std::time::Duration::from_millis(25)).await;
                }
            }
        }

        panic!("request failed after retries: {:?}", last_error);
    }
}

#[derive(Clone)]
pub enum MultipartPart {
    Text {
        name: String,
        value: String,
    },
    File {
        name: String,
        file_name: String,
        content_type: String,
        bytes: Vec<u8>,
    },
}

impl Drop for TestApp {
    fn drop(&mut self) {
        self._server_task.abort();
        self._mock_s3_server_task.abort();
    }
}

fn test_db_path() -> PathBuf {
    let dir = std::env::temp_dir().join("svogame-auth-tests");
    std::fs::create_dir_all(&dir).expect("create test db dir");
    dir.join(format!("{}.sqlite", Uuid::new_v4()))
}

fn normalize_sqlite_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn build_multipart_body(boundary: &str, parts: &[MultipartPart]) -> Vec<u8> {
    let mut body = Vec::new();
    for part in parts {
        body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
        match part {
            MultipartPart::Text { name, value } => {
                body.extend_from_slice(
                    format!("Content-Disposition: form-data; name=\"{name}\"\r\n\r\n").as_bytes(),
                );
                body.extend_from_slice(value.as_bytes());
                body.extend_from_slice(b"\r\n");
            }
            MultipartPart::File {
                name,
                file_name,
                content_type,
                bytes,
            } => {
                body.extend_from_slice(
                    format!(
                        "Content-Disposition: form-data; name=\"{name}\"; filename=\"{file_name}\"\r\n"
                    )
                    .as_bytes(),
                );
                body.extend_from_slice(format!("Content-Type: {content_type}\r\n\r\n").as_bytes());
                body.extend_from_slice(bytes);
                body.extend_from_slice(b"\r\n");
            }
        }
    }
    body.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());
    body
}

pub async fn test_s3_client(endpoint: &str) -> aws_sdk_s3::Client {
    let credentials = Credentials::new("test", "test", None, None, "tests");
    let shared_config = aws_config::defaults(aws_config::BehaviorVersion::latest())
        .credentials_provider(credentials)
        .region(aws_config::Region::new("us-east-1"))
        .load()
        .await;

    let config = aws_sdk_s3::config::Builder::from(&shared_config)
        .force_path_style(true)
        .endpoint_url(endpoint)
        .build();

    aws_sdk_s3::Client::from_conf(config)
}

async fn spawn_mock_s3_server() -> (String, tokio::task::JoinHandle<()>) {
    let storage = Arc::new(RwLock::new(std::collections::HashMap::new()));
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind mock s3 port");
    let address = listener.local_addr().expect("mock s3 local addr");
    let app = axum::Router::new()
        .route(
            "/{*path}",
            axum::routing::get(mock_s3_get)
                .put(mock_s3_put)
                .delete(mock_s3_delete),
        )
        .layer(axum::extract::DefaultBodyLimit::max(80 * 1024 * 1024))
        .with_state(storage);

    let server_task = tokio::spawn(async move {
        if let Err(error) = axum::serve(listener, app.into_make_service()).await {
            panic!("serve mock s3: {error}");
        }
    });

    for _ in 0..50 {
        if tokio::net::TcpStream::connect(address).await.is_ok() {
            return (format!("http://{}", address), server_task);
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }

    panic!("mock s3 server did not become ready in time");
}

async fn mock_s3_put(
    axum::extract::State(storage): axum::extract::State<
        Arc<RwLock<std::collections::HashMap<String, Vec<u8>>>>,
    >,
    axum::extract::Path(path): axum::extract::Path<String>,
    body: axum::body::Bytes,
) -> impl axum::response::IntoResponse {
    storage.write().await.insert(path, body.to_vec());
    StatusCode::OK
}

async fn mock_s3_get(
    axum::extract::State(storage): axum::extract::State<
        Arc<RwLock<std::collections::HashMap<String, Vec<u8>>>>,
    >,
    axum::extract::Path(path): axum::extract::Path<String>,
) -> impl axum::response::IntoResponse {
    let storage = storage.read().await;
    if let Some(bytes) = storage.get(&path) {
        (
            StatusCode::OK,
            [(reqwest::header::CONTENT_TYPE, "application/octet-stream")],
            bytes.clone(),
        )
            .into_response()
    } else {
        StatusCode::NOT_FOUND.into_response()
    }
}

async fn mock_s3_delete(
    axum::extract::State(storage): axum::extract::State<
        Arc<RwLock<std::collections::HashMap<String, Vec<u8>>>>,
    >,
    axum::extract::Path(path): axum::extract::Path<String>,
) -> impl axum::response::IntoResponse {
    storage.write().await.remove(&path);
    StatusCode::NO_CONTENT
}
