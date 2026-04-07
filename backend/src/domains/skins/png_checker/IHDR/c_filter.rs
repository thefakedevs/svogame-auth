/// If `filter` (arg) in [[0]] - returns true. Any other value returns false.
#[inline(always)]
pub const fn check_filter(filter: u8) -> bool {
    matches!(filter, 0)
}
