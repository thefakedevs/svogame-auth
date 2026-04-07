use std::fmt::Display;

use crate::png_checker::{CheckError, ChunkError};

#[derive(Clone)]
pub(super) struct Chunk {
    length: u32,
    chunk_type: ChunkType,
    data: Vec<u8>,
    crc: u32,
}

#[derive(Clone)]
pub struct ColorFormatInfo {
    #[allow(unused)]
    pub has_alpha: bool,
    pub plte_requirement: PLTERequirement,
    #[allow(unused)]
    pub trns_requirement: tRNSRequirement,
}

#[derive(Clone, Debug)]
pub enum ChunkType {
    IHDR,
    PLTE,
    tRNS,
    IDAT,
    IEND,
    tEXt,
    Other([u8; 4]),
}

#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub enum ColorType {
    IndexedColor = 3,

    TrueColor = 2,
    TrueColorAlpha = 6,

    GrayScale = 0,
    GrayScaleAlpha = 4,
}

#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub enum PLTERequirement {
    Must,
    Optional,
    Disallowed,
}

#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub enum tRNSRequirement {
    Allowed,
    Disallowed,
}

impl Chunk {
    pub fn new(length: u32, chunk_type: ChunkType, data: Vec<u8>, crc: u32) -> Self {
        Chunk {
            length,
            chunk_type,
            data,
            crc,
        }
    }
}

impl Chunk {
    pub fn length(&self) -> u32 {
        self.length
    }

    pub fn chunk_type(&self) -> &ChunkType {
        &self.chunk_type
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn crc(&self) -> u32 {
        self.crc
    }
}

impl ChunkType {
    pub fn from_bytes(bytes: &[u8; 4]) -> Self {
        match bytes {
            b"IHDR" => ChunkType::IHDR,
            b"PLTE" => ChunkType::PLTE,
            b"tRNS" => ChunkType::tRNS,
            b"IDAT" => ChunkType::IDAT,
            b"IEND" => ChunkType::IEND,
            b"tEXt" => ChunkType::tEXt,
            _ => ChunkType::Other(*bytes),
        }
    }

    pub fn to_bytes(&self) -> &[u8; 4] {
        match self {
            ChunkType::IHDR => b"IHDR",
            ChunkType::PLTE => b"PLTE",
            ChunkType::tRNS => b"tRNS",
            ChunkType::IDAT => b"IDAT",
            ChunkType::IEND => b"IEND",
            ChunkType::tEXt => b"tEXt",
            ChunkType::Other(bytes) => bytes,
        }
    }
}

impl TryFrom<u8> for ColorType {
    type Error = CheckError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            3 => Ok(Self::IndexedColor),

            2 => Ok(Self::TrueColor),
            6 => Ok(Self::TrueColorAlpha),

            0 => Ok(Self::GrayScale),
            4 => Ok(Self::GrayScaleAlpha),

            _ => Err(Self::Error::Chunk(ChunkError::InvalidColorTypeOrBitDepth)),
        }
    }
}

impl Display for ColorType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IndexedColor => write!(f, "IndexedColor"),

            Self::GrayScale => write!(f, "GrayScale"),
            Self::GrayScaleAlpha => write!(f, "GrayScaleAlpha"),

            Self::TrueColor => write!(f, "TrueColor"),
            Self::TrueColorAlpha => write!(f, "TrueColorAlpha"),
        }
    }
}

impl Display for PLTERequirement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Must => write!(f, "MustBe"),
            Self::Optional => write!(f, "Optional"),
            Self::Disallowed => write!(f, "NotAllowed"),
        }
    }
}

impl Display for ChunkType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IHDR => write!(f, "IHDR"),
            Self::PLTE => write!(f, "PLTE"),
            Self::tRNS => write!(f, "tRNS"),
            Self::IDAT => write!(f, "IDAT"),
            Self::IEND => write!(f, "IEND"),
            Self::tEXt => write!(f, "tEXt"),
            Self::Other(bytes) => write!(f, "Other({})", String::from_utf8_lossy(bytes)),
        }
    }
}
