#![doc = "Color Type and Bit Depth Checker"]

/// Returns array of allowed bit depths for given color type.
#[inline(always)]
const fn valid_bit_depths_by_color_type(color_type: u8) -> &'static [u8] {
    match color_type {
        0u8 => &[1, 2, 4, 8, 16], // Grayscale
        2u8 => &[8, 16],          // Truecolor
        3u8 => &[1, 2, 4, 8],     // Indexed
        4u8 => &[8, 16],          // Grayscale+Alpha
        6u8 => &[8, 16],          // Truecolor+Alpha
        _ => &[],
    }
}

/// Validates color type and bit depth.
#[inline(always)]
pub fn validate_color_type_and_bit_depth(color_type: u8, bit_depth: u8) -> bool {
    validate_color_type(color_type)
        && validate_bit_depth(bit_depth)
        && valid_bit_depths_by_color_type(color_type).contains(&bit_depth)
}

/// If `color_type` (arg) in [[0, 2, 3, 4, 6]] - returns true. Any other value returns false.
#[inline(always)]
const fn validate_color_type(color_type: u8) -> bool {
    matches!(color_type, 0 | 2 | 3 | 4 | 6)
}

/// If `bit_depth` (arg) in [[1, 2, 4, 8, 16]] - returns true. Any other value returns false.
#[inline(always)]
const fn validate_bit_depth(bit_depth: u8) -> bool {
    matches!(bit_depth, 1 | 2 | 4 | 8 | 16)
}
