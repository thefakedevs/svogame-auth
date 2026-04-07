use crate::png_checker::Chunk;

pub fn find_chunk<'a>(chunk_type: &[u8; 4], chunks: &'a Vec<Chunk>) -> Option<&'a Chunk> {
    chunks
        .iter()
        .find(|c| c.chunk_type().to_bytes() == chunk_type)
}
