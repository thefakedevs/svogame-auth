mod c_chunk;
mod c_compression;
mod c_filter;
mod c_interlace;

mod cnb;
mod wnh;

use crate::png_checker::CheckError;
use crate::png_checker::Chunk;
use crate::png_checker::ChunkError;
use crate::png_checker::ChunkType;
use crate::png_checker::IHDR::c_chunk::check_signature;
use crate::png_checker::IHDR::c_compression::check_compression;
use crate::png_checker::IHDR::c_filter::check_filter;
use crate::png_checker::IHDR::wnh::check_width_n_height;
use crate::png_checker::c_crc32;

pub use self::c_interlace::*;
pub use self::cnb::*;

pub fn check_and_validate_ihdr(chunk: &Chunk) -> Result<(&[u8; 4], &[u8; 4], u8, u8), CheckError> {
    // 0xD - 13 bytes
    // RFC chunk size (static)
    if chunk.length() != 0xD {
        return Err(CheckError::Chunk(ChunkError::InvalidSize(ChunkType::IHDR)));
    }

    if !check_signature(chunk.chunk_type().to_bytes()) {
        return Err(CheckError::Chunk(ChunkError::InvalidSignature(
            ChunkType::IHDR,
        )));
    }

    let data = chunk.data();

    if !c_crc32::check_crc32(data, chunk.crc(), ChunkType::IHDR) {
        return Err(CheckError::Chunk(ChunkError::InvalidCrc(ChunkType::IHDR)));
    }

    let width: &[u8; 4] = data[0..4].try_into().unwrap();
    let height: &[u8; 4] = data[4..8].try_into().unwrap();
    let bit_depth = data[8];
    let color_type = data[9];
    let compression_method = data[10];
    let filter_method = data[11];
    let interlace_method = data[12];

    if !check_width_n_height(width, height) {
        return Err(CheckError::Chunk(ChunkError::InvalidDimensions));
    }
    if !validate_color_type_and_bit_depth(color_type, bit_depth) {
        return Err(CheckError::Chunk(ChunkError::InvalidColorTypeOrBitDepth));
    }
    if !check_compression(compression_method) {
        return Err(CheckError::Chunk(ChunkError::InvalidCompressionMethod));
    }
    if !check_filter(filter_method) {
        return Err(CheckError::Chunk(ChunkError::InvalidFilterMethod));
    }
    if !check_interlace(interlace_method) {
        return Err(CheckError::Chunk(ChunkError::InvalidInterlaceMethod));
    }

    Ok((width, height, bit_depth, color_type))
}
