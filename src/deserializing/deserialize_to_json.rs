use crate::deserializing::primitives::{decode_f64_rust, get_sized_pointer_pos};
use crate::deserializing::utils::{decode_number_py_ssize_t, decode_number_usize, DESERIALIZATION_ERROR_TYPE};
use crate::utils::consts::{AMOUNT_OF_USED_FLAGS, ASCII_STR_FLAG, BOOL_FLAG, BYTES_FLAG, CONSISTENT_TYPE_LIST_FLAG, CORRUPTED_DATA, CUSTOM_TYPE_FLAG, DICT_FLAG, EMPTY_BYTES_FLAG, EMPTY_DICT_FLAG, EMPTY_LIST_FLAG, EMPTY_STR_FLAG, ENDING_FLAG, FALSE_FLAG, FLOAT_FLAG, INVALID_UTF_8_START_BYTE_COMPACT_ASCII, LEFTMOST_BIT_MASK, LIST_FLAG, LIST_OF_STRUCTURED_DICTS_FLAG, NEGATIVE_INT_FLAG, NULL_FLAG, NUMBER_BASE, POINTER_FLAG, POINTER_FLAG_1BYTE, POINTER_FLAG_2BYTE, POINTER_FLAG_3BYTE, POINTER_FLAG_4BYTE, POSITIVE_INT_FLAG, STR_FLAG, STR_KEY_DICT_FLAG, TRUE_FLAG, UNEXPECTED_END_OF_INPUT};
use crate::utils::py_dict_key::PyHashMap;
use crate::utils::py_helpers::ToPyErr;
use crate::safe_get;
use num_bigint::BigUint;
use pyo3_ffi::{PyErr_NoMemory, PyObject};
use seq_macro::seq;
use std::fmt::Write;

// TODO: do i need the is_base_254
type Pointers = Vec<(usize, bool /*is_base_254*/)>;


#[macro_export]
macro_rules! numbers_to_strings {
    ($n:literal) => {{
        // seq! evaluates the range at compile time
        seq!(i in 0..$n {
            [
                #(
                    // stringify! turns the integer token into a &str literal at compile time
                    stringify!(i),
                )*
            ]
        })
    }};
}

