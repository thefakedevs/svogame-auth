use std::path::{Path, PathBuf};
use std::sync::Arc;

use auth::app::config::{AppConfig, DatabaseConfig, DiscordConfig, S3Config};
use auth::app::router::build_router;
use auth::app::state::{AppState, SharedAppState};
use auth::entities::{
    AuditLog, AuditLogColumn, InventoryOperation, InventoryOperationColumn, Squad, User,
    WalletTransaction, WalletTransactionColumn,
};
use auth::services::db::{connect_db, run_migrations};
use aws_credential_types::Credentials;
use reqwest::Client;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter};
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use uuid::Uuid;

pub struct TestApp {
    pub address: String,
    pub client: Client,
    pub db: DatabaseConnection,
    _db_path: PathBuf,
    _server_task: tokio::task::JoinHandle<()>,
}

#[derive(Clone)]
pub struct IssuedUser {
    pub user_id: String,
    pub access_token: String,
}

impl TestApp {
    pub async fn spawn() -> Self {
        let db_path = test_db_path();
        let database = DatabaseConfig {
            db_url: format!("sqlite://{}?mode=rwc", normalize_sqlite_path(&db_path)),
        };
        let db = connect_db(&database).await.expect("connect test db");
        run_migrations(&db).await.expect("run migrations");

        let config = AppConfig {
            binding_address: "127.0.0.1:0".to_string(),
            discord: DiscordConfig {
                oauth2_url: "https://discord.test/oauth".to_string(),
                redirect_url: "http://localhost:5173/auth/callback".to_string(),
                client_id: "test-client".to_string(),
                client_secret: "test-secret".to_string(),
                required_scopes: vec!["identify".to_string()],
                discord_proxy: None,
            },
            database,
            s3: S3Config {
                endpoint: Some("http://127.0.0.1:9000".to_string()),
                region: "us-east-1".to_string(),
                bucket: "test-bucket".to_string(),
                access_key_id: "test".to_string(),
                secret_access_key: "test".to_string(),
                force_path_style: true,
            },
            pow_complexity: 1,
            jwt_secret: "test-jwt-secret".to_string(),
            gamervii_compat: None,
        };

        let state: SharedAppState = Arc::new(RwLock::new(AppState::new(
            config,
            db.clone(),
            test_s3_client().await,
        )));
        let app = build_router(state);
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind test port");
        let address = format!("http://{}", listener.local_addr().expect("local addr"));
        let server_task = tokio::spawn(async move {
            if let Err(error) = axum::serve(listener, app.into_make_service()).await {
                panic!("serve test app: {error}");
            }
        });
        let test_app = Self {
            address,
            client: Client::new(),
            db,
            _db_path: db_path,
            _server_task: server_task,
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
        self.send_with_retry(|| self.client.get(self.url(path))).await
    }

    pub async fn post_json(
        &self,
        path: &str,
        token: &str,
        body: serde_json::Value,
    ) -> reqwest::Response {
        self.send_with_retry(|| self.client.post(self.url(path)).bearer_auth(token).json(&body))
            .await
    }

    pub async fn patch_json(
        &self,
        path: &str,
        token: &str,
        body: serde_json::Value,
    ) -> reqwest::Response {
        self.send_with_retry(|| self.client.patch(self.url(path)).bearer_auth(token).json(&body))
            .await
    }

    pub async fn put_json(
        &self,
        path: &str,
        token: &str,
        body: serde_json::Value,
    ) -> reqwest::Response {
        self.send_with_retry(|| self.client.put(self.url(path)).bearer_auth(token).json(&body))
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
        self.send_with_retry(|| self.client.delete(self.url(path)).bearer_auth(token).json(&body))
            .await
    }

    pub async fn issue_invite(&self, leader: &IssuedUser, squad_id: &str, invited_user_id: &str) -> String {
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
            .post_json("/api/squads", &user.access_token, serde_json::json!({ "name": name }))
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
            .filter(auth::entities::SquadInviteColumn::SquadId.eq(
                Uuid::parse_str(squad_id).expect("uuid"),
            ))
            .count(&self.db)
            .await
            .expect("count invites")
    }

    pub async fn expire_invite(&self, invite_id: &str) {
        use auth::entities::SquadInviteActiveModel;
        use sea_orm::{ActiveModelTrait, Set};

        let invite = auth::entities::SquadInvite::find_by_id(Uuid::parse_str(invite_id).expect("uuid"))
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

    pub async fn post_without_auth(&self, path: &str, body: serde_json::Value) -> reqwest::Response {
        self.send_with_retry(|| self.client.post(self.url(path)).json(&body))
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

impl Drop for TestApp {
    fn drop(&mut self) {
        self._server_task.abort();
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

async fn test_s3_client() -> aws_sdk_s3::Client {
    let credentials = Credentials::new("test", "test", None, None, "tests");
    let shared_config = aws_config::defaults(aws_config::BehaviorVersion::latest())
        .credentials_provider(credentials)
        .region(aws_config::Region::new("us-east-1"))
        .load()
        .await;

    let config = aws_sdk_s3::config::Builder::from(&shared_config)
        .force_path_style(true)
        .endpoint_url("http://127.0.0.1:9000")
        .build();

    aws_sdk_s3::Client::from_conf(config)
}
