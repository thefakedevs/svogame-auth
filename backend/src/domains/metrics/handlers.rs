use std::collections::BTreeMap;
use std::time::Instant;

use axum::Json;
use axum::extract::{FromRequest, Multipart, Path, Query, Request, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use chrono::{DateTime, Duration, Utc};
use hmac::{Hmac, Mac};
use http_body_util::BodyExt;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use uuid::Uuid;

use crate::app::http::HttpError;
use crate::app::state::AppStateExtractor;
use crate::entities::{MetricMatchModel, MetricMatchPlayerModel, MetricPlayerMatchStatModel};
use crate::services::metrics::aggregation::PlayerStats;
use crate::services::metrics::persistence::{PersistenceError, merge_stats, persist_timeline};
use crate::services::metrics::read::{
    LeaderboardMetric, MatchBundle, PlayerSummary, TimeRange, get_match as load_match,
    get_player_matches as load_player_matches, get_player_profile, get_player_summary, leaderboard,
};
use crate::services::metrics::{ParsedTimeline, TimelineStream, TimelineStreamError};

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct IngestTimelineResponse {
    pub game_id: Uuid,
    pub status: &'static str,
    pub events_processed: i64,
    pub players_processed: i64,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MatchResponse {
    pub game_id: Uuid,
    pub map: String,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<i64>,
    pub winning_team: Option<String>,
    pub status: String,
    pub events_processed: i64,
    pub player_count: i64,
    pub teams: Vec<TeamSummaryResponse>,
    pub players: Vec<MatchPlayerResponse>,
}

#[derive(Clone, Debug, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MatchPlayerResponse {
    pub player_id: Uuid,
    pub nickname: String,
    pub initial_team: Option<String>,
    pub final_team: Option<String>,
    pub changed_team: bool,
    pub team_changes: i64,
    pub left_count: i64,
    pub time_in_game_ms: i64,
    pub kd: f64,
    pub kda: f64,
    pub damage_per_minute: f64,
    #[serde(flatten)]
    pub stats: PlayerStats,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TeamSummaryResponse {
    pub team: String,
    pub players: i64,
    pub kills: i64,
    pub deaths: i64,
    pub assists: i64,
    pub vehicle_destructions: i64,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PlayerProfileResponse {
    pub player_id: Uuid,
    pub nickname: String,
    pub first_seen_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
    pub nickname_history: Vec<NicknameResponse>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct NicknameResponse {
    pub nickname: String,
    pub first_seen_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, utoipa::ToSchema, utoipa::IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct PeriodQuery {
    pub from: Option<String>,
    pub to: Option<String>,
    pub period: Option<String>,
}

#[derive(Debug, Deserialize, utoipa::ToSchema, utoipa::IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct PlayerMatchesQuery {
    pub from: Option<String>,
    pub to: Option<String>,
    pub period: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PlayerMatchesResponse {
    pub player_id: Uuid,
    pub period: PeriodResponse,
    pub pagination: PaginationResponse,
    pub matches: Vec<MatchListItemResponse>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MatchListItemResponse {
    pub game_id: Uuid,
    pub map: String,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<i64>,
    pub winning_team: Option<String>,
    pub status: String,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PlayerStatsResponse {
    pub player_id: Uuid,
    pub nickname: String,
    pub period: PeriodResponse,
    pub time_in_game_ms: i64,
    pub kd: f64,
    pub kda: f64,
    pub damage_per_minute: f64,
    pub win_rate: f64,
    #[serde(flatten)]
    pub stats: PlayerStats,
}

#[derive(Debug, Deserialize, utoipa::ToSchema, utoipa::IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct LeaderboardQuery {
    pub metric: Option<String>,
    pub period: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub min_matches: Option<i64>,
    pub sort: Option<String>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LeaderboardResponse {
    pub metric: String,
    pub period: PeriodResponse,
    pub pagination: PaginationResponse,
    pub entries: Vec<LeaderboardEntryResponse>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LeaderboardEntryResponse {
    pub rank: i64,
    pub player_id: Uuid,
    pub nickname: String,
    pub metric_value: f64,
    pub matches_played: i64,
    pub kills: i64,
    pub deaths: i64,
    pub assists: i64,
    pub vehicle_destructions: i64,
    pub time_in_game_ms: i64,
    pub kd: f64,
    pub kda: f64,
    pub damage_dealt: f64,
    pub damage_per_minute: f64,
    pub win_rate: f64,
    pub headshots: i64,
}

#[derive(Clone, Debug, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PeriodResponse {
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    pub name: String,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PaginationResponse {
    pub limit: i64,
    pub offset: i64,
    pub total: i64,
}

#[utoipa::path(
    post,
    path = "/api/metrics/timeline/{game_id}",
    params(("game_id" = Uuid, Path, description = "Match UUID")),
    responses(
        (status = 201, description = "Timeline processed transactionally", body = IngestTimelineResponse),
        (status = 200, description = "Identical timeline was already processed", body = IngestTimelineResponse),
        (status = 401, description = "Missing or invalid ingest secret"),
        (status = 409, description = "Different content already exists for the game"),
        (status = 413, description = "Upload exceeds configured size"),
        (status = 422, description = "Invalid timeline")
    ),
    security(("metrics_ingest_secret" = [])),
    tag = "metrics"
)]
pub async fn ingest_timeline(
    State(state): AppStateExtractor,
    Path(game_id): Path<String>,
    headers: HeaderMap,
    request: Request,
) -> Result<Response, Response> {
    let game_id = parse_uuid(&game_id, "game ID").map_err(IntoResponse::into_response)?;
    let (metrics_config, db) = {
        let state = state.read().await;
        (state.config.metrics.clone(), state.db.clone())
    };
    authorize_ingestion(&headers, metrics_config.ingest_secret.as_deref())
        .map_err(IntoResponse::into_response)?;
    validate_upload_headers(&headers, metrics_config.upload_max_bytes)
        .map_err(IntoResponse::into_response)?;

    let started = Instant::now();
    tracing::info!(
        action = "metrics.ingestion.started",
        game_id = %game_id,
        content_type = headers
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or("unknown"),
        "Metric timeline ingestion started"
    );
    let parse_started = Instant::now();
    let parsed = tokio::time::timeout(
        std::time::Duration::from_secs(metrics_config.upload_timeout_seconds),
        read_timeline(
            request,
            &state,
            &headers,
            game_id,
            metrics_config.upload_max_bytes,
        ),
    )
    .await
    .map_err(|_| {
        HttpError::new(StatusCode::REQUEST_TIMEOUT, "Timeline upload timed out").into_response()
    })?
    .map_err(|error| map_stream_error(error).into_response())?;
    let parsing_ms = parse_started.elapsed().as_millis() as u64;
    let persistence_started = Instant::now();
    let persisted = persist_timeline(&db, &parsed, &metrics_config.public_base_url)
        .await
        .map_err(|error| map_persistence_error(error, game_id).into_response())?;
    let persistence_ms = persistence_started.elapsed().as_millis() as u64;
    tracing::info!(
        action = "metrics.ingestion.completed",
        game_id = %game_id,
        status = persisted.status,
        bytes = parsed.byte_count,
        lines = parsed.line_count,
        events = parsed.aggregate.event_count,
        players = parsed.aggregate.players.len(),
        parsing_ms,
        persistence_ms,
        total_ms = started.elapsed().as_millis() as u64,
        event_counts = ?parsed.aggregate.event_counts,
        warnings = parsed.aggregate.warnings.len(),
        "Metric timeline ingestion completed"
    );
    let status = if persisted.status == "duplicate" {
        StatusCode::OK
    } else {
        StatusCode::CREATED
    };
    Ok((
        status,
        Json(IngestTimelineResponse {
            game_id: persisted.game_id,
            status: persisted.status,
            events_processed: persisted.events_processed,
            players_processed: persisted.players_processed,
        }),
    )
        .into_response())
}

#[utoipa::path(
    get,
    path = "/api/metrics/matches/{game_id}",
    params(("game_id" = Uuid, Path)),
    responses((status = 200, body = MatchResponse), (status = 404, description = "Match not found")),
    tag = "metrics"
)]
pub async fn get_match(
    State(state): AppStateExtractor,
    Path(game_id): Path<String>,
) -> Result<Json<MatchResponse>, Response> {
    let game_id = parse_uuid(&game_id, "game ID").map_err(IntoResponse::into_response)?;
    let db = state.read().await.db.clone();
    let bundle = load_match(&db, game_id)
        .await
        .map_err(database_error)?
        .ok_or_else(|| HttpError::not_found("Match not found").into_response())?;
    Ok(Json(match_response(bundle)))
}

#[utoipa::path(
    get,
    path = "/api/metrics/matches/{game_id}/players",
    params(("game_id" = Uuid, Path)),
    responses((status = 200, body = [MatchPlayerResponse]), (status = 404, description = "Match not found")),
    tag = "metrics"
)]
pub async fn get_match_players(
    State(state): AppStateExtractor,
    Path(game_id): Path<String>,
) -> Result<Json<Vec<MatchPlayerResponse>>, Response> {
    let game_id = parse_uuid(&game_id, "game ID").map_err(IntoResponse::into_response)?;
    let db = state.read().await.db.clone();
    let bundle = load_match(&db, game_id)
        .await
        .map_err(database_error)?
        .ok_or_else(|| HttpError::not_found("Match not found").into_response())?;
    Ok(Json(map_match_players(&bundle.players)))
}

#[utoipa::path(
    get,
    path = "/api/metrics/matches/{game_id}/players/{player_id}",
    params(("game_id" = Uuid, Path), ("player_id" = Uuid, Path)),
    responses((status = 200, body = MatchPlayerResponse), (status = 404, description = "Match or player not found")),
    tag = "metrics"
)]
pub async fn get_match_player(
    State(state): AppStateExtractor,
    Path((game_id, player_id)): Path<(String, String)>,
) -> Result<Json<MatchPlayerResponse>, Response> {
    let game_id = parse_uuid(&game_id, "game ID").map_err(IntoResponse::into_response)?;
    let player_id = parse_uuid(&player_id, "player ID").map_err(IntoResponse::into_response)?;
    let db = state.read().await.db.clone();
    let bundle = load_match(&db, game_id)
        .await
        .map_err(database_error)?
        .ok_or_else(|| HttpError::not_found("Match not found").into_response())?;
    let player = bundle
        .players
        .iter()
        .find(|(player, _)| player.player_id == player_id)
        .map(|(player, stats)| match_player_response(player, stats))
        .ok_or_else(|| {
            HttpError::not_found("Player is not present in this match").into_response()
        })?;
    Ok(Json(player))
}

