use axum::body::Bytes;
use axum::extract::{Multipart, Path, Query, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use aws_sdk_s3::error::ProvideErrorMetadata;
use aws_sdk_s3::primitives::ByteStream;
use sea_orm::{ActiveModelTrait, EntityTrait, Set};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::app::auth::{get_user_from_headers, require_human_superuser};
use crate::app::state::AppStateExtractor;
use crate::domains::skins::types::{ModelParam, SkinError, SkinModel, UploadSkinResponse};
use crate::entities::{DefaultSkin, DefaultSkinActiveModel, DefaultSkinModel};
use crate::services::audit::{write_audit_log, ACTION_ADMIN_DEFAULT_SKIN_UPDATED};

const DEFAULT_SKIN_ROW_ID: i32 = 1;
const MIN_SIZE: usize = 78;
const MAX_SIZE: usize = 20 * 1024;
const MAX_MULTIPART_BODY_SIZE: usize = 64 * 1024;
const CHUNK_TIMEOUT_SECS: u64 = 5;
const DEFAULT_SKINS_PREFIX: &str = "default_skins";
const USER_SKINS_PREFIX: &str = "user_skins";
const PNG_CONTENT_TYPE: &str = "image/png";

#[derive(serde::Serialize, ToSchema)]
pub struct DefaultSkinResponse {
    #[serde(rename = "imageUrl")]
    pub image_url: String,
    #[serde(rename = "contentType")]
    pub content_type: String,
    #[serde(rename = "updatedByUserId")]
    pub updated_by_user_id: Option<String>,
    #[serde(rename = "updatedAt")]
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

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

    validate_multipart_headers(&headers)?;

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
            .content_type(PNG_CONTENT_TYPE)
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
    path = "/api/skins/default",
    responses(
        (status = 200, description = "Default PNG skin file used as fallback when user skin is missing.", content_type = "image/png"),
        (status = 404, description = "Default skin not configured")
    ),
    tag = "skins"
)]
pub async fn get_default_skin(
    State(state): AppStateExtractor,
) -> Result<Response, Response> {
    let state = state.read().await;
    let default_skin = DefaultSkin::find_by_id(DEFAULT_SKIN_ROW_ID)
        .one(&state.db)
        .await
        .map_err(|error| {
            tracing::error!("Failed to load default skin row: {}", error);
            SkinError::ReadFailed.into_response()
        })?
        .ok_or_else(|| SkinError::NotFound.into_response())?;

    let bytes = load_object_bytes(&state.s3, &state.config.s3.bucket, &default_skin.s3_key)
        .await?
        .ok_or_else(|| SkinError::NotFound.into_response())?;

    Ok(png_response(bytes))
}

#[utoipa::path(
    get,
    path = "/api/skins/{uuid}",
    params(
        ("uuid" = String, Path, description = "UUID v4")
    ),
    responses(
        (status = 200, description = "PNG skin file. Returns user skin when present, otherwise the configured default skin.", content_type = "image/png"),
        (status = 400, description = "Invalid UUID"),
        (status = 404, description = "Skin not found and default skin is not configured")
    ),
    tag = "skins"
)]
pub async fn get_skin(
    State(state): AppStateExtractor,
    Path(uuid_str): Path<String>,
) -> Result<Response, Response> {
    let uuid = validate_uuid(&uuid_str).map_err(IntoResponse::into_response)?;
    let state = state.read().await;
    let bucket = &state.config.s3.bucket;

    if let Some(bytes) = load_object_bytes(&state.s3, bucket, &skin_key(uuid)).await? {
        return Ok(png_response(bytes));
    }

    if let Some(default_skin) = DefaultSkin::find_by_id(DEFAULT_SKIN_ROW_ID)
        .one(&state.db)
        .await
        .map_err(|error| {
            tracing::error!("Failed to load default skin row: {}", error);
            SkinError::ReadFailed.into_response()
        })?
    {
        if let Some(bytes) = load_object_bytes(&state.s3, bucket, &default_skin.s3_key).await? {
            return Ok(png_response(bytes));
        }
    }

    Err(SkinError::NotFound.into_response())
}

#[utoipa::path(
    get,
    path = "/api/admin/skins/default",
    responses(
        (status = 200, description = "Current default skin metadata.", body = DefaultSkinResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Default skin not configured")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "admin"
)]
pub async fn get_default_skin_admin(
    State(state): AppStateExtractor,
    headers: HeaderMap,
) -> Result<Json<DefaultSkinResponse>, Response> {
    let state = state.read().await;
    require_human_superuser(&headers, &state)
        .await
        .map_err(IntoResponse::into_response)?;

    let model = DefaultSkin::find_by_id(DEFAULT_SKIN_ROW_ID)
        .one(&state.db)
        .await
        .map_err(|error| {
            tracing::error!("Failed to load default skin metadata: {}", error);
            SkinError::ReadFailed.into_response()
        })?
        .ok_or_else(|| SkinError::NotFound.into_response())?;

    Ok(Json(map_default_skin_response(&model)))
}

