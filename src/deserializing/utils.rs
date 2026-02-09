use crate::utils::consts::ENDING_FLAG;

#[inline(always)]
unsafe fn decode_number(
    buf: &[u8],
    mut ptr: usize,
    base: u64,
) -> (u64, usize) {
    let b = *buf.get_unchecked(ptr);
    if b != ENDING_FLAG {
        return (b as u64, ptr + 1);
    }

    ptr += 1;

    let mut res: u64 = 0;
    let mut mul: u64 = 1;

    loop {
        let v = *buf.get_unchecked(ptr);
        if v == ENDING_FLAG {
            return (res, ptr + 1);
        }
        res += (v as u64) * mul;
        mul *= base;
        ptr += 1;
    }
}