#[utoipa::path(
    get,
    path = "/api/metrics/players/{player_id}",
    params(("player_id" = Uuid, Path)),
    responses((status = 200, body = PlayerProfileResponse), (status = 404, description = "Player not found")),
    tag = "metrics"
)]
pub async fn get_player(
    State(state): AppStateExtractor,
    Path(player_id): Path<String>,
) -> Result<Json<PlayerProfileResponse>, Response> {
    let player_id = parse_uuid(&player_id, "player ID").map_err(IntoResponse::into_response)?;
    let db = state.read().await.db.clone();
    let profile = get_player_profile(&db, player_id)
        .await
        .map_err(database_error)?
        .ok_or_else(|| HttpError::not_found("Player not found").into_response())?;
    Ok(Json(PlayerProfileResponse {
        player_id: profile.player.player_id,
        nickname: profile.player.last_nickname,
        first_seen_at: profile.player.first_seen_at,
        last_seen_at: profile.player.last_seen_at,
        nickname_history: profile
            .nicknames
            .into_iter()
            .map(|nickname| NicknameResponse {
                nickname: nickname.nickname,
                first_seen_at: nickname.first_seen_at,
                last_seen_at: nickname.last_seen_at,
            })
            .collect(),
    }))
}

#[utoipa::path(
    get,
    path = "/api/metrics/players/{player_id}/matches",
    params(("player_id" = Uuid, Path), PlayerMatchesQuery),
    responses((status = 200, body = PlayerMatchesResponse), (status = 404, description = "Player not found")),
    tag = "metrics"
)]
pub async fn get_player_matches(
    State(state): AppStateExtractor,
    Path(player_id): Path<String>,
    Query(query): Query<PlayerMatchesQuery>,
) -> Result<Json<PlayerMatchesResponse>, Response> {
    let player_id = parse_uuid(&player_id, "player ID").map_err(IntoResponse::into_response)?;
    let (range, period) =
        parse_period(query.from, query.to, query.period).map_err(IntoResponse::into_response)?;
    let (limit, offset) =
        pagination(query.limit, query.offset).map_err(IntoResponse::into_response)?;
    let db = state.read().await.db.clone();
    if get_player_profile(&db, player_id)
        .await
        .map_err(database_error)?
        .is_none()
    {
        return Err(HttpError::not_found("Player not found").into_response());
    }
    let matches = load_player_matches(&db, player_id, &range)
        .await
        .map_err(database_error)?;
    let total = matches.len() as i64;
    let page = matches
        .into_iter()
        .skip(offset as usize)
        .take(limit as usize)
        .map(match_list_item)
        .collect();
    Ok(Json(PlayerMatchesResponse {
        player_id,
        period,
        pagination: PaginationResponse {
            limit,
            offset,
            total,
        },
        matches: page,
    }))
}