// TODO: dont return string, return bytes?
pub fn convert_to_json(
    str_buf: &mut String,
    buf: &[u8],
    ptr: &mut usize,
    pointers: &mut Pointers,
    custom_types: &Option<PyHashMap<*mut PyObject>>,
) -> Result<(), *mut PyObject> {
    let flag = *safe_get!(buf, *ptr);

    *ptr += 1;
    match flag {
        POSITIVE_INT_FLAG => decode_large_number_to_string::<NUMBER_BASE>(str_buf, buf, ptr),
        NEGATIVE_INT_FLAG => {
            str_buf.push('-');
            decode_large_number_to_string::<NUMBER_BASE>(str_buf, buf, ptr)
        }
        FLOAT_FLAG => convert_float(str_buf, buf, ptr),
        // TODO: make sure both work
        STR_FLAG | ASCII_STR_FLAG => convert_string::<NUMBER_BASE>(str_buf, buf, ptr, pointers),
        TRUE_FLAG => { str_buf.push_str("true"); Ok(()) },
        FALSE_FLAG => { str_buf.push_str("false"); Ok(()) },
        NULL_FLAG => { str_buf.push_str("null"); Ok(()) },
        POINTER_FLAG => {
            let pos = decode_number_usize::<NUMBER_BASE>(buf, ptr)?;
            convert_from_pointer_position(str_buf, buf, pointers, pos)
        },
        POINTER_FLAG_1BYTE => convert_sized_pointer::<1>(str_buf, buf, ptr, pointers),
        POINTER_FLAG_2BYTE => convert_sized_pointer::<2>(str_buf, buf, ptr, pointers),
        POINTER_FLAG_3BYTE => convert_sized_pointer::<3>(str_buf, buf, ptr, pointers),
        POINTER_FLAG_4BYTE => convert_sized_pointer::<4>(str_buf, buf, ptr, pointers),
        BYTES_FLAG => todo!(),
        EMPTY_BYTES_FLAG => todo!(),
        CONSISTENT_TYPE_LIST_FLAG => {
            let typ = *safe_get!(buf, *ptr);
            *ptr += 1;
            let len = decode_number_usize::<NUMBER_BASE>(buf, ptr)?;
            str_buf.push('[');

            match typ {
                NULL_FLAG => {
                    str_buf.reserve("null, ".len() * len - ", ".len());  // TODO: no need for space
                    for _ in 0..len-1 {
                        str_buf.push_str("null, ");  // TODO: no need for space
                    }
                    str_buf.push_str("null");
                },
                BOOL_FLAG => {
                    // true is the shorter of the booleans, so this is a minimum
                    str_buf.reserve("true, ".len() * len - ", ".len());  // TODO: no need for space

                    let amount_of_bytes = ((len as usize) + 7) >> 3;

                    let mut pos = 0;
                    let table = ["true", "false"];
                    for byte_index in 0..amount_of_bytes {
                        let mut byte = *safe_get!(buf, *ptr + byte_index);
                        for bit_index in 0..8 {
                            if byte_index != 0 || bit_index != 0 {
                                str_buf.push_str(", "); // TODO: no need for space
                            }
                            str_buf.push_str(table[((byte & LEFTMOST_BIT_MASK) == 0) as usize]);
                            pos += 1;
                            if pos == len {
                                break;
                            }
                            byte <<= 1;
                        }
                    }
                    *ptr += amount_of_bytes;
                },
                BYTES_FLAG => todo!(),
                STR_FLAG => {
                    for i in 0..len {
                        if i != 0 {
                            str_buf.push_str(", "); // TODO: no need for space
                        }
                        convert_string::<NUMBER_BASE>(str_buf, buf, ptr, pointers)?;
                    }
                },
                FLOAT_FLAG => {
                    for i in 0..len {
                        if i != 0 {
                            str_buf.push_str(", "); // TODO: no need for space
                        }
                        convert_float(str_buf, buf, ptr)?;
                    }
                },
                _ => return Err("Unexpected consistent list type".to_py_error(unsafe { DESERIALIZATION_ERROR_TYPE })),
            }
            str_buf.push(']');
            Ok(())
        },
        DICT_FLAG => {
            str_buf.push('{');
            let len = decode_number_usize::<NUMBER_BASE>(buf, ptr)?;
            // we know it's not empty because if it was it would have been EMPTY_DICT_FLAG
            convert_to_json(str_buf, buf, ptr, pointers, custom_types)?;
            str_buf.push_str(": "); // TODO: no need for space
            convert_to_json(str_buf, buf, ptr, pointers, custom_types)?;
            for _ in 1..len {
                str_buf.push_str(", "); // TODO: no need for space
                convert_to_json(str_buf, buf, ptr, pointers, custom_types)?;
                str_buf.push_str(": "); // TODO: no need for space
                convert_to_json(str_buf, buf, ptr, pointers, custom_types)?;
            }
            str_buf.push('}');
            Ok(())
        },
        STR_KEY_DICT_FLAG => {
            str_buf.push('{');
            let len = decode_number_usize::<NUMBER_BASE>(buf, ptr)?;
            // we know it's not empty because if it was it would have been EMPTY_DICT_FLAG
            fn convert_dict_key(
                str_buf: &mut String,
                buf: &[u8],
                ptr: &mut usize,
                pointers: &mut Pointers,
            ) -> Result<(), *mut PyObject> {
                if *safe_get!(buf, *ptr) == NUMBER_BASE as u8 - 1 {
                    *ptr += 1;
                    let pos = decode_number_usize::<NUMBER_BASE>(buf, ptr)?;
                    convert_from_pointer_position(str_buf, buf, pointers, pos)
                } else {
                    convert_string::<{ NUMBER_BASE - 1 }>(str_buf, buf, ptr, pointers)
                }
            }
            convert_dict_key(str_buf, buf, ptr, pointers)?;
            str_buf.push_str(": "); // TODO: no need for space
            convert_to_json(str_buf, buf, ptr, pointers, custom_types)?;
            for _ in 1..len {
                str_buf.push_str(", "); // TODO: no need for space
                convert_dict_key(str_buf, buf, ptr, pointers)?;
                str_buf.push_str(": "); // TODO: no need for space
                convert_to_json(str_buf, buf, ptr, pointers, custom_types)?;
            }
            str_buf.push('}');
            Ok(())
        },
        EMPTY_DICT_FLAG => { str_buf.push_str("{}"); Ok(()) },
        EMPTY_LIST_FLAG => { str_buf.push_str("[]"); Ok(()) },
        EMPTY_STR_FLAG => { str_buf.push_str("\"\""); Ok(()) },
        LIST_FLAG => {
            str_buf.push('[');
            let len = decode_number_py_ssize_t::<NUMBER_BASE>(buf, ptr)?;
            // we know it's not empty because if it was it would have been EMPTY_LIST_FLAG
            convert_to_json(str_buf, buf, ptr, pointers, custom_types)?;
            for _ in 1..len {
                str_buf.push_str(", "); // TODO: no need for space
                convert_to_json(str_buf, buf, ptr, pointers, custom_types)?;
            }
            str_buf.push(']');
            Ok(())
        },
        LIST_OF_STRUCTURED_DICTS_FLAG => {
            let list_len = decode_number_usize::<NUMBER_BASE>(buf, ptr)?;
            str_buf.push('[');
            // first dict:
            let dict_len = decode_number_usize::<NUMBER_BASE>(buf, ptr)?;
            let mut keys = Vec::with_capacity(dict_len);
            for i in 0..dict_len {
                if i == 0 {
                    str_buf.push('{');
                } else {
                    str_buf.push_str(", {");
                }
                let key_start = str_buf.len();
                convert_to_json(str_buf, buf, ptr, pointers, custom_types)?;
                keys.push((key_start, str_buf.len()));
                str_buf.push_str(": "); // TODO: no need for space
                convert_to_json(str_buf, buf, ptr, pointers, custom_types)?;
                str_buf.push('}');
            }

            // the rest of the dicts:
            for i in 1usize..list_len {
                for key_index in 0..dict_len {
                    if i == 0 {
                        str_buf.push('{');
                    } else {
                        str_buf.push_str(", {");
                    }
                    let start = keys[key_index].0;
                    let end = keys[key_index].1 + 2; // +2 for `:` and the space after it  // TODO: no need for space
                    push_from_own_slice(str_buf, start, end);

                    convert_to_json(str_buf, buf, ptr, pointers, custom_types)?;
                    str_buf.push('}');
                }
            }
            str_buf.push(']');
            Ok(())
        },
        CUSTOM_TYPE_FLAG => todo!(),
        _ => {
            let numbers = numbers_to_strings!(256);
            match write!(str_buf, "{}", numbers[(flag - AMOUNT_OF_USED_FLAGS) as usize]) {
                Ok(_) => Ok(()),
                Err(_) => Err(unsafe { PyErr_NoMemory() })
            }
        },
    }
}

