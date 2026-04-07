// const BLOCK_SIZE: [u8; 4] = [0x00, 0x00, 0x00, 0xD];
pub const IHDR_SIGNATURE: &[u8; 4] = &[0x49, 0x48, 0x44, 0x52];

// pub const fn check_block_size(block_size: &[u8; 4]) -> bool {
//     matches!(BLOCK_SIZE, block_size)
// }

pub fn check_signature(signature: &[u8; 4]) -> bool {
    IHDR_SIGNATURE == signature
}
