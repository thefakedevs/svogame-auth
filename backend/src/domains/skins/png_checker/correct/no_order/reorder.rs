use crate::png_checker::{Chunk, ChunkType};

fn chunk_order(chunk: &Chunk) -> u8 {
    match chunk.chunk_type() {
        ChunkType::IHDR => 0,
        ChunkType::PLTE => 1,
        ChunkType::tRNS => 2,
        ChunkType::IDAT => 3,
        ChunkType::tEXt => 4,
        ChunkType::IEND => 5,
        ChunkType::Other(_) => 255, // всё остальное в конец
    }
}

/// Упорядочивает и очищает чанки PNG in-place.
/// Удаляет все неизвестные чанки, оставляя только:
/// IHDR → PLTE → tRNS → IDAT(s) → tEXt → IEND
pub fn normalize_chunks_order_in_place(chunks: &mut Vec<Chunk>) {
    // first - filter allowed
    chunks.retain(|chunk| {
        matches!(
            chunk.chunk_type(),
            ChunkType::IHDR
                | ChunkType::PLTE
                | ChunkType::tRNS
                | ChunkType::IDAT
                | ChunkType::tEXt
                | ChunkType::IEND
        )
    });
    // after check - reorder)
    chunks.sort_by_key(chunk_order);
}
