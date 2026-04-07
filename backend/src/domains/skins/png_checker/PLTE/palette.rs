use crate::png_checker::{ChunkError, errors::CheckError};

#[inline(always)]
pub fn check_palette_length(len: u32) -> Result<(), CheckError> {
    if len != 0 && (len % 3 != 0) {
        return Err(CheckError::Chunk(ChunkError::PlteInvalidLength));
    }
    Ok(())
}
