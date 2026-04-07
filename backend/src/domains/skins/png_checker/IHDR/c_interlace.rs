/// If `interlace` (arg) in [[0, 1]] - returns true. Any other value returns false.
#[inline(always)]
pub const fn check_interlace(interlace: u8) -> bool {
    matches!(interlace, 0 | 1)
}