#[utoipa::path(
    get,
    path = "/api/metrics/players/{player_id}/stats",
    params(("player_id" = Uuid, Path), PeriodQuery),
    responses((status = 200, body = PlayerStatsResponse), (status = 404, description = "Player not found")),
    tag = "metrics"
)]
pub async fn get_player_stats(
    State(state): AppStateExtractor,
    Path(player_id): Path<String>,
    Query(query): Query<PeriodQuery>,
) -> Result<Json<PlayerStatsResponse>, Response> {
    let player_id = parse_uuid(&player_id, "player ID").map_err(IntoResponse::into_response)?;
    let (range, period) =
        parse_period(query.from, query.to, query.period).map_err(IntoResponse::into_response)?;
    let db = state.read().await.db.clone();
    let profile = get_player_profile(&db, player_id)
        .await
        .map_err(database_error)?
        .ok_or_else(|| HttpError::not_found("Player not found").into_response())?;
    let summary = get_player_summary(&db, player_id, &range)
        .await
        .map_err(database_error)?;
    Ok(Json(player_stats_response(
        player_id,
        profile.player.last_nickname,
        period,
        summary,
    )))
}

#[utoipa::path(
    get,
    path = "/api/metrics/leaderboard",
    params(LeaderboardQuery),
    responses((status = 200, body = LeaderboardResponse), (status = 400, description = "Invalid metric, period or pagination")),
    tag = "metrics"
)]
pub async fn get_leaderboard(
    State(state): AppStateExtractor,
    Query(query): Query<LeaderboardQuery>,
) -> Result<Json<LeaderboardResponse>, Response> {
    let metric_name = query.metric.unwrap_or_else(|| "kills".to_string());
    let metric = LeaderboardMetric::parse(&metric_name)
        .ok_or_else(|| HttpError::bad_request("Unknown leaderboard metric").into_response())?;
    let descending = match query.sort.as_deref().unwrap_or("desc") {
        "desc" => true,
        "asc" => false,
        _ => return Err(HttpError::bad_request("sort must be asc or desc").into_response()),
    };
    let min_matches = query.min_matches.unwrap_or(1);
    if min_matches < 0 {
        return Err(HttpError::bad_request("minMatches must be non-negative").into_response());
    }
    let (limit, offset) =
        pagination(query.limit, query.offset).map_err(IntoResponse::into_response)?;
    let (range, period) =
        parse_period(query.from, query.to, query.period).map_err(IntoResponse::into_response)?;
    let db = state.read().await.db.clone();
    let rows = leaderboard(&db, &range, metric, min_matches, descending)
        .await
        .map_err(database_error)?;
    let total = rows.len() as i64;
    let entries = rows
        .into_iter()
        .skip(offset as usize)
        .take(limit as usize)
        .enumerate()
        .map(|(index, row)| {
            let stats = &row.summary.stats;
            LeaderboardEntryResponse {
                rank: offset + index as i64 + 1,
                player_id: row.player_id,
                nickname: row.nickname,
                metric_value: round(row.metric_value, 3),
                matches_played: stats.matches_played,
                kills: stats.kills,
                deaths: stats.deaths,
                assists: stats.assists,
                vehicle_destructions: stats.vehicle_destructions,
                time_in_game_ms: row.summary.time_in_game_ms,
                kd: ratio(stats.kills, stats.deaths),
                kda: ratio(stats.kills + stats.assists, stats.deaths),
                damage_dealt: round(stats.damage_dealt, 3),
                damage_per_minute: damage_per_minute(
                    stats.damage_dealt,
                    row.summary.time_in_game_ms,
                ),
                win_rate: win_rate(stats),
                headshots: stats.headshots,
            }
        })
        .collect();
    Ok(Json(LeaderboardResponse {
        metric: metric.as_str().to_string(),
        period,
        pagination: PaginationResponse {
            limit,
            offset,
            total,
        },
        entries,
    }))
}

