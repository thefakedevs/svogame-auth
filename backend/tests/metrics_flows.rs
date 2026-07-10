mod common;

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use auth::entities::{
    MetricDiscordOutbox, MetricDiscordOutboxActiveModel, MetricIngestion, MetricIngestionColumn,
};
use auth::services::metrics::discord::process_next_delivery;
use axum::extract::State;
use axum::response::IntoResponse;
use axum::routing::post;
use axum::{Json, Router};
use chrono::{Duration, Utc};
use common::TestApp;
use reqwest::StatusCode;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter,
};
use serde_json::Value;
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use uuid::Uuid;

const GAME: &str = "10000000-0000-0000-0000-000000000001";
const MULTIPART_GAME: &str = "10000000-0000-0000-0000-000000000002";
const INCOMPLETE_GAME: &str = "10000000-0000-0000-0000-000000000003";
const INVALID_GAME: &str = "10000000-0000-0000-0000-000000000004";
const A: &str = "20000000-0000-0000-0000-000000000001";
const B: &str = "20000000-0000-0000-0000-000000000002";
const C: &str = "20000000-0000-0000-0000-000000000003";
const D: &str = "20000000-0000-0000-0000-000000000004";
const SECRET: &str = "test-metrics-secret";

#[tokio::test]
async fn metrics_pipeline_is_authenticated_transactional_idempotent_and_queryable() {
    let app = TestApp::spawn().await;
    let timeline = complete_timeline(GAME);

    let unauthorized = app
        .client
        .post(app.url(&format!("/api/metrics/timeline/{GAME}")))
        .header(reqwest::header::CONTENT_TYPE, "application/x-ndjson")
        .body(timeline.clone())
        .send()
        .await
        .expect("send unauthorized ingestion");
    assert_eq!(unauthorized.status(), StatusCode::UNAUTHORIZED);

    let created = upload_raw(&app, GAME, &timeline, SECRET).await;
    assert_eq!(created.status(), StatusCode::CREATED);
    let created_body: Value = created.json().await.expect("created JSON");
    assert_eq!(created_body["status"], "processed");
    assert_eq!(created_body["playersProcessed"], 3);
    assert!(created_body.get("warnings").is_none());

    let duplicate = upload_raw(&app, GAME, &timeline, SECRET).await;
    assert_eq!(duplicate.status(), StatusCode::OK);
    let duplicate_body: Value = duplicate.json().await.expect("duplicate JSON");
    assert_eq!(duplicate_body["status"], "duplicate");

    let conflict = upload_raw(&app, GAME, &format!("{timeline}\n"), SECRET).await;
    assert_eq!(conflict.status(), StatusCode::CONFLICT);
    assert_eq!(
        MetricIngestion::find()
            .filter(
                MetricIngestionColumn::GameId
                    .eq(Uuid::parse_str(GAME).expect("valid fixture game UUID")),
            )
            .count(&app.db)
            .await
            .expect("count ingestion"),
        1
    );

    let match_response = app
        .get_without_auth(&format!("/api/metrics/matches/{GAME}"))
        .await;
    assert_eq!(match_response.status(), StatusCode::OK);
    let match_body: Value = match_response.json().await.expect("match JSON");
    assert_eq!(match_body["map"], "integration-map");
    assert_eq!(match_body["durationMs"], 60_000);
    assert!(match_body.get("warnings").is_none());
    assert_eq!(match_body["players"].as_array().map(Vec::len), Some(3));
    let player_a = match_body["players"]
        .as_array()
        .and_then(|players| players.iter().find(|player| player["playerId"] == A))
        .expect("player A in match");
    assert_eq!(player_a["kills"], 1);
    assert_eq!(player_a["vehicleDestructions"], 1);
    let player_c = match_body["players"]
        .as_array()
        .and_then(|players| players.iter().find(|player| player["playerId"] == C))
        .expect("player C in match");
    assert_eq!(player_c["assists"], 1);

    let exact_player = app
        .get_without_auth(&format!("/api/metrics/matches/{GAME}/players/{A}"))
        .await;
    assert_eq!(exact_player.status(), StatusCode::OK);

    let profile = app
        .get_without_auth(&format!("/api/metrics/players/{A}"))
        .await;
    assert_eq!(profile.status(), StatusCode::OK);
    let profile_body: Value = profile.json().await.expect("profile JSON");
    assert_eq!(profile_body["nickname"], "Alpha");

    let stats = app
        .get_without_auth(&format!(
            "/api/metrics/players/{A}/stats?from=2023-01-01T00%3A00%3A00Z&to=2030-01-01T00%3A00%3A00Z"
        ))
        .await;
    assert_eq!(stats.status(), StatusCode::OK);
    let stats_body: Value = stats.json().await.expect("stats JSON");
    assert_eq!(stats_body["matchesPlayed"], 1);
    assert_eq!(stats_body["wins"], 1);

    let matches = app
        .get_without_auth(&format!(
            "/api/metrics/players/{A}/matches?limit=1&offset=0"
        ))
        .await;
    assert_eq!(matches.status(), StatusCode::OK);
    let matches_body: Value = matches.json().await.expect("matches JSON");
    assert_eq!(matches_body["pagination"]["total"], 1);

    let leaderboard_page_1 = app
        .get_without_auth("/api/metrics/leaderboard?metric=kills&limit=1&offset=0")
        .await;
    assert_eq!(leaderboard_page_1.status(), StatusCode::OK);
    let page_1: Value = leaderboard_page_1.json().await.expect("leaderboard page 1");
    assert_eq!(page_1["entries"][0]["rank"], 1);
    assert_eq!(page_1["entries"][0]["playerId"], A);
    assert_eq!(page_1["pagination"]["total"], 3);
    let leaderboard_page_2 = app
        .get_without_auth("/api/metrics/leaderboard?metric=kills&limit=1&offset=1")
        .await;
    let page_2: Value = leaderboard_page_2.json().await.expect("leaderboard page 2");
    assert_eq!(page_2["entries"][0]["rank"], 2);
    assert_eq!(page_2["entries"][0]["playerId"], C);

    let outbox =
        MetricDiscordOutbox::find_by_id(Uuid::parse_str(GAME).expect("valid fixture game UUID"))
            .one(&app.db)
            .await
            .expect("query outbox")
            .expect("outbox row");
    assert_eq!(outbox.status, "pending");
    assert!(
        !process_next_delivery(&app.db, &app.config.discord, &app.config.metrics)
            .await
            .expect("disabled Discord worker")
    );
    let (discord_api_url, discord_mock, _discord_task) = spawn_discord_mock().await;
    let mut discord_config = app.config.discord.clone();
    discord_config.api_base_url = discord_api_url;
    discord_config.bot_token = Some("test-bot-token".to_string());
    let mut metrics_config = app.config.metrics.clone();
    metrics_config.discord_channel_id = Some("metrics-channel".to_string());
    assert!(
        process_next_delivery(&app.db, &discord_config, &metrics_config)
            .await
            .expect("first Discord delivery attempt")
    );
    let first_attempt =
        MetricDiscordOutbox::find_by_id(Uuid::parse_str(GAME).expect("valid fixture game UUID"))
            .one(&app.db)
            .await
            .expect("query first Discord attempt")
            .expect("first Discord outbox row");
    assert_eq!(first_attempt.status, "retry");
    assert_eq!(first_attempt.attempt_count, 1);
    let mut due: MetricDiscordOutboxActiveModel = first_attempt.into();
    due.next_attempt_at = Set(Utc::now() - Duration::seconds(1));
    due.update(&app.db).await.expect("make Discord retry due");
    assert!(
        process_next_delivery(&app.db, &discord_config, &metrics_config)
            .await
            .expect("second Discord delivery attempt")
    );
    let delivered =
        MetricDiscordOutbox::find_by_id(Uuid::parse_str(GAME).expect("valid fixture game UUID"))
            .one(&app.db)
            .await
            .expect("query delivered Discord row")
            .expect("delivered Discord outbox row");
    assert_eq!(delivered.status, "delivered");
    assert_eq!(delivered.attempt_count, 2);
    assert_eq!(delivered.discord_message_id.as_deref(), Some("message-1"));
    let payloads = discord_mock.payloads.lock().await;
    assert_eq!(payloads.len(), 2);
    assert_eq!(payloads[0]["nonce"], payloads[1]["nonce"]);
    assert_eq!(payloads[0]["enforce_nonce"], true);
    let embeds = payloads[1]["embeds"].as_array().expect("Discord embeds");
    assert_eq!(embeds.len(), 2);
    assert_eq!(embeds[0]["title"], "🎮 Статистика игры");
    assert_eq!(embeds[0]["color"], 3_447_003);
    assert_eq!(embeds[0]["fields"].as_array().map(Vec::len), Some(3));
    assert_eq!(embeds[0]["fields"][0]["name"], "🏆 🔴 Атака");
    assert_eq!(embeds[0]["fields"][1]["name"], "🔵 Защита");
    assert_eq!(embeds[0]["fields"][2]["name"], "📊 Особые достижения");
    assert_eq!(embeds[1]["color"], 0x005ed5);
    assert_eq!(
        embeds[1]["author"]["name"],
        "Спонсор сервера: lifehosting.pro"
    );
    drop(payloads);

    let multipart = app
        .post_multipart(
            &format!("/api/metrics/timeline/{MULTIPART_GAME}"),
            SECRET,
            "timeline.ndjson",
            "application/x-ndjson",
            complete_timeline(MULTIPART_GAME).into_bytes(),
        )
        .await;
    assert_eq!(multipart.status(), StatusCode::CREATED);

    let incomplete = upload_raw(
        &app,
        INCOMPLETE_GAME,
        &incomplete_timeline(INCOMPLETE_GAME),
        SECRET,
    )
    .await;
    assert_eq!(incomplete.status(), StatusCode::CREATED);
    let incomplete_body: Value = incomplete.json().await.expect("incomplete JSON");
    assert_eq!(incomplete_body["status"], "incomplete");
    let incomplete_stats = app
        .get_without_auth(&format!("/api/metrics/players/{D}/stats"))
        .await;
    let incomplete_stats: Value = incomplete_stats
        .json()
        .await
        .expect("incomplete player stats JSON");
    assert_eq!(incomplete_stats["matchesPlayed"], 0);

    let invalid = upload_raw(
        &app,
        INVALID_GAME,
        &format!(
            "{}\nnot-json",
            game_started(INVALID_GAME, 1_700_000_000_000)
        ),
        SECRET,
    )
    .await;
    assert_eq!(invalid.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert!(
        MetricIngestion::find()
            .filter(
                MetricIngestionColumn::GameId
                    .eq(Uuid::parse_str(INVALID_GAME).expect("valid fixture game UUID")),
            )
            .one(&app.db)
            .await
            .expect("query invalid ingestion")
            .is_none()
    );

    let invalid_metric = app
        .get_without_auth("/api/metrics/leaderboard?metric=unknown")
        .await;
    assert_eq!(invalid_metric.status(), StatusCode::BAD_REQUEST);
    let invalid_period = app
        .get_without_auth(
            "/api/metrics/leaderboard?from=2030-01-01T00%3A00%3A00Z&to=2020-01-01T00%3A00%3A00Z",
        )
        .await;
    assert_eq!(invalid_period.status(), StatusCode::BAD_REQUEST);
}

async fn upload_raw(app: &TestApp, game_id: &str, body: &str, secret: &str) -> reqwest::Response {
    app.client
        .post(app.url(&format!("/api/metrics/timeline/{game_id}")))
        .header(reqwest::header::CONTENT_TYPE, "application/x-ndjson")
        .header(reqwest::header::AUTHORIZATION, secret)
        .body(body.to_string())
        .send()
        .await
        .expect("send metric timeline")
}

fn complete_timeline(game_id: &str) -> String {
    let start = 1_700_000_000_000_i64;
    [
        game_started(game_id, start),
        join(A, "Alpha", "Attack", start + 1),
        join(B, "Bravo", "Defense", start + 2),
        join(C, "Charlie", "Attack", start + 3),
        damage(B, A, 70.0, start + 10),
        damage(B, C, 30.0, start + 11),
        format!(
            r#"{{"type":"player_death","timestampMs":{},"victimId":"{B}","killerId":"{A}","totalDamage":100.0,"source":{{"type":"tacz:gun","weaponId":"tacz:m4a1"}}}}"#,
            start + 12
        ),
        format!(
            r#"{{"type":"vehicle_destroyed","timestampMs":{},"vehicleId":"vvp:car","vehicleEntityId":"vehicle-1","vehicleType":"CAR","vehicleMaxHealth":100.0,"attackerId":"{A}","vehicleOwnerId":"{B}","finalDamage":100.0,"source":{{"type":"gunfire","weaponId":"gun"}},"isTeamKill":false}}"#,
            start + 13
        ),
        format!(
            r#"{{"type":"game_ended","timestampMs":{},"gameId":"{game_id}","winningTeam":"Attack"}}"#,
            start + 60_000
        ),
    ]
    .join("\n")
}

fn incomplete_timeline(game_id: &str) -> String {
    [
        game_started(game_id, 1_700_100_000_000),
        join(D, "Delta", "Attack", 1_700_100_000_001),
    ]
    .join("\n")
}

fn game_started(game_id: &str, timestamp: i64) -> String {
    format!(
        r#"{{"type":"game_started","timestampMs":{timestamp},"gameId":"{game_id}","map":"integration-map"}}"#
    )
}

fn join(player_id: &str, nickname: &str, team: &str, timestamp: i64) -> String {
    format!(
        r#"{{"type":"player_joined","timestampMs":{timestamp},"playerId":"{player_id}","nickname":"{nickname}","team":"{team}"}}"#
    )
}

fn damage(victim_id: &str, attacker_id: &str, damage: f64, timestamp: i64) -> String {
    format!(
        r#"{{"type":"player_damage","timestampMs":{timestamp},"victimId":"{victim_id}","attackerId":"{attacker_id}","damage":{damage},"source":{{"type":"tacz:gun","weaponId":"tacz:m4a1"}},"headshot":false}}"#
    )
}

#[derive(Clone, Default)]
struct DiscordMock {
    calls: Arc<AtomicUsize>,
    payloads: Arc<Mutex<Vec<Value>>>,
}

async fn spawn_discord_mock() -> (String, DiscordMock, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind Discord mock");
    let address = listener.local_addr().expect("Discord mock address");
    let state = DiscordMock::default();
    let app = Router::new()
        .route("/channels/{channel_id}/messages", post(discord_message))
        .with_state(state.clone());
    let task = tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .expect("serve Discord mock");
    });
    (format!("http://{address}"), state, task)
}

async fn discord_message(
    State(state): State<DiscordMock>,
    Json(payload): Json<Value>,
) -> axum::response::Response {
    state.payloads.lock().await.push(payload);
    let attempt = state.calls.fetch_add(1, Ordering::SeqCst);
    if attempt == 0 {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({"message": "temporary failure"})),
        )
            .into_response()
    } else {
        (StatusCode::OK, Json(serde_json::json!({"id": "message-1"}))).into_response()
    }
}
