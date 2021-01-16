use maligned::{A4096, align_first};

pub mod random;
pub mod pattern;
pub mod readrandom;

pub enum PassType {
    Pattern(bool),
    Random(),
    VeriRandom(),
}

/// create page-aligned buffer
pub fn create_buf(len: usize) -> Vec<u8> {
    let mut buf = align_first::<_,A4096>(len);
    assert_eq!(buf.capacity(),len);
    buf.resize(len, 0);
    assert_eq!(buf.len(),len);
    assert_eq!(buf.capacity(),len);
    buf
}