async fn read_timeline(
    request: Request,
    state: &crate::app::state::SharedAppState,
    headers: &HeaderMap,
    game_id: Uuid,
    max_bytes: usize,
) -> Result<ParsedTimeline, TimelineStreamError> {
    let content_type = headers
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();
    let mut stream = TimelineStream::new(game_id, max_bytes);
    if content_type.starts_with("multipart/form-data") {
        let mut multipart = Multipart::from_request(request, state)
            .await
            .map_err(|error| stream_transport_error(format!("invalid multipart body: {error}")))?;
        let mut found_file = false;
        while let Some(mut field) = multipart
            .next_field()
            .await
            .map_err(|error| stream_transport_error(format!("invalid multipart field: {error}")))?
        {
            let name = field.name().unwrap_or_default();
            let is_file = field.file_name().is_some()
                || matches!(name, "file" | "timeline")
                || field
                    .content_type()
                    .is_some_and(|value| value.contains("ndjson"));
            if !found_file && is_file {
                found_file = true;
                while let Some(chunk) = field.chunk().await.map_err(|error| {
                    stream_transport_error(format!("failed to read multipart file: {error}"))
                })? {
                    stream.push_chunk(&chunk)?;
                }
            }
        }
        if !found_file {
            return Err(stream_transport_error(
                "multipart body does not contain a file".to_string(),
            ));
        }
    } else {
        let mut body = request.into_body();
        while let Some(frame) = body.frame().await {
            let frame = frame
                .map_err(|error| stream_transport_error(format!("failed to read body: {error}")))?;
            if let Some(data) = frame.data_ref() {
                stream.push_chunk(data)?;
            }
        }
    }
    stream.finish()
}

