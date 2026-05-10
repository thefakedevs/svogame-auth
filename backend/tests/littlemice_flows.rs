mod common;

use auth::entities::{LittlemiceCheck, LittlemiceCheckActiveModel, LittlemiceCheckColumn};
use auth::services::littlemice;
use common::{MultipartPart, TestApp};
use sea_orm::ActiveModelTrait;
use sea_orm::ActiveValue::Set;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serial_test::serial;
use uuid::Uuid;

fn extract_push_token(push_url: &str) -> &str {
    push_url
        .rsplit('/')
        .next()
        .filter(|value| !value.is_empty())
        .expect("push token in url")
}

#[tokio::test]
#[serial]
async fn openapi_documents_all_littlemice_routes() {
    let app = TestApp::spawn().await;

    let openapi = app.get_json_value_without_auth("/api/openapi.json").await;
    let paths = openapi["paths"].as_object().expect("paths object");

    for path in [
        "/api/littlemice/checks",
        "/api/littlemice/checks/{check_id}",
        "/api/littlemice/checks/{check_id}/fail",
        "/api/littlemice/push/{push_token}",
        "/api/admin/littlemice/checks",
        "/api/admin/littlemice/checks/{check_id}",
        "/api/admin/littlemice/checks/{check_id}/screenshot",
        "/api/admin/littlemice/checks/{check_id}/log",
        "/api/admin/users/{player_uuid}/littlemice-checks",
    ] {
        assert!(paths.contains_key(path), "missing openapi path: {path}");
    }

    let tags = openapi["tags"].as_array().expect("tags array");
    assert!(tags.iter().any(|tag| tag["name"] == "littlemice"));
}

#[tokio::test]
#[serial]
async fn service_can_create_upload_and_admin_can_read_littlemice_check() {
    let app = TestApp::spawn().await;
    let admin = app.issue_user_token("LittlemiceAdmin", true, &[]).await;
    let player = app.issue_user_token("LittlemicePlayer", false, &[]).await;
    let service_token = app.create_service_token(&admin, "littlemice-game-1").await;

    let create = app
        .post_json(
            "/api/littlemice/checks",
            &service_token,
            serde_json::json!({ "playerUuid": &player.user_id }),
        )
        .await;
    assert!(
        create.status().is_success(),
        "{}",
        create.text().await.unwrap_or_default()
    );
    let created: serde_json::Value = create.json().await.expect("create check json");
    let check_id = created["id"].as_str().expect("check id");
    let push_url = created["pushUrl"].as_str().expect("push url");
    let push_path = push_url.strip_prefix(&app.address).expect("same host push url");

    let screenshot_bytes = b"fake-screenshot-binary".to_vec();
    let log_bytes = b"line1\nline2\n".to_vec();
    let push = app
        .post_multipart_without_auth_fields(
            push_path,
            vec![
                MultipartPart::File {
                    name: "screenshot".to_string(),
                    file_name: "screen.bin".to_string(),
                    content_type: "image/png".to_string(),
                    bytes: screenshot_bytes.clone(),
                },
                MultipartPart::File {
                    name: "log".to_string(),
                    file_name: "client.log".to_string(),
                    content_type: "text/plain".to_string(),
                    bytes: log_bytes.clone(),
                },
                MultipartPart::Text {
                    name: "clientInfo".to_string(),
                    value: "shaders: fancy\nresourcepacks: yes".to_string(),
                },
            ],
        )
        .await;
    assert!(
        push.status().is_success(),
        "{}",
        push.text().await.unwrap_or_default()
    );

    let status = app
        .get_json(&format!("/api/littlemice/checks/{check_id}"), &service_token)
        .await;
    assert!(status.status().is_success());
    let status_body: serde_json::Value = status.json().await.expect("status body");
    assert_eq!(status_body["status"], "passed");

    let admin_list = app
        .get_json("/api/admin/littlemice/checks", &admin.access_token)
        .await;
    assert!(admin_list.status().is_success());
    let admin_list_body: serde_json::Value = admin_list.json().await.expect("admin list");
    assert_eq!(admin_list_body["total"], 1);
    assert_eq!(admin_list_body["items"][0]["id"], check_id);

    let player_list = app
        .get_json(
            &format!("/api/admin/users/{}/littlemice-checks", player.user_id),
            &admin.access_token,
        )
        .await;
    assert!(player_list.status().is_success());
    let player_list_body: serde_json::Value = player_list.json().await.expect("player list");
    assert_eq!(player_list_body["total"], 1);
    assert_eq!(player_list_body["items"][0]["status"], "passed");

    let detail = app
        .get_json(
            &format!("/api/admin/littlemice/checks/{check_id}"),
            &admin.access_token,
        )
        .await;
    assert!(detail.status().is_success());
    let detail_body: serde_json::Value = detail.json().await.expect("detail body");
    assert_eq!(detail_body["status"], "passed");
    assert_eq!(detail_body["playerUuid"], player.user_id);
    assert_eq!(detail_body["serviceSystemName"], "littlemice-game-1");
    assert_eq!(detail_body["clientInfoText"], "shaders: fancy\nresourcepacks: yes");
    assert_eq!(detail_body["screenshotSizeBytes"], screenshot_bytes.len() as u64);
    assert_eq!(detail_body["logSizeBytes"], log_bytes.len() as u64);

    let screenshot = app
        .get_json(
            &format!("/api/admin/littlemice/checks/{check_id}/screenshot"),
            &admin.access_token,
        )
        .await;
    assert!(screenshot.status().is_success());
    assert_eq!(
        screenshot
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok()),
        Some("image/png")
    );
    assert_eq!(screenshot.bytes().await.expect("screenshot bytes"), screenshot_bytes);

    let log = app
        .get_json(
            &format!("/api/admin/littlemice/checks/{check_id}/log"),
            &admin.access_token,
        )
        .await;
    assert!(log.status().is_success());
    assert_eq!(
        log.headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok()),
        Some("text/plain")
    );
    assert_eq!(log.bytes().await.expect("log bytes"), log_bytes);
}

