mod inject;
mod no_order;
mod remove_all;

use remove_all::remove_chunks;

pub use inject::inject_model;
pub use no_order::normalize_chunks_order;

use crate::png_checker::Chunk;

/// Delete all chunks except "MAINLINE" ([["IHDR", "PLTE", "tRNS", "IDAT", "tEXt", "IEND"]]) chunks
pub fn remove_useless_chunks(chunks: &mut Vec<Chunk>) {
    remove_chunks(chunks)
}
