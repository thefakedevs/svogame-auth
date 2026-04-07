//mod model_inject;
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

mod errors;
pub(super) mod magic_num;
mod types;

mod find_chunk;
mod parse_color;

mod c_crc32;

mod IDAT;
mod IEND;
mod IHDR;
mod PLTE;

mod correct;
mod make;
mod splitter;

pub use errors::*;
pub use types::*;

use crate::png_checker::{
    IDAT::validate_idat_length,
    IEND::validate_iend,
    IHDR::check_and_validate_ihdr,
    PLTE::{check_PLTE, check_palette_useless},
    find_chunk::find_chunk,
    splitter::split_png_chunks,
};

const MAX_CHUNK_LEN: u32 = 2_147_483_647;

pub fn process_png(data: &[u8], model: &str, out: &mut Vec<u8>) -> Result<(), CheckError> {
    if !magic_num::check_magic_num(&data[0..8]) {
        return Err(CheckError::InvalidMagicNumber);
    }

    let mut chunks = split_png_chunks(data)?;

    IDAT::collapse_idat_chunks(&mut chunks)?;
    correct::remove_useless_chunks(&mut chunks);
    correct::inject_model(&model, &mut chunks)?;
    correct::normalize_chunks_order(&mut chunks);

    let ihdr = find_chunk(ChunkType::IHDR.to_bytes(), &chunks)
        .ok_or(CheckError::Chunk(ChunkError::Missing(ChunkType::IHDR)));
    let idat = find_chunk(ChunkType::IDAT.to_bytes(), &chunks)
        .ok_or(CheckError::Chunk(ChunkError::Missing(ChunkType::IDAT)));
    let plte_chunk = find_chunk(ChunkType::PLTE.to_bytes(), &chunks);
    let iend = find_chunk(ChunkType::IEND.to_bytes(), &chunks)
        .ok_or(CheckError::Chunk(ChunkError::Missing(ChunkType::IEND)));

    let (width, height, bit_depth, _color_type) = check_and_validate_ihdr(ihdr?)?;
    let color_type = ColorType::try_from(_color_type)?;

    let color_format_info = parse_color::analyze_color_format(color_type);

    {
        let check_helper = |c: &Chunk| -> Result<(), CheckError> {
            check_PLTE(color_type, c.length(), c.crc(), c.data())
        };

        match check_palette_useless(color_type) {
            PLTERequirement::Must | PLTERequirement::Optional => {
                if let Some(chunk) = plte_chunk {
                    check_helper(chunk)?;
                } else if check_palette_useless(color_type) == PLTERequirement::Must {
                    return Err(CheckError::Chunk(ChunkError::Missing(ChunkType::PLTE)));
                }
            }
            PLTERequirement::Disallowed => {
                if plte_chunk.is_some() {
                    return Err(CheckError::Chunk(ChunkError::PlteNotAllowed {
                        color_type,
                        requirement: color_format_info.plte_requirement,
                    }));
                }
            }
        }
    }

    if !validate_idat_length(
        idat?.length() as u64,
        u32::from_be_bytes(*width),
        u32::from_be_bytes(*height),
        bit_depth,
        _color_type,
    ) {
        return Err(CheckError::Chunk(ChunkError::InvalidLength(
            ChunkType::IDAT,
        )));
    }

    {
        let iend = iend?;
        validate_iend(
            iend.chunk_type().to_bytes(),
            &iend.length().to_be_bytes(),
            iend.crc(),
        )?
    }

    let _png = make::assemble_png(&chunks, out);

    Ok(())
}