fn stream_transport_error(detail: String) -> TimelineStreamError {
    TimelineStreamError::Parse(crate::services::metrics::model::ParseError::InvalidJson {
        line: 0,
        detail,
    })
}

fn authorize_ingestion(
    headers: &HeaderMap,
    configured_secret: Option<&str>,
) -> Result<(), HttpError> {
    let expected = configured_secret.ok_or_else(|| {
        HttpError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "Metrics ingestion is not configured",
        )
    })?;
    let supplied = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| HttpError::unauthorized("Missing or invalid Authorization header"))?;
    let supplied = supplied.strip_prefix("Bearer ").unwrap_or(supplied);
    if !constant_time_secret_eq(expected, supplied) {
        return Err(HttpError::unauthorized("Invalid ingestion credentials"));
    }
    Ok(())
}

fn constant_time_secret_eq(expected: &str, supplied: &str) -> bool {
    let Ok(mut expected_mac) = HmacSha256::new_from_slice(expected.as_bytes()) else {
        return false;
    };
    expected_mac.update(b"svocraft-metrics-ingestion-v1");
    let expected_tag = expected_mac.finalize().into_bytes();
    let Ok(mut supplied_mac) = HmacSha256::new_from_slice(supplied.as_bytes()) else {
        return false;
    };
    supplied_mac.update(b"svocraft-metrics-ingestion-v1");
    supplied_mac.verify_slice(&expected_tag).is_ok()
}

