use crate::png_checker::{Chunk, magic_num::MAGIC_NUM};

#[inline(always)]
pub fn assemble_png(chunks: &[Chunk], out: &mut Vec<u8>) {
    out.reserve(MAGIC_NUM.len() + chunks.iter().map(|c| 12 + c.data().len()).sum::<usize>());

    // PNG SIG
    out.extend_from_slice(&MAGIC_NUM);

    for chunk in chunks {
        out.extend_from_slice(&chunk.length().to_be_bytes());
        out.extend_from_slice(chunk.chunk_type().to_bytes());
        out.extend_from_slice(&chunk.data());
        out.extend_from_slice(&chunk.crc().to_be_bytes());
    }
}