fn convert_float(str_buf: &mut String, buf: &[u8], ptr: &mut usize) -> Result<(), *mut PyObject> {
    let rust_float = decode_f64_rust(buf, ptr)?;
    let mut buffer = ryu::Buffer::new();
    str_buf.push_str(buffer.format(rust_float));
    Ok(())
}

fn push_from_own_slice(str_buf: &mut String, start: usize, end: usize) {
    str_buf.reserve(end - start);
    let len = str_buf.len();
    unsafe {
        std::ptr::copy_nonoverlapping(
            str_buf.as_ptr().add(start),
            str_buf.as_mut_ptr().add(len),
            end - start,
        );
        str_buf.as_mut_vec().set_len(len + end - start);
    }
}

#[inline(always)]
pub fn decode_large_number_to_string<const BASE: u128>(
    str_buf: &mut String,
    buf: &[u8],
    ptr: &mut usize,
) -> Result<(), *mut PyObject> {
    let byte = *safe_get!(buf, *ptr);
    *ptr += 1;

    if byte != ENDING_FLAG {
        let mut buffer = itoa::Buffer::new();
        str_buf.push_str(buffer.format(byte));
        return Ok(());
    }

    let start_ptr = *ptr;
    let mut end_ptr = start_ptr;
    while *safe_get!(buf, end_ptr) != ENDING_FLAG {
        end_ptr += 1;
    }

    let num_length = end_ptr - start_ptr;

    if num_length <= (u128::BITS / 8) as usize {
        let mut res: u128 = 0;

        for &b in buf[start_ptr..end_ptr].iter().rev() {
            res = res * BASE + (b as u128);
        }
        res += BASE;
        *ptr = end_ptr + 1;
        let mut buffer = itoa::Buffer::new();
        str_buf.push_str(buffer.format(res));
        return Ok(());
    }

    let mut res = BigUint::from(0u32);
    let base_u32 = BASE as u32;

    for &b in buf[start_ptr..end_ptr].iter().rev() {
        res *= base_u32;
        res += b as u32;
    }

    res += BASE;
    *ptr = end_ptr + 1;
    // TODO: check that this works
    if let Err(_) = write!(str_buf, "{res}") {
        return Err(unsafe { PyErr_NoMemory() });
    }
    Ok(())
}


