use crate::png_checker::{CheckError, ChunkError, ChunkType};

pub mod super_big_filename_for_soo_small_validator_lol;

#[inline(always)]
pub fn validate_iend(chunk_name: &[u8; 4], length: &[u8; 4], crc: u32) -> Result<(), CheckError> {
    use super_big_filename_for_soo_small_validator_lol::*;

    if !crc_checker(crc) {
        return Err(CheckError::Chunk(ChunkError::InvalidCrc(ChunkType::IEND)));
    }

    if !chunk_name_checker(chunk_name) {
        return Err(CheckError::Chunk(ChunkError::InvalidName(ChunkType::IEND)));
    }

    if !length_checker(length) {
        return Err(CheckError::Chunk(ChunkError::InvalidLength(
            ChunkType::IEND,
        )));
    }

    Ok(())
}
