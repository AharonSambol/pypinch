use crate::utils::consts::ENDING_FLAG;

#[inline(always)]
pub unsafe fn decode_number<const BASE: u128>(
    buf: &[u8],
    ptr: &mut usize,
) -> u128 {
    let b = *buf.get_unchecked(*ptr);
    *ptr += 1;
    if b != ENDING_FLAG {
        return b as u128;
    }


    let mut res: u128 = 0;
    let mut mul: u128 = 1;

    loop {
        let v = *buf.get_unchecked(*ptr);
        *ptr += 1;
        if v == ENDING_FLAG {
            return res;
        }
        res += (v as u128) * mul;
        mul *= BASE;
    }
}

// #[inline(always)]
// unsafe fn decode_number<const BASE: u128>(
//     buf: &mut *const [u8],
// ) -> u128 {
//     let b = buf[0];
//     *buf = buf.add(1);
//     if b != ENDING_FLAG {
//         return b as u128;
//     }
//
//
//     let mut res: u128 = 0;
//     let mut mul: u128 = 1;
//
//     loop {
//         let v = buf[0];
//         *buf = buf.add(1);
//         if v == ENDING_FLAG {
//             return res;
//         }
//         res += (v as u128) * mul;
//         mul *= BASE;
//     }
// }