#[tokio::test]
#[serial]
async fn service_can_finish_check_with_client_error_and_stacktrace() {
    let app = TestApp::spawn().await;
    let admin = app.issue_user_token("LittlemiceAdmin2", true, &[]).await;
    let player = app.issue_user_token("LittlemicePlayer2", false, &[]).await;
    let service_token = app.create_service_token(&admin, "littlemice-game-2").await;

    let create = app
        .post_json(
            "/api/littlemice/checks",
            &service_token,
            serde_json::json!({ "playerUuid": &player.user_id }),
        )
        .await;
    let created: serde_json::Value = create.json().await.expect("create check");
    let check_id = created["id"].as_str().expect("check id");

    let fail = app
        .post_json(
            &format!("/api/littlemice/checks/{check_id}/fail"),
            &service_token,
            serde_json::json!({
                "errorMessage": "snapshot collector crashed",
                "stacktrace": "java.lang.IllegalStateException: boom\n\tat Snapshot.run(Snapshot.java:42)"
            }),
        )
        .await;
    assert!(
        fail.status().is_success(),
        "{}",
        fail.text().await.unwrap_or_default()
    );
    let fail_body: serde_json::Value = fail.json().await.expect("fail body");
    assert_eq!(fail_body["status"], "failed_client_error");
    assert_eq!(fail_body["failureReason"], "client_error");

    let detail = app
        .get_json(
            &format!("/api/admin/littlemice/checks/{check_id}"),
            &admin.access_token,
        )
        .await;
    assert!(detail.status().is_success());
    let detail_body: serde_json::Value = detail.json().await.expect("detail");
    let info = detail_body["clientInfoText"]
        .as_str()
        .expect("clientInfoText");
    assert!(info.contains("clientError: snapshot collector crashed"));
    assert!(info.contains("stacktrace:"));
    assert!(info.contains("IllegalStateException"));
}

#[tokio::test]
#[serial]
async fn user_token_cannot_use_service_only_littlemice_routes() {
    let app = TestApp::spawn().await;
    let user = app.issue_user_token("NotAService", false, &[]).await;
    let random_check_id = Uuid::new_v4();

    let create = app
        .post_json(
            "/api/littlemice/checks",
            &user.access_token,
            serde_json::json!({ "playerUuid": &user.user_id }),
        )
        .await;
    assert_eq!(create.status(), reqwest::StatusCode::FORBIDDEN);

    let fail = app
        .post_json(
            &format!("/api/littlemice/checks/{random_check_id}/fail"),
            &user.access_token,
            serde_json::json!({ "errorMessage": "boom" }),
        )
        .await;
    assert_eq!(fail.status(), reqwest::StatusCode::FORBIDDEN);
}

#[tokio::test]
#[serial]
async fn service_cannot_read_or_fail_checks_of_another_service() {
    let app = TestApp::spawn().await;
    let admin = app.issue_user_token("LittlemiceAdmin3", true, &[]).await;
    let player = app.issue_user_token("LittlemicePlayer3", false, &[]).await;
    let service_a = app.create_service_token(&admin, "littlemice-service-a").await;
    let service_b = app.create_service_token(&admin, "littlemice-service-b").await;

    let create = app
        .post_json(
            "/api/littlemice/checks",
            &service_a,
            serde_json::json!({ "playerUuid": &player.user_id }),
        )
        .await;
    let created: serde_json::Value = create.json().await.expect("create check");
    let check_id = created["id"].as_str().expect("check id");

    let status = app
        .get_json(&format!("/api/littlemice/checks/{check_id}"), &service_b)
        .await;
    assert_eq!(status.status(), reqwest::StatusCode::FORBIDDEN);

    let fail = app
        .post_json(
            &format!("/api/littlemice/checks/{check_id}/fail"),
            &service_b,
            serde_json::json!({ "errorMessage": "foreign service tried to fail" }),
        )
        .await;
    assert_eq!(fail.status(), reqwest::StatusCode::FORBIDDEN);
}

