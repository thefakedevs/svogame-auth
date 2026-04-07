use crc32fast;

use crate::png_checker::ChunkType;

#[inline(always)]
fn calculate_crc32(data: &[u8], chunk_type: ChunkType) -> u32 {
    let mut hasher = crc32fast::Hasher::new();
    hasher.update(chunk_type.to_bytes());
    hasher.update(data);
    hasher.finalize()
}

#[inline(always)]
pub fn check_crc32(data: &[u8], crc: u32, chunk_type: ChunkType) -> bool {
    crc == calculate_crc32(data, chunk_type)
}
