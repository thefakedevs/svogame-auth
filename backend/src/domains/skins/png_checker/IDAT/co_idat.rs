use crate::png_checker::{Chunk, ChunkType, CheckError, ChunkError, c_crc32};

pub fn collapse_idat_chunks(chunks: &mut Vec<Chunk>) -> Result<(), CheckError> {
    fn helper(chunks: Vec<Chunk>) -> Result<Vec<Chunk>, CheckError> {
        let mut res = Vec::new();
        let mut acc = Vec::new();

        for chunk in chunks {
            if chunk.chunk_type().to_bytes() == b"IDAT" {
                if !c_crc32::check_crc32(chunk.data(), chunk.crc(), ChunkType::IDAT) {
                    return Err(CheckError::Chunk(ChunkError::InvalidCrc(ChunkType::IDAT)));
                }
                acc.extend_from_slice(&chunk.data());
            } else {
                if !acc.is_empty() {
                    let length = acc.len() as u32;
                    let chunk_type = *b"IDAT";

                    let mut hasher = crc32fast::Hasher::new();
                    hasher.update(&chunk_type);
                    hasher.update(&acc);
                    let crc = hasher.finalize();

                    res.push(Chunk::new(
                        length,
                        ChunkType::from_bytes(&chunk_type),
                        acc.clone(),
                        crc,
                    ));
                    acc.clear();
                }

                res.push(chunk);
            }
        }

        if !acc.is_empty() {
            let length = acc.len() as u32;
            let chunk_type = *b"IDAT";

            let mut hasher = crc32fast::Hasher::new();
            hasher.update(&chunk_type);
            hasher.update(&acc);
            let crc = hasher.finalize();

            res.push(Chunk::new(
                length,
                ChunkType::from_bytes(&chunk_type),
                acc.clone(),
                crc,
            ));
        }

        Ok(res)
    }

    *chunks = helper(std::mem::take(chunks))?;
    Ok(())
}
