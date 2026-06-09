use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};

use crate::app::auth::get_user_from_headers;
use crate::app::http::{HttpError, HttpResult};
use crate::app::state::AppStateExtractor;
use crate::services::referrals::{
    MyReferralCampaignListResponse, PublicReferralCampaignView, ReferralStatsResponse,
};

#[derive(Deserialize, IntoParams, ToSchema)]
pub struct ReferralStatsQuery {
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
}

#[utoipa::path(
    get,
    path = "/api/referrals/{code}",
    params(("code" = String, Path, description = "Referral code")),
    responses(
        (status = 200, description = "Active referral campaign preview.", body = PublicReferralCampaignView),
        (status = 404, description = "Referral campaign was not found or is inactive.")
    ),
    tag = "referrals"
)]
pub async fn get_referral(
    State(state): AppStateExtractor,
    Path(code): Path<String>,
) -> HttpResult<Json<PublicReferralCampaignView>> {
    let state = state.read().await;
    crate::services::referrals::get_public_campaign(&state.db, &code)
        .await
        .map(Json)
        .map_err(|_| HttpError::not_found("Referral campaign not found"))
}

#[utoipa::path(
    get,
    path = "/api/referrals/me/campaigns",
    responses(
        (status = 200, description = "Current user's referral campaigns.", body = MyReferralCampaignListResponse),
        (status = 401, description = "Unauthorized")
    ),
    security(("bearer_auth" = [])),
    tag = "referrals"
)]
pub async fn my_referral_campaigns(
    State(state): AppStateExtractor,
    headers: HeaderMap,
) -> HttpResult<Json<MyReferralCampaignListResponse>> {
    let state = state.read().await;
    let user = get_user_from_headers(&headers, &state).await?;
    crate::services::referrals::list_creator_campaigns(&state.db, user.id)
        .await
        .map(Json)
        .map_err(|e| HttpError::bad_request(e.to_string()))
}

#[utoipa::path(
    get,
    path = "/api/referrals/me/stats",
    params(ReferralStatsQuery),
    responses(
        (status = 200, description = "Current content creator anonymized referral stats.", body = ReferralStatsResponse),
        (status = 401, description = "Unauthorized")
    ),
    security(("bearer_auth" = [])),
    tag = "referrals"
)]
pub async fn my_referral_stats(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Query(query): Query<ReferralStatsQuery>,
) -> HttpResult<Json<ReferralStatsResponse>> {
    let state = state.read().await;
    let user = get_user_from_headers(&headers, &state).await?;
    crate::services::referrals::creator_stats(&state.db, user.id, query.from, query.to)
        .await
        .map(Json)
        .map_err(|e| HttpError::bad_request(e.to_string()))
}