#[tokio::test]
#[serial]
async fn push_token_reports_not_found_and_gone_when_invalid_or_expired() {
    let app = TestApp::spawn().await;
    let admin = app.issue_user_token("LittlemiceAdmin4", true, &[]).await;
    let player = app.issue_user_token("LittlemicePlayer4", false, &[]).await;
    let service_token = app.create_service_token(&admin, "littlemice-game-4").await;

    let invalid = app
        .post_multipart_without_auth_fields(
            "/api/littlemice/push/lm_invalid",
            vec![MultipartPart::File {
                name: "screenshot".to_string(),
                file_name: "screen.bin".to_string(),
                content_type: "image/png".to_string(),
                bytes: b"fake".to_vec(),
            }],
        )
        .await;
    assert_eq!(invalid.status(), reqwest::StatusCode::NOT_FOUND);

    let create = app
        .post_json(
            "/api/littlemice/checks",
            &service_token,
            serde_json::json!({ "playerUuid": &player.user_id }),
        )
        .await;
    let created: serde_json::Value = create.json().await.expect("create");
    let check_id = created["id"].as_str().expect("check id");
    let push_token = extract_push_token(created["pushUrl"].as_str().expect("push url")).to_string();

    let model = LittlemiceCheck::find_by_id(Uuid::parse_str(check_id).expect("uuid"))
        .one(&app.db)
        .await
        .expect("load check")
        .expect("check exists");
    let mut active: LittlemiceCheckActiveModel = model.into();
    active.push_token_expires_at = Set(chrono::Utc::now() - chrono::Duration::seconds(1));
    active.update(&app.db).await.expect("expire check");

    let expired = app
        .post_multipart_without_auth_fields(
            &format!("/api/littlemice/push/{push_token}"),
            vec![MultipartPart::File {
                name: "screenshot".to_string(),
                file_name: "screen.bin".to_string(),
                content_type: "image/png".to_string(),
                bytes: b"fake".to_vec(),
            }],
        )
        .await;
    assert_eq!(expired.status(), reqwest::StatusCode::GONE);
}

#[tokio::test]
#[serial]
async fn expire_due_checks_marks_timeouts_and_cleanup_removes_old_entries() {
    let app = TestApp::spawn().await;
    let admin = app.issue_user_token("LittlemiceAdmin5", true, &[]).await;
    let player = app.issue_user_token("LittlemicePlayer5", false, &[]).await;
    let service_token = app.create_service_token(&admin, "littlemice-game-5").await;

    let create = app
        .post_json(
            "/api/littlemice/checks",
            &service_token,
            serde_json::json!({ "playerUuid": &player.user_id }),
        )
        .await;
    let created: serde_json::Value = create.json().await.expect("create");
    let check_id = created["id"].as_str().expect("check id");

    let model = LittlemiceCheck::find_by_id(Uuid::parse_str(check_id).expect("uuid"))
        .one(&app.db)
        .await
        .expect("load check")
        .expect("check exists");
    let mut active: LittlemiceCheckActiveModel = model.into();
    active.push_token_expires_at = Set(chrono::Utc::now() - chrono::Duration::seconds(1));
    active.update(&app.db).await.expect("expire check");

    let expired = littlemice::expire_due_checks(&app.db, &app.config)
        .await
        .expect("expire due checks");
    assert_eq!(expired.len(), 1);
    assert_eq!(expired[0].status, "failed_timeout");
    assert_eq!(expired[0].failure_reason.as_deref(), Some("push_timeout"));

    let refreshed = LittlemiceCheck::find_by_id(Uuid::parse_str(check_id).expect("uuid"))
        .one(&app.db)
        .await
        .expect("reload check")
        .expect("check exists");
    let mut active: LittlemiceCheckActiveModel = refreshed.into();
    active.created_at = Set(chrono::Utc::now() - chrono::Duration::days(31));
    active.updated_at = Set(chrono::Utc::now() - chrono::Duration::days(31));
    active.update(&app.db).await.expect("age check");

    let removed = littlemice::cleanup_old_checks(
        &app.db,
        &common::test_s3_client(
            app.config
                .s3
                .endpoint
                .as_deref()
                .expect("s3 endpoint"),
        )
        .await,
        &app.config,
    )
    .await
    .expect("cleanup old checks");
    assert_eq!(removed, 1);

    let gone = LittlemiceCheck::find()
        .filter(LittlemiceCheckColumn::Id.eq(Uuid::parse_str(check_id).expect("uuid")))
        .one(&app.db)
        .await
        .expect("query cleaned check");
    assert!(gone.is_none());
}