fn validate_upload_headers(headers: &HeaderMap, max_bytes: usize) -> Result<(), HttpError> {
    if let Some(length) = headers
        .get(header::CONTENT_LENGTH)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<usize>().ok())
        && length > max_bytes.saturating_add(1024 * 1024)
    {
        return Err(HttpError::new(
            StatusCode::PAYLOAD_TOO_LARGE,
            "Timeline upload exceeds the configured size limit",
        ));
    }
    let content_type = headers
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| HttpError::bad_request("Missing Content-Type"))?;
    if !content_type.starts_with("multipart/form-data")
        && !content_type.starts_with("application/x-ndjson")
        && !content_type.starts_with("application/ndjson")
    {
        return Err(HttpError::new(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "Expected multipart/form-data or application/x-ndjson",
        ));
    }
    Ok(())
}

fn map_stream_error(error: TimelineStreamError) -> HttpError {
    match error {
        TimelineStreamError::TooLarge { .. } | TimelineStreamError::LineTooLarge => {
            HttpError::new(StatusCode::PAYLOAD_TOO_LARGE, error.to_string())
        }
        TimelineStreamError::Parse(_) | TimelineStreamError::Aggregate(_) => {
            HttpError::new(StatusCode::UNPROCESSABLE_ENTITY, error.to_string())
        }
    }
}

fn map_persistence_error(error: PersistenceError, game_id: Uuid) -> HttpError {
    match error {
        PersistenceError::Conflict { .. } => {
            HttpError::new(StatusCode::CONFLICT, error.to_string())
        }
        PersistenceError::InvalidTimestamp(_) => HttpError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Timeline contains an unsupported timestamp",
        ),
        PersistenceError::Serialization(_) | PersistenceError::Database(_) => {
            tracing::error!(
                action = "metrics.ingestion.persistence_failed",
                game_id = %game_id,
                error = %error,
                "Metric timeline persistence failed"
            );
            HttpError::internal_error("Failed to persist timeline")
        }
    }
}

fn match_response(bundle: MatchBundle) -> MatchResponse {
    let players = map_match_players(&bundle.players);
    let mut teams: BTreeMap<String, TeamSummaryResponse> = BTreeMap::new();
    for player in &players {
        let Some(team) = player
            .final_team
            .as_deref()
            .filter(|team| !team.is_empty() && *team != "Spectators")
        else {
            continue;
        };
        let totals = teams
            .entry(team.to_string())
            .or_insert(TeamSummaryResponse {
                team: team.to_string(),
                players: 0,
                kills: 0,
                deaths: 0,
                assists: 0,
                vehicle_destructions: 0,
            });
        totals.players += 1;
        totals.kills += player.stats.kills;
        totals.deaths += player.stats.deaths;
        totals.assists += player.stats.assists;
        totals.vehicle_destructions += player.stats.vehicle_destructions;
    }
    MatchResponse {
        game_id: bundle.match_model.game_id,
        map: bundle.match_model.map,
        started_at: bundle.match_model.started_at,
        ended_at: bundle.match_model.ended_at,
        duration_ms: bundle.match_model.duration_ms,
        winning_team: bundle.match_model.winning_team,
        status: bundle.match_model.status,
        events_processed: bundle.match_model.event_count,
        player_count: bundle.match_model.player_count,
        teams: teams.into_values().collect(),
        players,
    }
}

