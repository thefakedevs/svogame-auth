use crate::png_checker::Chunk;

const MAINLINE: [&[u8; 4]; 5] = [b"IHDR", b"PLTE", b"tRNS", b"IDAT", b"IEND"];

pub(super) fn remove_chunks(chunks: &mut Vec<Chunk>) {
    chunks.retain(|chunk| {
        MAINLINE
            .iter()
            .any(|&core| &chunk.chunk_type().to_bytes() == &core)
    });
}
