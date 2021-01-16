use std::ops::Range;

pub fn rw_next(off: usize) -> Range<usize> {
    let next = (off>>24 +1)<<24;
    off..next
}
