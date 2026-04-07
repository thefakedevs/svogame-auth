/// If `compression` (arg) in [[0]] - returns true. Any other value returns false.
#[inline(always)]
pub const fn check_compression(compression: u8) -> bool {
    matches!(compression, 0)
}
