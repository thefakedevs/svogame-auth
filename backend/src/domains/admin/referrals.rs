use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::app::auth::require_human_superuser;
use crate::app::http::{HttpError, HttpResult};
use crate::app::state::AppStateExtractor;
use crate::services::referrals::{
    CreateReferralCampaignInput, ReferralCampaignListResponse, ReferralCampaignView,
    ReferralStatsResponse, UpdateReferralCampaignInput,
};

const DEFAULT_PAGE: u64 = 1;
const DEFAULT_PER_PAGE: u64 = 20;
const MAX_PER_PAGE: u64 = 100;

#[derive(Deserialize, IntoParams, ToSchema)]
pub struct ListReferralCampaignsQuery {
    pub page: Option<u64>,
    #[serde(rename = "perPage")]
    pub per_page: Option<u64>,
}

#[derive(Deserialize, IntoParams, ToSchema)]
pub struct ReferralCampaignStatsQuery {
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
}

#[utoipa::path(
    get,
    path = "/api/admin/referral-campaigns",
    params(ListReferralCampaignsQuery),
    responses(
        (status = 200, description = "List referral campaigns.", body = ReferralCampaignListResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    ),
    security(("bearer_auth" = [])),
    tag = "referrals-admin"
)]
pub async fn list_campaigns(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Query(query): Query<ListReferralCampaignsQuery>,
) -> HttpResult<Json<ReferralCampaignListResponse>> {
    let state = state.read().await;
    require_human_superuser(&headers, &state).await?;
    let page = query.page.unwrap_or(DEFAULT_PAGE).max(1);
    let per_page = query
        .per_page
        .unwrap_or(DEFAULT_PER_PAGE)
        .clamp(1, MAX_PER_PAGE);
    crate::services::referrals::list_campaigns(&state.db, page, per_page)
        .await
        .map(Json)
        .map_err(|e| HttpError::internal_error(e.to_string()))
}

#[utoipa::path(
    post,
    path = "/api/admin/referral-campaigns",
    request_body = CreateReferralCampaignInput,
    responses(
        (status = 200, description = "Create referral campaign.", body = ReferralCampaignView),
        (status = 400, description = "Invalid campaign payload."),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    ),
    security(("bearer_auth" = [])),
    tag = "referrals-admin"
)]
pub async fn create_campaign(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Json(body): Json<CreateReferralCampaignInput>,
) -> HttpResult<Json<ReferralCampaignView>> {
    let state = state.read().await;
    let admin = require_human_superuser(&headers, &state).await?;
    crate::services::referrals::create_campaign(&state.db, admin.id, body)
        .await
        .map(Json)
        .map_err(|e| HttpError::bad_request(e.to_string()))
}

#[utoipa::path(
    get,
    path = "/api/admin/referral-campaigns/{campaign_id}",
    params(("campaign_id" = String, Path, description = "Referral campaign UUID")),
    responses(
        (status = 200, description = "Get referral campaign.", body = ReferralCampaignView),
        (status = 400, description = "Invalid campaign ID."),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Referral campaign not found.")
    ),
    security(("bearer_auth" = [])),
    tag = "referrals-admin"
)]
pub async fn get_campaign(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Path(campaign_id): Path<String>,
) -> HttpResult<Json<ReferralCampaignView>> {
    let state = state.read().await;
    require_human_superuser(&headers, &state).await?;
    let campaign_id = parse_uuid(&campaign_id)?;
    crate::services::referrals::get_campaign(&state.db, campaign_id)
        .await
        .map(Json)
        .map_err(|_| HttpError::not_found("Referral campaign not found"))
}

#[utoipa::path(
    patch,
    path = "/api/admin/referral-campaigns/{campaign_id}",
    params(("campaign_id" = String, Path, description = "Referral campaign UUID")),
    request_body = UpdateReferralCampaignInput,
    responses(
        (status = 200, description = "Patch referral campaign.", body = ReferralCampaignView),
        (status = 400, description = "Invalid campaign payload."),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    ),
    security(("bearer_auth" = [])),
    tag = "referrals-admin"
)]
pub async fn patch_campaign(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Path(campaign_id): Path<String>,
    Json(body): Json<UpdateReferralCampaignInput>,
) -> HttpResult<Json<ReferralCampaignView>> {
    let state = state.read().await;
    require_human_superuser(&headers, &state).await?;
    let campaign_id = parse_uuid(&campaign_id)?;
    crate::services::referrals::update_campaign(&state.db, campaign_id, body)
        .await
        .map(Json)
        .map_err(|e| HttpError::bad_request(e.to_string()))
}

#[utoipa::path(
    post,
    path = "/api/admin/referral-campaigns/{campaign_id}/revoke",
    params(("campaign_id" = String, Path, description = "Referral campaign UUID")),
    responses(
        (status = 200, description = "Revoke referral campaign.", body = ReferralCampaignView),
        (status = 400, description = "Invalid campaign ID."),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    ),
    security(("bearer_auth" = [])),
    tag = "referrals-admin"
)]
pub async fn revoke_campaign(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Path(campaign_id): Path<String>,
) -> HttpResult<Json<ReferralCampaignView>> {
    let state = state.read().await;
    require_human_superuser(&headers, &state).await?;
    let campaign_id = parse_uuid(&campaign_id)?;
    crate::services::referrals::revoke_campaign(&state.db, campaign_id)
        .await
        .map(Json)
        .map_err(|e| HttpError::bad_request(e.to_string()))
}

#[utoipa::path(
    get,
    path = "/api/admin/referral-campaigns/{campaign_id}/stats",
    params(
        ("campaign_id" = String, Path, description = "Referral campaign UUID"),
        ReferralCampaignStatsQuery
    ),
    responses(
        (status = 200, description = "Referral campaign daily registration stats.", body = ReferralStatsResponse),
        (status = 400, description = "Invalid campaign ID or query."),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    ),
    security(("bearer_auth" = [])),
    tag = "referrals-admin"
)]
pub async fn campaign_stats(
    State(state): AppStateExtractor,
    headers: HeaderMap,
    Path(campaign_id): Path<String>,
    Query(query): Query<ReferralCampaignStatsQuery>,
) -> HttpResult<Json<ReferralStatsResponse>> {
    let state = state.read().await;
    require_human_superuser(&headers, &state).await?;
    let campaign_id = parse_uuid(&campaign_id)?;
    crate::services::referrals::campaign_stats(&state.db, campaign_id, query.from, query.to)
        .await
        .map(Json)
        .map_err(|e| HttpError::bad_request(e.to_string()))
}

fn parse_uuid(value: &str) -> HttpResult<Uuid> {
    Uuid::parse_str(value).map_err(|_| HttpError::bad_request("Invalid UUID"))
}
