use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use serde_json::json;
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
pub struct ProblemDetails {
    pub status: u16,
    pub title: String,
    pub detail: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ProblemResponse {
    pub error: String,
    pub problem: ProblemDetails,
}

#[derive(Debug)]
pub struct HttpError {
    pub status: StatusCode,
    pub message: String,
}

impl HttpError {
    pub fn new(status: StatusCode, message: impl Into<String>) -> Self {
        Self {
            status,
            message: message.into(),
        }
    }

    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, message)
    }

    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self::new(StatusCode::UNAUTHORIZED, message)
    }

    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::new(StatusCode::FORBIDDEN, message)
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_FOUND, message)
    }

    pub fn internal_error(message: impl Into<String>) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, message)
    }
}

impl IntoResponse for HttpError {
    fn into_response(self) -> Response {
        let message = self.message;
        let status = self.status;
        let body = Json(json!({
            "error": message.clone(),
            "problem": {
                "status": status.as_u16(),
                "title": status.canonical_reason().unwrap_or("Request failed"),
                "detail": message,
            }
        }));
        (status, body).into_response()
    }
}

pub type HttpResult<T> = Result<T, HttpError>;
