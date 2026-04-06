use axum::Json;
use chrono::Utc;
use serde::Serialize;

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    #[serde(rename = "serverTime")]
    pub server_time: chrono::DateTime<chrono::Utc>,
}

pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        server_time: Utc::now(),
    })
}
