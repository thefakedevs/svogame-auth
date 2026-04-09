use axum::Json;
use axum::extract::multipart::MultipartError;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::json;
use utoipa::ToSchema;

#[derive(Deserialize, Serialize, Debug, Clone, Copy, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum SkinModel {
    Default,
    Slim,
}

impl Default for SkinModel {
    fn default() -> Self {
        Self::Default
    }
}

impl SkinModel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Slim => "slim",
        }
    }
}

fn deserialize_model<'de, D>(deserializer: D) -> Result<SkinModel, D::Error>
where
    D: Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    Ok(match s.as_str() {
        "default" => SkinModel::Default,
        "slim" => SkinModel::Slim,
        _ => SkinModel::Default,
    })
}

#[derive(Deserialize, Debug, ToSchema)]
pub struct ModelParam {
    #[serde(default, deserialize_with = "deserialize_model")]
    pub model: SkinModel,
}

#[derive(Serialize, ToSchema)]
pub struct UploadSkinResponse {
    pub status: &'static str,
    pub uuid: String,
    pub model: String,
    pub message: &'static str,
}

#[derive(Debug, thiserror::Error)]
pub enum SkinError {
    #[error("Invalid UUID v4 format")]
    InvalidUuid,
    #[error("Missing or invalid Content-Type header. Expected multipart/form-data")]
    MissingContentType,
    #[error("Only image/png is allowed")]
    InvalidContentType,
    #[error("Max file size is {0} bytes")]
    FileTooLarge(usize),
    #[error("Minimum allowed size is {0} bytes")]
    FileTooSmall(usize),
    #[error("No data received within timeout")]
    UploadTimeout,
    #[error("Failed to read upload chunk: {0}")]
    ChunkReadError(String),
    #[error("Invalid PNG file: {0}")]
    InvalidPng(#[from] crate::png_checker::CheckError),
    #[error("No file uploaded")]
    NoFile,
    #[error("Multipart error: {0}")]
    MultipartError(#[from] MultipartError),
    #[error("Skin not found")]
    NotFound,
    #[error("Failed to read skin from storage")]
    ReadFailed,
    #[error("Failed to store skin in storage")]
    WriteFailed,
}

impl IntoResponse for SkinError {
    fn into_response(self) -> Response {
        let (status, code) = match &self {
            Self::InvalidUuid => (StatusCode::BAD_REQUEST, "invalid_uuid"),
            Self::MissingContentType => (StatusCode::BAD_REQUEST, "missing_content_type"),
            Self::InvalidContentType => (StatusCode::UNSUPPORTED_MEDIA_TYPE, "invalid_file_type"),
            Self::FileTooLarge(_) => (StatusCode::PAYLOAD_TOO_LARGE, "file_too_large"),
            Self::FileTooSmall(_) => (StatusCode::BAD_REQUEST, "file_too_small"),
            Self::UploadTimeout => (StatusCode::REQUEST_TIMEOUT, "upload_timeout"),
            Self::ChunkReadError(_) => (StatusCode::BAD_REQUEST, "chunk_read_error"),
            Self::InvalidPng(_) => (StatusCode::UNPROCESSABLE_ENTITY, "invalid_png"),
            Self::NoFile => (StatusCode::BAD_REQUEST, "no_file"),
            Self::MultipartError(_) => (StatusCode::BAD_REQUEST, "multipart_error"),
            Self::NotFound => (StatusCode::NOT_FOUND, "not_found"),
            Self::ReadFailed => (StatusCode::INTERNAL_SERVER_ERROR, "read_error"),
            Self::WriteFailed => (StatusCode::INTERNAL_SERVER_ERROR, "write_error"),
        };

        if status.is_server_error() {
            tracing::error!(%status, "Skin request error: {}", self);
        } else {
            tracing::warn!(%status, "Skin request rejected: {}", self);
        }

        (
            status,
            Json(json!({
                "status": "error",
                "code": code,
                "message": self.to_string(),
            })),
        )
            .into_response()
    }
}