fn map_match_players(
    players: &[(MetricMatchPlayerModel, MetricPlayerMatchStatModel)],
) -> Vec<MatchPlayerResponse> {
    players
        .iter()
        .map(|(player, stats)| match_player_response(player, stats))
        .collect()
}

fn match_player_response(
    player: &MetricMatchPlayerModel,
    model: &MetricPlayerMatchStatModel,
) -> MatchPlayerResponse {
    let mut stats = PlayerStats::default();
    merge_stats(&mut stats, model);
    normalize_stats(&mut stats);
    MatchPlayerResponse {
        player_id: player.player_id,
        nickname: player.nickname.clone(),
        initial_team: player.initial_team.clone(),
        final_team: player.final_team.clone(),
        changed_team: player.changed_team,
        team_changes: player.team_changes,
        left_count: player.left_count,
        time_in_game_ms: player.time_in_game_ms,
        kd: ratio(stats.kills, stats.deaths),
        kda: ratio(stats.kills + stats.assists, stats.deaths),
        damage_per_minute: damage_per_minute(stats.damage_dealt, player.time_in_game_ms),
        stats,
    }
}

fn player_stats_response(
    player_id: Uuid,
    nickname: String,
    period: PeriodResponse,
    mut summary: PlayerSummary,
) -> PlayerStatsResponse {
    normalize_stats(&mut summary.stats);
    PlayerStatsResponse {
        player_id,
        nickname,
        period,
        time_in_game_ms: summary.time_in_game_ms,
        kd: ratio(summary.stats.kills, summary.stats.deaths),
        kda: ratio(
            summary.stats.kills + summary.stats.assists,
            summary.stats.deaths,
        ),
        damage_per_minute: damage_per_minute(summary.stats.damage_dealt, summary.time_in_game_ms),
        win_rate: win_rate(&summary.stats),
        stats: summary.stats,
    }
}

fn match_list_item(model: MetricMatchModel) -> MatchListItemResponse {
    MatchListItemResponse {
        game_id: model.game_id,
        map: model.map,
        started_at: model.started_at,
        ended_at: model.ended_at,
        duration_ms: model.duration_ms,
        winning_team: model.winning_team,
        status: model.status,
    }
}

fn parse_period(
    from: Option<String>,
    to: Option<String>,
    period: Option<String>,
) -> Result<(TimeRange, PeriodResponse), HttpError> {
    if (from.is_some() || to.is_some()) && period.as_deref().is_some_and(|value| value != "all") {
        return Err(HttpError::bad_request(
            "period cannot be combined with explicit from/to",
        ));
    }
    let now = Utc::now();
    let (from, to, name) = if from.is_some() || to.is_some() {
        (
            from.map(|value| parse_datetime(&value, "from"))
                .transpose()?,
            to.map(|value| parse_datetime(&value, "to")).transpose()?,
            "custom".to_string(),
        )
    } else {
        match period.as_deref().unwrap_or("all") {
            "all" => (None, None, "all".to_string()),
            "day" => (Some(now - Duration::days(1)), Some(now), "day".to_string()),
            "week" => (Some(now - Duration::days(7)), Some(now), "week".to_string()),
            "month" => (
                Some(now - Duration::days(30)),
                Some(now),
                "month".to_string(),
            ),
            _ => {
                return Err(HttpError::bad_request(
                    "period must be day, week, month or all",
                ));
            }
        }
    };
    if let (Some(from), Some(to)) = (from, to)
        && from > to
    {
        return Err(HttpError::bad_request("from must not be after to"));
    }
    Ok((TimeRange { from, to }, PeriodResponse { from, to, name }))
}

