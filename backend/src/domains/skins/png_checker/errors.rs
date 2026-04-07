use thiserror::Error;

use crate::png_checker::{ChunkType, ColorType, PLTERequirement};

use crate::png_checker::ChunkType as ChunkName;

#[derive(Error, Debug)]
pub enum CheckError {
    #[error("Invalid PNG magic number")]
    InvalidMagicNumber,

    #[error(transparent)]
    Chunk(#[from] ChunkError),

    #[error(transparent)]
    FileSize(#[from] FileSizeError),

    #[error(transparent)]
    Splitter(#[from] SplitterError),

    #[error(transparent)]
    Inserter(#[from] InserterError),

    #[error(transparent)]
    Maker(#[from] PngMakerError),
}

// ============================================================================
// Universal Chunk Errors
// ============================================================================

#[derive(Error, Debug)]
pub enum ChunkError {
    #[error("{0} chunk not found")]
    Missing(ChunkName),

    #[error("{0} chunk has invalid length")]
    InvalidLength(ChunkName),

    #[error("{0} chunk has invalid size")]
    InvalidSize(ChunkName),

    #[error("{0} chunk has invalid signature")]
    InvalidSignature(ChunkName),

    #[error("{0} chunk has invalid content")]
    InvalidContent(ChunkName),

    #[error("{0} chunk has invalid name")]
    InvalidName(ChunkName),

    #[error("{0} CRC checksum is invalid")]
    InvalidCrc(ChunkName),

    #[error("File width or height is invalid")]
    InvalidDimensions,

    // IHDR-specific
    #[error("IHDR compression method is invalid")]
    InvalidCompressionMethod,

    #[error("IHDR filter method is invalid")]
    InvalidFilterMethod,

    #[error("IHDR interlace method is invalid")]
    InvalidInterlaceMethod,

    #[error("IHDR color type or bit depth combination is invalid")]
    InvalidColorTypeOrBitDepth,

    #[error("Invalid IHDR parameter: {0}")]
    InvalidParameter(#[from] IhdrParameterError),

    // PLTE-specific
    #[error("PLTE chunk length must be divisible by 3")]
    PlteInvalidLength,

    #[error("PLTE chunk is not allowed for color type {color_type} (requirement: {requirement})")]
    PlteNotAllowed {
        color_type: ColorType,
        requirement: PLTERequirement,
    },
}

#[derive(Error, Debug)]
pub enum IhdrParameterError {
    #[error("color type")]
    ColorType,

    #[error("compression method")]
    CompressionMethod,

    #[error("filter method")]
    FilterMethod,

    #[error("interlace method")]
    InterlaceMethod,
}

// ============================================================================
// File Size Errors
// ============================================================================

#[derive(Error, Debug)]
pub enum FileSizeError {
    #[error("File size exceeds maximum allowed limit")]
    TooLarge,

    #[error("File size is below minimum required size")]
    TooSmall,
}

// ============================================================================
// Processing Errors
// ============================================================================

#[derive(Error, Debug)]
pub enum InserterError {
    #[error("Failed to insert skin model metadata")]
    ModelInsertFailed,

    #[error("Invalid keyword '{keyword}': {reason}")]
    InvalidKeyword {
        keyword: String,
        reason: &'static str,
    },
}

#[derive(Error, Debug)]
pub enum SplitterError {
    #[error("Failed to read chunks: {0}")]
    ReadFailed(String),

    #[error("Too many chunks in PNG file (max: {0})")]
    TooManyChunks(usize),

    #[error("Chunk size {0} exceeds maximum allowed {1} bytes")]
    ChunkTooLarge(u32, u32),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

#[derive(Error, Debug)]
pub enum PngMakerError {
    #[error("Missing required chunk: {0}")]
    MissingChunk(ChunkType),

    #[error("Missing one or more required chunks: {0}")]
    MissingMultipleChunks(ChunkType),
}