#[inline(always)]
pub fn convert_string<'a, const BASE: u128>(
    str_buf: &mut String,
    buf: &'a [u8],
    ptr: &mut usize,
    pointers: &mut Pointers,
) -> Result<(), *mut PyObject> {
    pointers.push((*ptr, BASE == 254));
    convert_string_without_inserting_pointer::<BASE>(str_buf, &buf, ptr)
}


pub fn convert_string_without_inserting_pointer<'a, const BASE: u128>(str_buf: &mut String, buf: &[u8], ptr: &mut usize) -> Result<(), *mut PyObject> {
    let len = decode_number_usize::<BASE>(buf, ptr)?;
    if *ptr + len > buf.len() {
        return Err(UNEXPECTED_END_OF_INPUT.to_py_error(unsafe { DESERIALIZATION_ERROR_TYPE }));
    }
    let start = *ptr;
    *ptr += len;

    if len == 0 {   // avoid get_unchecked out of bounds for this case
        return Ok(());
    }
    let starts_with_ascii_flag = unsafe { *buf.get_unchecked(start) } == INVALID_UTF_8_START_BYTE_COMPACT_ASCII;
    let str = &buf[start + (starts_with_ascii_flag as usize)..*ptr];

    write_string_escaped(str_buf, str)
}

fn write_string_escaped(str_buf: &mut String, bytes: &[u8]) -> Result<(), *mut PyObject> {
    str_buf.reserve(2 + bytes.len());
    str_buf.push('"');
    inner_write_string_escaped(str_buf, bytes)?;
    str_buf.push('"');
    Ok(())
}