#[utoipa::path(
    post,
    path = "/api/admin/skins/default",
    params(
        ("model" = Option<String>, Query, description = "Model type used while validating and normalizing the uploaded default skin: 'default' or 'slim'")
    ),
    request_body(
        content_type = "multipart/form-data",
        description = "PNG skin file to use as the global default fallback skin."
    ),
    responses(
        (status = 200, description = "Default skin uploaded or replaced.", body = DefaultSkinResponse),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 415, description = "Unsupported media type"),
        (status = 422, description = "Invalid PNG file")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "admin"
)]
pub async fn upload_default_skin_admin(
    State(state): AppStateExtractor,
    Query(model): Query<ModelParam>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> Result<Json<DefaultSkinResponse>, Response> {
    let state = state.read().await;
    let admin = require_human_superuser(&headers, &state)
        .await
        .map_err(IntoResponse::into_response)?;

    validate_multipart_headers(&headers)?;

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
        let key = default_skin_key();

        state
            .s3
            .put_object()
            .bucket(&state.config.s3.bucket)
            .key(&key)
            .content_type(PNG_CONTENT_TYPE)
            .metadata("model", model.model.as_str())
            .body(ByteStream::from(processed_png))
            .send()
            .await
            .map_err(|error| {
                tracing::error!("Failed to upload default skin to S3: {}", error);
                SkinError::WriteFailed.into_response()
            })?;

        let now = chrono::Utc::now();
        let stored = if let Some(existing) = DefaultSkin::find_by_id(DEFAULT_SKIN_ROW_ID)
            .one(&state.db)
            .await
            .map_err(|error| {
                tracing::error!("Failed to load default skin row for update: {}", error);
                SkinError::ReadFailed.into_response()
            })?
        {
            let mut active: DefaultSkinActiveModel = existing.into();
            active.s3_key = Set(key.clone());
            active.content_type = Set(PNG_CONTENT_TYPE.to_string());
            active.updated_by_user_id = Set(Some(admin.id));
            active.updated_at = Set(now);
            active.update(&state.db).await.map_err(|error| {
                tracing::error!("Failed to update default skin row: {}", error);
                SkinError::WriteFailed.into_response()
            })?
        } else {
            DefaultSkinActiveModel {
                id: Set(DEFAULT_SKIN_ROW_ID),
                s3_key: Set(key.clone()),
                content_type: Set(PNG_CONTENT_TYPE.to_string()),
                updated_by_user_id: Set(Some(admin.id)),
                updated_at: Set(now),
            }
            .insert(&state.db)
            .await
            .map_err(|error| {
                tracing::error!("Failed to create default skin row: {}", error);
                SkinError::WriteFailed.into_response()
            })?
        };

        write_audit_log(
            &state.db,
            ACTION_ADMIN_DEFAULT_SKIN_UPDATED,
            Some(admin.id),
            None,
            None,
            Some(serde_json::json!({
                "s3Key": key,
                "model": model.model.as_str(),
            })),
        )
        .await
        .map_err(|error| {
            tracing::error!("Failed to write default skin audit log: {}", error);
            SkinError::WriteFailed.into_response()
        })?;

        return Ok(Json(map_default_skin_response(&stored)));
    }

    Err(SkinError::NoFile.into_response())
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
    if ct != PNG_CONTENT_TYPE {
        return Err(SkinError::InvalidContentType);
    }
    Ok(())
}

fn validate_multipart_headers(headers: &HeaderMap) -> Result<(), Response> {
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

fn default_skin_key() -> String {
    format!("{}/default.png", DEFAULT_SKINS_PREFIX)
}

async fn load_object_bytes(
    s3: &aws_sdk_s3::Client,
    bucket: &str,
    key: &str,
) -> Result<Option<Bytes>, Response> {
    let object = match s3.get_object().bucket(bucket).key(key).send().await {
        Ok(object) => object,
        Err(error) => {
            let maybe_code = error.as_service_error().and_then(|service_error| service_error.code());
            if matches!(maybe_code, Some("NoSuchKey") | Some("NotFound") | Some("404")) {
                return Ok(None);
            }
            tracing::error!("Failed to load skin object '{}': {}", key, error);
            return Err(SkinError::ReadFailed.into_response());
        }
    };

    let bytes = object
        .body
        .collect()
        .await
        .map_err(|error| {
            tracing::error!("Failed to read skin body from S3 for key '{}': {}", key, error);
            SkinError::ReadFailed.into_response()
        })?
        .into_bytes();

    Ok(Some(bytes))
}

fn png_response(bytes: Bytes) -> Response {
    (StatusCode::OK, [(header::CONTENT_TYPE, PNG_CONTENT_TYPE)], bytes).into_response()
}

fn map_default_skin_response(model: &DefaultSkinModel) -> DefaultSkinResponse {
    DefaultSkinResponse {
        image_url: "/api/skins/default".to_string(),
        content_type: model.content_type.clone(),
        updated_by_user_id: model.updated_by_user_id.map(|value| value.to_string()),
        updated_at: model.updated_at,
    }
}
