
pub const NUMBER_BASE: u8 = 255;
pub const ENDING_FLAG: u8 = 255;
pub const HEADER: &[u8] = b"<o>";

pub const POSITIVE_INT_FLAG: u8 = 0;
pub const NEGATIVE_INT_FLAG: u8 = 1;
pub const INT_FLAG: u8 = 2;
pub const FLOAT_FLAG: u8 = 3;
pub const STR_FLAG: u8 = 4;
pub const EMPTY_STR_FLAG: u8 = 5;
pub const BYTES_FLAG: u8 = 6;
pub const EMPTY_BYTES_FLAG: u8 = 7;
pub const BOOL_FLAG: u8 = 8;
pub const TRUE_FLAG: u8 = 9;
pub const FALSE_FLAG: u8 = 10;
pub const NULL_FLAG: u8 = 11;
pub const LIST_FLAG: u8 = 12;
pub const EMPTY_LIST_FLAG: u8 = 13;
pub const CONSISTENT_TYPE_LIST_FLAG: u8 = 14;
pub const DICT_FLAG: u8 = 15;
pub const EMPTY_DICT_FLAG: u8 = 16;
pub const STR_KEY_DICT_FLAG: u8 = 17;
pub const CONSISTENT_TYPE_DICT_FLAG: u8 = 18;
pub const POINTER_FLAG: u8 = 19;

/// SMALL_INTS: Python {i: byte}
pub const fn small_int(n: i64) -> Option<u8> {
    if n >= 0 && n < 235 {
        Some((20 + n) as u8)
    } else {
        None
    }
}