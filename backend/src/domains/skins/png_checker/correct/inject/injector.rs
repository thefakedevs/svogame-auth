use std::io::Write;

use crate::png_checker::{Chunk, ChunkType, InserterError};

/// Adds a tEXt chunk before IEND
pub(super) fn add_tEXt_chunk(
    chunks: &mut Vec<Chunk>,
    keyword: &str,
    text: &str,
) -> Result<(), InserterError> {
    if keyword.is_empty() || keyword.len() > 79 {
        return Err(InserterError::InvalidKeyword {
            keyword: keyword.to_string(),
            reason: "length must be 1–79 bytes",
        });
    }

    if keyword.contains('\0') {
        return Err(InserterError::InvalidKeyword {
            keyword: keyword.to_string(),
            reason: "Contains null byte",
        });
    }

    let mut data = Vec::with_capacity(keyword.len() + 1 + text.len());
    data.write_all(keyword.as_bytes()).unwrap();
    data.push(0);
    data.write_all(text.as_bytes()).unwrap();

    let length = data.len() as u32;
    let chunk_type = ChunkType::tEXt.to_bytes();

    // Calculate CRC
    let mut hasher = crc32fast::Hasher::new();
    hasher.update(chunk_type);
    hasher.update(&data);
    let crc = hasher.finalize();

    // Find IEND pos and insert before it
    // Why not "find_chunk"? This function... make more errors than efficency here :P
    let iend_pos = chunks
        .iter()
        .position(|c| &c.chunk_type().to_bytes() == &b"IEND")
        .unwrap_or(chunks.len());

    let text_chunk = Chunk::new(length, ChunkType::from_bytes(chunk_type), data, crc);

    chunks.insert(iend_pos, text_chunk);
    Ok(())
}
