#![doc = "Height and width checker"]

const DIMENSIONS: [u8; 4] = [0, 0, 0, 64];

#[inline(always)]
pub fn check_width_n_height(width: &[u8; 4], height: &[u8; 4]) -> bool {
    let w = check_width(width);
    let h = check_height(height);
    w && h
}

#[inline(always)]
fn check_width(width: &[u8; 4]) -> bool {
    &DIMENSIONS == width
}

#[inline(always)]
fn check_height(height: &[u8; 4]) -> bool {
    &DIMENSIONS == height
}
