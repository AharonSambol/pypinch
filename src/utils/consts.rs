pub const NUMBER_BASE: u128 = 255;
pub const NUMBER_BASE_USIZE: usize = NUMBER_BASE as usize;
pub const ENDING_FLAG: u8 = 255;
pub const NEGATIVE_NUMBER_SIGN: u128 = NUMBER_BASE - 1;
pub const LEFTMOST_BIT_MASK: u8 = 128;
pub const HEADER: &[u8] = b"<o>";
pub const INVALID_UTF_8_START_BYTE: u8 = 0xff;

pub const EMPTY_STR_FLAG: u8 = 0;
pub const EMPTY_BYTES_FLAG: u8 = 1;
pub const TRUE_FLAG: u8 = 2;
pub const FALSE_FLAG: u8 = 3;
pub const NULL_FLAG: u8 = 4;
pub const EMPTY_LIST_FLAG: u8 = 5;
pub const EMPTY_DICT_FLAG: u8 = 6;
pub const POSITIVE_INT_FLAG: u8 = 7;
pub const NEGATIVE_INT_FLAG: u8 = 8;
pub const INT_FLAG: u8 = 9;
pub const FLOAT_FLAG: u8 = 10;
pub const STR_FLAG: u8 = 11;
pub const BYTES_FLAG: u8 = 12;
pub const BOOL_FLAG: u8 = 13;
pub const LIST_FLAG: u8 = 14;
pub const CONSISTENT_TYPE_LIST_FLAG: u8 = 15;
pub const DICT_FLAG: u8 = 16;
pub const STR_KEY_DICT_FLAG: u8 = 17;
pub const POINTER_FLAG: u8 = 18;

pub const AMOUNT_OF_USED_FLAGS: u8 = 30; // for future flags

pub const NOT_A_STR_BUT_A_POINTER_FLAG: [u8; 2] = [
    AMOUNT_OF_USED_FLAGS + 1,   // a str of length 1
    INVALID_UTF_8_START_BYTE,   // sike, not really
];
