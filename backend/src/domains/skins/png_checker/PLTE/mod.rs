use crate::png_checker::{
    CheckError, ChunkError, ChunkType, ColorType, PLTE::palette::check_palette_length,
    PLTERequirement, c_crc32,
};

mod palette;

pub fn check_palette_useless(color_type: ColorType) -> PLTERequirement {
    match color_type {
        ColorType::IndexedColor => PLTERequirement::Must, // 3
        ColorType::TrueColor | ColorType::TrueColorAlpha => PLTERequirement::Optional, // 2 | 6
        ColorType::GrayScale | ColorType::GrayScaleAlpha => PLTERequirement::Disallowed, // 0 | 4
    }
}

pub fn check_PLTE(
    color_type: ColorType,
    len: u32,
    crc: u32,
    data: &[u8],
) -> Result<(), CheckError> {
    if !c_crc32::check_crc32(data, crc, ChunkType::PLTE) {
        return Err(CheckError::Chunk(ChunkError::InvalidCrc(ChunkType::PLTE)));
    }

    if check_palette_useless(color_type) == PLTERequirement::Disallowed {
        return Err(CheckError::Chunk(ChunkError::PlteNotAllowed {
            color_type,
            requirement: PLTERequirement::Disallowed,
        }));
    }

    if let Err(e) = check_palette_length(len) {
        return Err(e);
    }

    Ok(())
}
