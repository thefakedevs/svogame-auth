use axum::body::Bytes;
use axum::extract::{Multipart, Path, Query, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use aws_sdk_s3::error::ProvideErrorMetadata;
use aws_sdk_s3::primitives::ByteStream;
use uuid::Uuid;

use crate::app::auth::get_user_from_headers;
use crate::app::state::AppStateExtractor;
use crate::domains::skins::types::{ModelParam, SkinError, SkinModel, UploadSkinResponse};

const MIN_SIZE: usize = 78;
const MAX_SIZE: usize = 20 * 1024;
const MAX_MULTIPART_BODY_SIZE: usize = 64 * 1024;
const CHUNK_TIMEOUT_SECS: u64 = 5;
const USER_SKINS_PREFIX: &str = "user_skins";

#[utoipa::path(
    post,
    path = "/api/skins/me",
    params(
        ("model" = Option<String>, Query, description = "Model type: 'default' or 'slim'")
    ),
    request_body(
        content_type = "multipart/form-data",
        description = "PNG skin file (64x64, up to 20KB)"
    ),
    responses(
        (status = 200, description = "Skin uploaded successfully", body = UploadSkinResponse),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 415, description = "Unsupported media type"),
        (status = 422, description = "Invalid PNG file")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "skins"
)]
pub async fn upload_my_skin(
    State(state): AppStateExtractor,
    Query(model): Query<ModelParam>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> Result<Json<UploadSkinResponse>, Response> {
    let state = state.read().await;
    let user = get_user_from_headers(&headers, &state)
        .await
        .map_err(IntoResponse::into_response)?;

    if let Some(content_length) = headers.get(header::CONTENT_LENGTH) {
        if let Ok(len_str) = content_length.to_str() {
            if let Ok(len) = len_str.parse::<usize>() {
                if len > MAX_MULTIPART_BODY_SIZE {
                    return Err(SkinError::FileTooLarge(MAX_SIZE).into_response());
                }
            }
        }
    }

    let content_type = headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| SkinError::MissingContentType.into_response())?;

    if !content_type.starts_with("multipart/form-data") {
        return Err(SkinError::MissingContentType.into_response());
    }

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(SkinError::MultipartError)
        .map_err(IntoResponse::into_response)?
    {
        let field_content_type = field.content_type().map(|s| s.to_owned());
        validate_content_type(field_content_type.as_deref()).map_err(IntoResponse::into_response)?;

        let data = read_multipart_field(field).await.map_err(IntoResponse::into_response)?;
        let processed_png = process_skin(&data, model.model).map_err(IntoResponse::into_response)?;

        let key = skin_key(user.id);
        state
            .s3
            .put_object()
            .bucket(&state.config.s3.bucket)
            .key(&key)
            .content_type("image/png")
            .metadata("model", model.model.as_str())
            .body(ByteStream::from(processed_png))
            .send()
            .await
            .map_err(|error| {
                tracing::error!("Failed to upload skin to S3 for user {}: {}", user.id, error);
                SkinError::WriteFailed.into_response()
            })?;

        return Ok(Json(UploadSkinResponse {
            status: "ok",
            uuid: user.id.to_string(),
            model: model.model.as_str().to_string(),
            message: "Skin uploaded successfully",
        }));
    }

    Err(SkinError::NoFile.into_response())
}

#[utoipa::path(
    get,
    path = "/api/skins/{uuid}",
    params(
        ("uuid" = String, Path, description = "UUID v4")
    ),
    responses(
        (status = 200, description = "PNG skin file", content_type = "image/png"),
        (status = 400, description = "Invalid UUID"),
        (status = 404, description = "Skin not found")
    ),
    tag = "skins"
)]
pub async fn get_skin(
    State(state): AppStateExtractor,
    Path(uuid_str): Path<String>,
) -> Result<Response, Response> {
    let uuid = validate_uuid(&uuid_str).map_err(IntoResponse::into_response)?;
    let key = skin_key(uuid);
    let state = state.read().await;
    let bucket = state.config.s3.bucket.clone();
    let s3 = state.s3.clone();

    let object = s3
        .get_object()
        .bucket(&bucket)
        .key(&key)
        .send()
        .await
        .map_err(|error| {
            let maybe_code = error.as_service_error().and_then(|service_error| service_error.code());
            if matches!(maybe_code, Some("NoSuchKey") | Some("NotFound") | Some("404")) {
                SkinError::NotFound.into_response()
            } else {
                tracing::error!("Failed to load skin from S3: {}", error);
                SkinError::ReadFailed.into_response()
            }
        })?;

    let bytes = object
        .body
        .collect()
        .await
        .map_err(|error| {
            tracing::error!("Failed to read skin body from S3 for key '{}': {}", key, error);
            SkinError::ReadFailed.into_response()
        })?
        .into_bytes();

    Ok((
        StatusCode::OK,
        [(header::CONTENT_TYPE, "image/png")],
        bytes,
    )
        .into_response())
}

fn validate_uuid(input: &str) -> Result<Uuid, SkinError> {
    if input.len() != 36 {
        return Err(SkinError::InvalidUuid);
    }

    let uuid = Uuid::try_parse(input).map_err(|_| SkinError::InvalidUuid)?;
    if uuid.get_version_num() != 4 {
        return Err(SkinError::InvalidUuid);
    }

    Ok(uuid)
}

fn validate_content_type(content_type: Option<&str>) -> Result<(), SkinError> {
    let ct = content_type.ok_or(SkinError::MissingContentType)?;
    if ct != "image/png" {
        return Err(SkinError::InvalidContentType);
    }
    Ok(())
}

async fn read_multipart_field(
    field: axum::extract::multipart::Field<'_>,
) -> Result<Bytes, SkinError> {
    let data = tokio::time::timeout(
        std::time::Duration::from_secs(CHUNK_TIMEOUT_SECS),
        field.bytes(),
    )
    .await
    .map_err(|_| SkinError::UploadTimeout)?
    .map_err(|e| SkinError::ChunkReadError(e.to_string()))?;

    if data.len() > MAX_SIZE {
        return Err(SkinError::FileTooLarge(MAX_SIZE));
    }
    if data.len() < MIN_SIZE {
        return Err(SkinError::FileTooSmall(MIN_SIZE));
    }

    Ok(data)
}

fn process_skin(data: &[u8], model: SkinModel) -> Result<Vec<u8>, SkinError> {
    let mut output = Vec::new();
    crate::png_checker::process_png(data, model.as_str(), &mut output)?;
    Ok(output)
}

fn skin_key(uuid: Uuid) -> String {
    format!("{}/{}.png", USER_SKINS_PREFIX, uuid.as_hyphenated())
}
