use crate::png_checker::MAX_CHUNK_LEN;

pub fn validate_idat_length(
    idat_len: u64,
    width: u32,
    height: u32,
    bit_depth: u8,
    color_type: u8,
) -> bool {
    let bits_per_pixel: u32 = match color_type {
        0 => bit_depth as u32,       // Grayscale
        2 => (bit_depth as u32) * 3, // RGB
        3 => bit_depth as u32,       // Indexed
        4 => (bit_depth as u32) * 2, // Gray+Alpha
        6 => (bit_depth as u32) * 4, // RGBA
        _ => return false,
    };

    let scanline = ((width as u64 * bits_per_pixel as u64 + 7) / 8) + 1;
    let raw_size = scanline.saturating_mul(height as u64);

    #[cfg(debug_assertions)]
    {
        dbg!(idat_len);
        dbg!(raw_size);
        dbg!(MAX_CHUNK_LEN);
        dbg!(idat_len <= MAX_CHUNK_LEN as u64);
        dbg!(idat_len >= 6);
        dbg!(idat_len <= raw_size * 10);
    }

    // min zlib size = 6 (header + adler + 1 data byte)
    // "sane upper bound" — 10× the uncompressed size
    idat_len <= MAX_CHUNK_LEN as u64 && idat_len >= 6 && idat_len <= raw_size * 10
}
