use axum::Json;
use serde::Serialize;

use crate::services::restrictions::RestrictionKind;
use crate::services::squads::{
    SQUAD_IMAGE_MAX_BYTES, SQUAD_IMAGE_MAX_HEIGHT, SQUAD_IMAGE_MAX_WIDTH, SQUAD_INVITE_TTL_HOURS,
    SQUAD_MAX_MEMBERS, SQUAD_NAME_MAX_CHARS, SQUAD_NAME_MIN_CHARS, SQUAD_NAME_REGEX,
};

#[derive(Serialize)]
pub struct RestrictionMetaResponse {
    pub key: &'static str,
    pub locale: RestrictionLocale,
}

#[derive(Serialize)]
pub struct RestrictionLocale {
    pub en: RestrictionLocaleEntry,
}

#[derive(Serialize)]
pub struct RestrictionLocaleEntry {
    pub title: &'static str,
    pub description: &'static str,
}

#[derive(Serialize)]
pub struct SquadConfigResponse {
    #[serde(rename = "maxMembers")]
    pub max_members: u64,
    #[serde(rename = "inviteTtlHours")]
    pub invite_ttl_hours: i64,
    #[serde(rename = "nameMinChars")]
    pub name_min_chars: usize,
    #[serde(rename = "nameMaxChars")]
    pub name_max_chars: usize,
    #[serde(rename = "nameRegex")]
    pub name_regex: &'static str,
    #[serde(rename = "imageMaxBytes")]
    pub image_max_bytes: usize,
    #[serde(rename = "imageMaxWidth")]
    pub image_max_width: u32,
    #[serde(rename = "imageMaxHeight")]
    pub image_max_height: u32,
}

pub async fn get_restrictions_meta() -> Json<Vec<RestrictionMetaResponse>> {
    Json(
        RestrictionKind::ALL
            .into_iter()
            .map(|kind| RestrictionMetaResponse {
                key: kind.as_str(),
                locale: RestrictionLocale {
                    en: RestrictionLocaleEntry {
                        title: kind.title_en(),
                        description: kind.description_en(),
                    },
                },
            })
            .collect(),
    )
}

pub async fn get_squads_config() -> Json<SquadConfigResponse> {
    Json(SquadConfigResponse {
        max_members: SQUAD_MAX_MEMBERS,
        invite_ttl_hours: SQUAD_INVITE_TTL_HOURS,
        name_min_chars: SQUAD_NAME_MIN_CHARS,
        name_max_chars: SQUAD_NAME_MAX_CHARS,
        name_regex: SQUAD_NAME_REGEX,
        image_max_bytes: SQUAD_IMAGE_MAX_BYTES,
        image_max_width: SQUAD_IMAGE_MAX_WIDTH,
        image_max_height: SQUAD_IMAGE_MAX_HEIGHT,
    })
}
