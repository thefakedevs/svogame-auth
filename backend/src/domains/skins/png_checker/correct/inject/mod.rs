use crate::png_checker::{Chunk, correct::inject::injector::add_tEXt_chunk};

mod injector;

#[inline(always)]
pub fn inject_model(
    model: &str,
    chunks: &mut Vec<Chunk>,
) -> Result<(), crate::png_checker::InserterError> {
    add_tEXt_chunk(chunks, "Comment", &*format!("model={}", model))
}