fn parse_datetime(value: &str, field: &str) -> Result<DateTime<Utc>, HttpError> {
    DateTime::parse_from_rfc3339(value)
        .map(|value| value.with_timezone(&Utc))
        .map_err(|_| HttpError::bad_request(format!("{field} must be ISO-8601/RFC3339")))
}

fn pagination(limit: Option<i64>, offset: Option<i64>) -> Result<(i64, i64), HttpError> {
    let limit = limit.unwrap_or(50);
    let offset = offset.unwrap_or(0);
    if !(1..=100).contains(&limit) {
        return Err(HttpError::bad_request("limit must be between 1 and 100"));
    }
    if offset < 0 {
        return Err(HttpError::bad_request("offset must be non-negative"));
    }
    Ok((limit, offset))
}

fn parse_uuid(value: &str, label: &str) -> Result<Uuid, HttpError> {
    Uuid::parse_str(value).map_err(|_| HttpError::bad_request(format!("Invalid {label}")))
}

fn database_error(error: sea_orm::DbErr) -> Response {
    tracing::error!(action = "metrics.read.failed", error = %error, "Metric read failed");
    HttpError::internal_error("Database error").into_response()
}

fn normalize_stats(stats: &mut PlayerStats) {
    stats.damage_dealt = round(stats.damage_dealt, 3);
    stats.damage_taken = round(stats.damage_taken, 3);
    stats.friendly_damage = round(stats.friendly_damage, 3);
    stats.self_damage = round(stats.self_damage, 3);
    stats.headshot_damage = round(stats.headshot_damage, 3);
    stats.vehicle_damage_dealt = round(stats.vehicle_damage_dealt, 3);
    stats.vehicle_damage_taken = round(stats.vehicle_damage_taken, 3);
    stats.friendly_vehicle_damage = round(stats.friendly_vehicle_damage, 3);
    for map in [
        &mut stats.damage_by_weapon,
        &mut stats.damage_by_source,
        &mut stats.vehicle_damage_by_type,
        &mut stats.vehicle_damage_by_weapon,
    ] {
        for value in map.values_mut() {
            *value = round(*value, 3);
        }
    }
}

fn ratio(numerator: i64, denominator: i64) -> f64 {
    round(numerator as f64 / denominator.max(1) as f64, 3)
}

fn damage_per_minute(damage: f64, time_in_game_ms: i64) -> f64 {
    let minutes = time_in_game_ms as f64 / 60_000.0;
    if minutes > 0.0 {
        round(damage / minutes, 3)
    } else {
        0.0
    }
}

fn win_rate(stats: &PlayerStats) -> f64 {
    if stats.matches_played > 0 {
        round(stats.wins as f64 * 100.0 / stats.matches_played as f64, 2)
    } else {
        0.0
    }
}

fn round(value: f64, precision: i32) -> f64 {
    let factor = 10_f64.powi(precision);
    (value * factor).round() / factor
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derived_metrics_never_return_nan_or_infinity() {
        assert_eq!(ratio(0, 0), 0.0);
        assert_eq!(ratio(5, 0), 5.0);
        assert_eq!(damage_per_minute(100.0, 0), 0.0);
        assert!(ratio(5, 0).is_finite());
        assert!(damage_per_minute(100.0, 0).is_finite());
    }

    #[test]
    fn validates_periods_and_pagination() {
        assert!(
            parse_period(
                Some("2026-01-02T00:00:00Z".to_string()),
                Some("2026-01-01T00:00:00Z".to_string()),
                None,
            )
            .is_err()
        );
        assert!(pagination(Some(-1), Some(0)).is_err());
        assert!(pagination(Some(101), Some(0)).is_err());
        assert!(pagination(Some(50), Some(-1)).is_err());
    }
}
