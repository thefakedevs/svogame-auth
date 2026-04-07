#[inline(always)]
pub(super) const fn length_checker(length: &[u8; 4]) -> bool {
    matches!(length, [0, 0, 0, 0])
}

#[inline(always)]
pub(super) const fn chunk_name_checker(chunk_name: &[u8; 4]) -> bool {
    matches!(chunk_name, [0x49, 0x45, 0x4E, 0x44])
}

#[inline(always)]
pub(super) const fn crc_checker(crc: u32) -> bool {
    matches!(crc, 0xAE426082)
}