fn inner_write_string_escaped(str_buf: &mut String, bytes: &[u8]) -> Result<(), *mut PyObject> {
    const BB: &'static str = r"\b";
    const TT: &'static str = r"\t";
    const NN: &'static str = r"\n";
    const FF: &'static str = r"\f";
    const RR: &'static str = r"\r";
    const QU: &'static str = "\\\"";
    const BS: &'static str = r"\\";
    const __: &'static str = "";
    const O0: &'static str = r"\u0000";
    const O1: &'static str = r"\u0001";
    const O2: &'static str = r"\u0002";
    const O3: &'static str = r"\u0003";
    const O4: &'static str = r"\u0004";
    const O5: &'static str = r"\u0005";
    const O6: &'static str = r"\u0006";
    const O7: &'static str = r"\u0007";
    const OB: &'static str = r"\u000b";
    const OE: &'static str = r"\u000e";
    const OF: &'static str = r"\u000f";
    const I0: &'static str = r"\u0010";
    const I1: &'static str = r"\u0011";
    const I2: &'static str = r"\u0012";
    const I3: &'static str = r"\u0013";
    const I4: &'static str = r"\u0014";
    const I5: &'static str = r"\u0015";
    const I6: &'static str = r"\u0016";
    const I7: &'static str = r"\u0017";
    const I8: &'static str = r"\u0018";
    const I9: &'static str = r"\u0019";
    const IA: &'static str = r"\u001a";
    const IB: &'static str = r"\u001b";
    const IC: &'static str = r"\u001c";
    const ID: &'static str = r"\u001d";
    const IE: &'static str = r"\u001e";
    const IF: &'static str = r"\u001f";
    static ESCAPE: [&'static str; 256] = [
        //   1   2   3   4   5   6   7   8   9   A   B   C   D   E   F
        O0, O1, O2, O3, O4, O5, O6, O7, BB, TT, NN, OB, FF, RR, OE, OF, // 0
        I0, I1, I2, I3, I4, I5, I6, I7, I8, I9, IA, IB, IC, ID, IE, IF, // 1
        __, __, QU, __, __, __, __, __, __, __, __, __, __, __, __, __, // 2
        __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // 3
        __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // 4
        __, __, __, __, __, __, __, __, __, __, __, __, BS, __, __, __, // 5
        __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // 6
        __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // 7
        __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // 8
        __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // 9
        __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // A
        __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // B
        __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // C
        __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // D
        __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // E
        __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // F
    ];

    let mut start = 0;
    for i in 0..bytes.len() {
        let byte = unsafe { *bytes.get_unchecked(i) };
        let escape = ESCAPE[byte as usize];
        if escape == __ {
            continue;
        }

        if start != i {
            let string_run = match str::from_utf8(&bytes[start..i]) {
                Ok(s) => s,
                Err(_) => return Err(CORRUPTED_DATA.to_py_error(unsafe { DESERIALIZATION_ERROR_TYPE }))
            };
            match write!(str_buf, "{string_run}") {
                Ok(_) => {},
                Err(_) => return Err(unsafe { PyErr_NoMemory() })
            }
        }

        match write!(str_buf, "{escape}") {
            Ok(_) => {},
            Err(_) => return Err(unsafe { PyErr_NoMemory() })   // TODO: is a memory error here possible or should i just `.unwrap()` ?
        }
        start = i + 1;
    }
    if start == bytes.len() {
        return Ok(());
    }

    let string_run = match str::from_utf8(&bytes[start..]) {
        Ok(s) => s,
        Err(_) => return Err(CORRUPTED_DATA.to_py_error(unsafe { DESERIALIZATION_ERROR_TYPE }))
    };
    write!(str_buf, "{string_run}").map_err(|_| unsafe { PyErr_NoMemory() })
}


// TODO: change pointers to point at positions in str (like in structured dict list) instead of pointers to buf
pub fn convert_sized_pointer<const SIZE: usize>(
    str_buf: &mut String,
    buf: &[u8],
    ptr: &mut usize,
    pointers: &mut Pointers,
) -> Result<(), *mut PyObject> {
    if *ptr + SIZE > buf.len() {
        return Err(UNEXPECTED_END_OF_INPUT.to_py_error(unsafe { DESERIALIZATION_ERROR_TYPE }));
    }
    let pos = get_sized_pointer_pos::<SIZE>(buf, ptr);
    convert_from_pointer_position(str_buf, buf, pointers, pos)
}

fn convert_from_pointer_position(str_buf: &mut String, buf: &[u8], pointers: &mut Pointers, pos: usize) -> Result<(), *mut PyObject> {
    match pointers.get(pos) {
        Some((p, is_base_254)) => {
            let mut p = *p;
            if *is_base_254 {
                convert_string_without_inserting_pointer::<254>(str_buf, buf, &mut p)
            } else {
                convert_string_without_inserting_pointer::<NUMBER_BASE>(str_buf, buf, &mut p)
            }
        },
        None => Err(CORRUPTED_DATA.to_py_error(unsafe { DESERIALIZATION_ERROR_TYPE }))
    }
}

