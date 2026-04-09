use crate::app::http::{HttpError, HttpResult};
use crate::app::state::AppStateExtractor;
use crate::domains::auth::runtime::AuthPollResult;
use axum::Json;
use axum::extract::Path;
use serde::Serialize;
use std::time::Duration;
use tokio::time::timeout;
use utoipa::ToSchema;

const POLL_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Serialize, ToSchema)]
pub struct PollResponse {
    status: String,
    #[serde(rename = "accessToken", skip_serializing_if = "Option::is_none")]
    access_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    username: Option<String>,
    #[serde(rename = "avatarUrl", skip_serializing_if = "Option::is_none")]
    avatar_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

impl PollResponse {
    fn pending() -> Self {
        Self {
            status: "pending".to_string(),
            access_token: None,
            id: None,
            username: None,
            avatar_url: None,
            error: None,
        }
    }

    fn success(access_token: String, id: String, username: String, avatar_url: String) -> Self {
        Self {
            status: "success".to_string(),
            access_token: Some(access_token),
            id: Some(id),
            username: Some(username),
            avatar_url: Some(avatar_url),
            error: None,
        }
    }

    fn error(message: String) -> Self {
        Self {
            status: "error".to_string(),
            access_token: None,
            id: None,
            username: None,
            avatar_url: None,
            error: Some(message),
        }
    }

    fn expired() -> Self {
        Self {
            status: "expired".to_string(),
            access_token: None,
            id: None,
            username: None,
            avatar_url: None,
            error: Some("Время ожидания авторизации истекло".to_string()),
        }
    }
}

impl From<AuthPollResult> for PollResponse {
    fn from(result: AuthPollResult) -> Self {
        match result {
            AuthPollResult::Pending => PollResponse::pending(),
            AuthPollResult::Success {
                access_token,
                user_id,
                username,
                avatar_url,
            } => PollResponse::success(access_token, user_id, username, avatar_url),
            AuthPollResult::Error { message } => PollResponse::error(message),
            AuthPollResult::Expired => PollResponse::expired(),
        }
    }
}

#[utoipa::path(
    get,
    path = "/api/auth/poll/{poll_id}",
    params(
        ("poll_id" = String, Path, description = "Polling delivery identifier returned by `/api/auth/prepare` when `deliveryMethod = polling`.")
    ),
    responses(
        (status = 200, description = "Long-poll auth status. Returns `pending`, `success`, `error`, or cached completion state. Success includes issued token and user snapshot.", body = PollResponse),
        (status = 400, description = "Invalid poll ID format.")
    ),
    tag = "auth"
)]
pub async fn poll_auth_status(
    state: AppStateExtractor,
    Path(poll_id): Path<String>,
) -> HttpResult<Json<PollResponse>> {
    if uuid::Uuid::parse_str(&poll_id).is_err() {
        return Err(HttpError::bad_request("Invalid poll ID format"));
    }

    {
        let state_guard = state.read().await;
        if let Some(cached_result) = state_guard.auth.get_cached_result(&poll_id).await {
            return Ok(Json(PollResponse::from(cached_result)));
        }
    }

    let mut receiver = {
        let state_guard = state.read().await;
        state_guard.auth.subscribe_events()
    };

    let cache_check_interval = Duration::from_secs(2);

    let result = timeout(POLL_TIMEOUT, async {
        loop {
            let recv_result = timeout(cache_check_interval, receiver.recv()).await;

            match recv_result {
                Ok(Ok(event)) => {
                    if event.poll_id == poll_id {
                        return Some(event.result);
                    }
                }
                Ok(Err(tokio::sync::broadcast::error::RecvError::Closed)) => {
                    return Some(AuthPollResult::Error {
                        message: "Сервер завершает работу".to_string(),
                    });
                }
                Ok(Err(tokio::sync::broadcast::error::RecvError::Lagged(_))) => {
                    let state_guard = state.read().await;
                    if let Some(cached_result) = state_guard.auth.get_cached_result(&poll_id).await
                    {
                        return Some(cached_result);
                    }
                    continue;
                }
                Err(_) => {
                    let state_guard = state.read().await;
                    if let Some(cached_result) = state_guard.auth.get_cached_result(&poll_id).await
                    {
                        return Some(cached_result);
                    }
                    continue;
                }
            }
        }
    })
    .await;

    let response = match result {
        Ok(Some(poll_result)) => PollResponse::from(poll_result),
        Ok(None) => PollResponse::pending(),
        Err(_) => {
            let state_guard = state.read().await;
            if let Some(cached_result) = state_guard.auth.get_cached_result(&poll_id).await {
                return Ok(Json(PollResponse::from(cached_result)));
            }
            PollResponse::pending()
        }
    };

    Ok(Json(response))
}
