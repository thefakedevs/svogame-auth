use crate::png_checker::{Chunk, ChunkType, SplitterError};
use std::io::{Cursor, Read, Seek, SeekFrom};

const MAX_CHUNKS: usize = 100;
const MAX_CHUNK_SIZE: u32 = 20 * 1024;

pub fn split_png_chunks(png_bytes: &[u8]) -> Result<Vec<Chunk>, SplitterError> {
    let mut chunks = Vec::new();
    let mut cursor = Cursor::new(png_bytes);

    cursor.seek(SeekFrom::Start(8))?;

    while (cursor.position() as usize) < png_bytes.len() {
        if chunks.len() >= MAX_CHUNKS {
            return Err(SplitterError::TooManyChunks(MAX_CHUNKS));
        }

        // Length
        let mut length_bytes = [0; 4];
        if cursor.read_exact(&mut length_bytes).is_err() {
            break;
        }
        let length = u32::from_be_bytes(length_bytes);

        if length > MAX_CHUNK_SIZE {
            return Err(SplitterError::ChunkTooLarge(length, MAX_CHUNK_SIZE));
        }

        let mut chunk_type = [0; 4];
        cursor.read_exact(&mut chunk_type)?;

        // Chunk Data
        let mut data = vec![0; length as usize];
        cursor.read_exact(&mut data)?;

        // CRC
        let mut crc_bytes = [0; 4];
        cursor.read_exact(&mut crc_bytes)?;
        let crc = u32::from_be_bytes(crc_bytes);

        chunks.push(Chunk::new(
            length,
            ChunkType::from_bytes(&chunk_type),
            data,
            crc,
        ));

        if &chunk_type == b"IEND" {
            break;
        }
    }

    Ok(chunks)
}
