use std::io::Cursor;

use anyhow::Result;
use image::ImageFormat;
use regex::Regex;
use uuid::Uuid;

pub const SQUAD_MAX_MEMBERS: u64 = 4;
pub const SQUAD_INVITE_TTL_HOURS: i64 = 24;
pub const SQUAD_NAME_MIN_CHARS: usize = 4;
pub const SQUAD_NAME_MAX_CHARS: usize = 16;
pub const SQUAD_IMAGE_MAX_BYTES: usize = 2 * 1024 * 1024;
pub const SQUAD_IMAGE_MAX_WIDTH: u32 = 1024;
pub const SQUAD_IMAGE_MAX_HEIGHT: u32 = 1024;
pub const SQUAD_IMAGES_PREFIX: &str = "squad_images";
pub const SQUAD_NAME_REGEX: &str = r"^[\p{Latin}\p{Cyrillic} -]+$";

pub fn validate_squad_name(name: &str) -> Result<String> {
    let trimmed = name.trim();
    let len = trimmed.chars().count();

    if !(SQUAD_NAME_MIN_CHARS..=SQUAD_NAME_MAX_CHARS).contains(&len) {
        anyhow::bail!(
            "Squad name must be between {} and {} characters long",
            SQUAD_NAME_MIN_CHARS,
            SQUAD_NAME_MAX_CHARS
        );
    }

    let regex = Regex::new(SQUAD_NAME_REGEX)?;
    if !regex.is_match(trimmed) {
        anyhow::bail!("Squad name contains unsupported characters");
    }

    if trimmed.starts_with('-')
        || trimmed.ends_with('-')
        || trimmed.starts_with(' ')
        || trimmed.ends_with(' ')
    {
        anyhow::bail!("Squad name cannot start or end with a space or hyphen");
    }

    Ok(trimmed.to_string())
}

pub fn squad_image_key(squad_id: Uuid) -> String {
    format!("{}/{}.png", SQUAD_IMAGES_PREFIX, squad_id.as_hyphenated())
}

pub fn process_squad_image(data: &[u8]) -> Result<Vec<u8>> {
    if data.is_empty() {
        anyhow::bail!("Image file is empty");
    }

    if data.len() > SQUAD_IMAGE_MAX_BYTES {
        anyhow::bail!("Image exceeds maximum size of {} bytes", SQUAD_IMAGE_MAX_BYTES);
    }

    let format = image::guess_format(data)?;
    let image = image::load_from_memory_with_format(data, format)?;

    if image.width() > SQUAD_IMAGE_MAX_WIDTH || image.height() > SQUAD_IMAGE_MAX_HEIGHT {
        anyhow::bail!(
            "Image exceeds maximum resolution of {}x{}",
            SQUAD_IMAGE_MAX_WIDTH,
            SQUAD_IMAGE_MAX_HEIGHT
        );
    }

    let mut cursor = Cursor::new(Vec::new());
    image.write_to(&mut cursor, ImageFormat::Png)?;
    Ok(cursor.into_inner())
}
