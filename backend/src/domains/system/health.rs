use axum::Json;
use chrono::Utc;
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
pub struct HealthResponse {
    pub status: &'static str,
    #[serde(rename = "serverTime")]
    pub server_time: chrono::DateTime<chrono::Utc>,
}

#[utoipa::path(
    get,
    path = "/api/health",
    responses(
        (status = 200, description = "Basic service healthcheck with current server time.", body = HealthResponse)
    ),
    tag = "system"
)]
pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        server_time: Utc::now(),
    })
}
