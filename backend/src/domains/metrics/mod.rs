use axum::Router;
use axum::routing::{get, post};

pub mod handlers;

pub fn router() -> Router<crate::app::state::SharedAppState> {
    Router::new()
        .route(
            "/api/metrics/timeline/{game_id}",
            post(handlers::ingest_timeline),
        )
        .route("/api/metrics/matches/{game_id}", get(handlers::get_match))
        .route(
            "/api/metrics/matches/{game_id}/players",
            get(handlers::get_match_players),
        )
        .route(
            "/api/metrics/matches/{game_id}/players/{player_id}",
            get(handlers::get_match_player),
        )
        .route(
            "/api/metrics/players/{player_id}",
            get(handlers::get_player),
        )
        .route(
            "/api/metrics/players/{player_id}/matches",
            get(handlers::get_player_matches),
        )
        .route(
            "/api/metrics/players/{player_id}/stats",
            get(handlers::get_player_stats),
        )
        .route("/api/metrics/leaderboard", get(handlers::get_leaderboard))
}
