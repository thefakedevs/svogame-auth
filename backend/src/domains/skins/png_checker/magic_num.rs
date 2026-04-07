pub const MAGIC_NUM: [u8; 8] = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];

#[inline(always)]
pub fn check_magic_num(magic_num: &[u8]) -> bool {
    &MAGIC_NUM == magic_num
}
