use crate::deserializing::custom_types::deserialize_custom_type;
use crate::deserializing::deserialize::deserialize_object;
use crate::deserializing::pointer_holders::position_pointer_holder::Pointer::{Position, Str};
use crate::deserializing::pointer_holders::position_pointer_holder::PositionPointerHolder;
use crate::deserializing::primitives::{decode_f64_rust, get_sized_pointer_pos};
use crate::deserializing::utils::{
    decode_number_py_ssize_t, decode_number_usize, DESERIALIZATION_ERROR_TYPE,
};
use crate::safe_get;
use crate::serializing::py_bytes_buffer::{MemoryPyBytesBuffer, PyBytesBuffer};
use crate::utils::consts::{
    AMOUNT_OF_USED_FLAGS, ASCII_STR_FLAG, BOOL_FLAG, BYTES_FLAG, CONSISTENT_TYPE_LIST_FLAG,
    CUSTOM_TYPE_FLAG, DICT_FLAG, EMPTY_BYTES_FLAG, EMPTY_DICT_FLAG, EMPTY_LIST_FLAG,
    EMPTY_STR_FLAG, ENDING_FLAG, FALSE_FLAG, FLOAT_FLAG, INVALID_UTF_8_START_BYTE_COMPACT_ASCII,
    LEFTMOST_BIT_MASK, LIST_FLAG, LIST_OF_STRUCTURED_DICTS_FLAG, NEGATIVE_INT_FLAG, NULL_FLAG,
    NUMBER_BASE, POINTER_FLAG, POINTER_FLAG_1BYTE, POINTER_FLAG_2BYTE, POINTER_FLAG_3BYTE,
    POINTER_FLAG_4BYTE, POSITIVE_INT_FLAG, STR_FLAG, STR_KEY_DICT_FLAG, TRUE_FLAG,
    UNEXPECTED_END_OF_INPUT,
};
use crate::utils::py_dict_key::{PyHashMap, PyKey};
use crate::utils::py_helpers::{
    pretty_type, py_str_to_rust_str, raise_exception_with_cause, temporary_tuple_of, ToPyErr,
};
use crate::utils::safe_py_pointer::PyPointer;
use num_bigint::BigUint;
use pyo3_ffi::{
    PyBytes_Check, PyBytes_FromStringAndSize, PyBytes_Size, PyObject, PyObject_CallObject,
    PyObject_Str, PyUnicode_AsUTF8AndSize, Py_ssize_t,
};
use seq_macro::seq;
use std::ffi::c_char;
use std::ptr;

#[macro_export]
macro_rules! numbers_to_strings {
    ($n:literal) => {{
        seq!(i in 0..$n {
            [
                #(
                    stringify!(i).as_bytes(),
                )*
            ]
        })
    }};
}

pub fn convert_to_json<'a>(
    output_buf: &mut MemoryPyBytesBuffer,
    buf: &'a [u8],
    ptr: &mut usize,
    pointers: &mut PositionPointerHolder<'a>,
    bytes_converter: *mut PyObject,
    deserializing_custom_types: &Option<PyHashMap<*mut PyObject>>,
    serialization_custom_types: &Option<PyHashMap<*mut PyObject>>,
) -> Result<(), *mut PyObject> {
    let flag = *safe_get!(buf, *ptr);

    *ptr += 1;
    match flag {
        POSITIVE_INT_FLAG => decode_large_number_to_string::<NUMBER_BASE>(output_buf, buf, ptr),
        NEGATIVE_INT_FLAG => {
            output_buf.push('-' as u8)?;
            decode_large_number_to_string::<NUMBER_BASE>(output_buf, buf, ptr)
        }
        FLOAT_FLAG => convert_float(output_buf, buf, ptr),
        // TODO: make sure both work
        STR_FLAG | ASCII_STR_FLAG => convert_string::<NUMBER_BASE>(output_buf, buf, ptr, pointers),
        TRUE_FLAG => output_buf.extend_from_slice(b"true"),
        FALSE_FLAG => output_buf.extend_from_slice(b"false"),
        NULL_FLAG => output_buf.extend_from_slice(b"null"),
        POINTER_FLAG => {
            let pos = decode_number_usize::<NUMBER_BASE>(buf, ptr)?;
            convert_from_pointer_position(output_buf, buf, pointers, pos)
        }
        POINTER_FLAG_1BYTE => convert_sized_pointer::<1>(output_buf, buf, ptr, pointers),
        POINTER_FLAG_2BYTE => convert_sized_pointer::<2>(output_buf, buf, ptr, pointers),
        POINTER_FLAG_3BYTE => convert_sized_pointer::<3>(output_buf, buf, ptr, pointers),
        POINTER_FLAG_4BYTE => convert_sized_pointer::<4>(output_buf, buf, ptr, pointers),
        BYTES_FLAG => convert_bytes(output_buf, buf, ptr, bytes_converter),
        EMPTY_BYTES_FLAG => convert_bytes_slice(output_buf, &[], bytes_converter),
        CONSISTENT_TYPE_LIST_FLAG => {
            let typ = *safe_get!(buf, *ptr);
            *ptr += 1;
            let len = decode_number_usize::<NUMBER_BASE>(buf, ptr)?;
            output_buf.push('[' as u8)?;

            match typ {
                NULL_FLAG => {
                    output_buf.ensure_capacity(", null".len() * len - ", ".len())?; // TODO: no need for space
                    output_buf.extend_from_slice_unchecked(b"null");
                    for _ in 1..len {
                        output_buf.extend_from_slice_unchecked(b", null"); // TODO: no need for space
                    }
                }
                BOOL_FLAG => {
                    // true is the shorter of the booleans, so this is a minimum
                    output_buf.ensure_capacity("true, ".len() * len - ", ".len())?; // TODO: no need for space

                    let amount_of_bytes = (len + 7) >> 3;

                    let mut pos = 0;
                    let table: [&[u8]; 2] = [b"true", b"false"];
                    for byte_index in 0..amount_of_bytes {
                        let mut byte = *safe_get!(buf, *ptr + byte_index);
                        for bit_index in 0..8 {
                            if byte_index != 0 || bit_index != 0 {
                                output_buf.extend_from_slice(b", ")?; // TODO: no need for space
                            }
                            output_buf.extend_from_slice(
                                table[((byte & LEFTMOST_BIT_MASK) == 0) as usize],
                            )?;
                            pos += 1;
                            if pos == len {
                                break;
                            }
                            byte <<= 1;
                        }
                    }
                    *ptr += amount_of_bytes;
                }
                BYTES_FLAG => {
                    convert_bytes(output_buf, buf, ptr, bytes_converter)?;
                    for _ in 1..len {
                        output_buf.extend_from_slice(b", ")?; // TODO: no need for space
                        convert_bytes(output_buf, buf, ptr, bytes_converter)?;
                    }
                }
                STR_FLAG => {
                    convert_string::<NUMBER_BASE>(output_buf, buf, ptr, pointers)?;
                    for _ in 1..len {
                        output_buf.extend_from_slice(b", ")?; // TODO: no need for space
                        convert_string::<NUMBER_BASE>(output_buf, buf, ptr, pointers)?;
                    }
                }
                FLOAT_FLAG => {
                    convert_float(output_buf, buf, ptr)?;
                    for _ in 1..len {
                        output_buf.extend_from_slice(b", ")?; // TODO: no need for space
                        convert_float(output_buf, buf, ptr)?;
                    }
                }
                _ => {
                    return Err("Unexpected consistent list type"
                        .to_py_error(unsafe { DESERIALIZATION_ERROR_TYPE }))
                }
            }
            output_buf.push(']' as u8)
        }
        DICT_FLAG => {
            output_buf.push('{' as u8)?;
            let len = decode_number_usize::<NUMBER_BASE>(buf, ptr)?;
            // we know it's not empty because if it was it would have been EMPTY_DICT_FLAG
            convert_to_json(
                output_buf,
                buf,
                ptr,
                pointers,
                bytes_converter,
                deserializing_custom_types,
                serialization_custom_types,
            )?;
            output_buf.extend_from_slice(b": ")?; // TODO: no need for space
            convert_to_json(
                output_buf,
                buf,
                ptr,
                pointers,
                bytes_converter,
                deserializing_custom_types,
                serialization_custom_types,
            )?;
            for _ in 1..len {
                output_buf.extend_from_slice(b", ")?; // TODO: no need for space
                convert_to_json(
                    output_buf,
                    buf,
                    ptr,
                    pointers,
                    bytes_converter,
                    deserializing_custom_types,
                    serialization_custom_types,
                )?;
                output_buf.extend_from_slice(b": ")?; // TODO: no need for space
                convert_to_json(
                    output_buf,
                    buf,
                    ptr,
                    pointers,
                    bytes_converter,
                    deserializing_custom_types,
                    serialization_custom_types,
                )?;
            }
            output_buf.push('}' as u8)
        }
        STR_KEY_DICT_FLAG => {
            output_buf.push('{' as u8)?;
            let len = decode_number_usize::<NUMBER_BASE>(buf, ptr)?;
            // we know it's not empty because if it was it would have been EMPTY_DICT_FLAG
            fn convert_dict_key<'a>(
                output_buf: &mut MemoryPyBytesBuffer,
                buf: &'a [u8],
                ptr: &mut usize,
                pointers: &mut PositionPointerHolder<'a>,
            ) -> Result<(), *mut PyObject> {
                if *safe_get!(buf, *ptr) == NUMBER_BASE as u8 - 1 {
                    *ptr += 1;
                    let pos = decode_number_usize::<NUMBER_BASE>(buf, ptr)?;
                    convert_from_pointer_position(output_buf, buf, pointers, pos)
                } else {
                    convert_string::<{ NUMBER_BASE - 1 }>(output_buf, buf, ptr, pointers)
                }
            }
            convert_dict_key(output_buf, buf, ptr, pointers)?;
            output_buf.extend_from_slice(b": ")?; // TODO: no need for space
            convert_to_json(
                output_buf,
                buf,
                ptr,
                pointers,
                bytes_converter,
                deserializing_custom_types,
                serialization_custom_types,
            )?;

            for _ in 1..len {
                output_buf.extend_from_slice(b", ")?; // TODO: no need for space
                convert_dict_key(output_buf, buf, ptr, pointers)?;
                output_buf.extend_from_slice(b": ")?; // TODO: no need for space
                convert_to_json(
                    output_buf,
                    buf,
                    ptr,
                    pointers,
                    bytes_converter,
                    deserializing_custom_types,
                    serialization_custom_types,
                )?;
            }
            output_buf.push('}' as u8)
        }
        EMPTY_DICT_FLAG => output_buf.extend_from_slice(b"{}"),
        EMPTY_LIST_FLAG => output_buf.extend_from_slice(b"[]"),
        EMPTY_STR_FLAG => output_buf.extend_from_slice(b"\"\""),
        LIST_FLAG => {
            output_buf.push('[' as u8)?;
            let len = decode_number_py_ssize_t::<NUMBER_BASE>(buf, ptr)?;
            // we know it's not empty because if it was it would have been EMPTY_LIST_FLAG
            convert_to_json(
                output_buf,
                buf,
                ptr,
                pointers,
                bytes_converter,
                deserializing_custom_types,
                serialization_custom_types,
            )?;
            for _ in 1..len {
                output_buf.extend_from_slice(b", ")?; // TODO: no need for space
                convert_to_json(
                    output_buf,
                    buf,
                    ptr,
                    pointers,
                    bytes_converter,
                    deserializing_custom_types,
                    serialization_custom_types,
                )?;
            }
            output_buf.push(']' as u8)
        }
        LIST_OF_STRUCTURED_DICTS_FLAG => {
            let list_len = decode_number_usize::<NUMBER_BASE>(buf, ptr)?;
            output_buf.push('[' as u8)?;
            // first dict:
            let dict_len = decode_number_usize::<NUMBER_BASE>(buf, ptr)?;
            let mut keys = Vec::with_capacity(dict_len);
            for i in 0..dict_len {
                if i == 0 {
                    output_buf.push('{' as u8)?;
                } else {
                    output_buf.extend_from_slice(b", {")?;
                }
                let key_start = output_buf.len();
                convert_to_json(
                    output_buf,
                    buf,
                    ptr,
                    pointers,
                    bytes_converter,
                    deserializing_custom_types,
                    serialization_custom_types,
                )?;
                keys.push((key_start, output_buf.len()));
                output_buf.extend_from_slice(b": ")?; // TODO: no need for space
                convert_to_json(
                    output_buf,
                    buf,
                    ptr,
                    pointers,
                    bytes_converter,
                    deserializing_custom_types,
                    serialization_custom_types,
                )?;
                output_buf.push('}' as u8)?;
            }

            // the rest of the dicts:
            for i in 1usize..list_len {
                for key_index in 0..dict_len {
                    if i == 0 {
                        output_buf.push('{' as u8)?;
                    } else {
                        output_buf.extend_from_slice(b", {")?;
                    }
                    let start = keys[key_index].0;
                    let end = keys[key_index].1 + 2; // +2 for `:` and the space after it  // TODO: no need for space
                    output_buf.extend_from_own_slice(start, end)?;

                    convert_to_json(
                        output_buf,
                        buf,
                        ptr,
                        pointers,
                        bytes_converter,
                        deserializing_custom_types,
                        serialization_custom_types,
                    )?;
                    output_buf.push('}' as u8)?;
                }
            }
            output_buf.push(']' as u8)
        }
        CUSTOM_TYPE_FLAG => {
            let mut fake_pointer = *ptr;
            let type_identifier = PyPointer::new(deserialize_object(
                buf,
                &mut fake_pointer,
                pointers,
                false,
                deserializing_custom_types,
            )?);
            let deserialized_obj =
                deserialize_custom_type(buf, ptr, pointers, false, deserializing_custom_types)?;
            convert_custom_type(
                output_buf,
                deserialized_obj.as_ptr(),
                serialization_custom_types,
                type_identifier,
            )
        }
        _ => {
            let numbers = numbers_to_strings!(256);
            output_buf.extend_from_slice(numbers[(flag - AMOUNT_OF_USED_FLAGS) as usize])
        }
    }
}

fn convert_bytes(
    output_buf: &mut MemoryPyBytesBuffer,
    buf: &[u8],
    ptr: &mut usize,
    bytes_converter: *mut PyObject,
) -> Result<(), *mut PyObject> {
    let len = decode_number_usize::<NUMBER_BASE>(buf, ptr)?;
    if len + *ptr > buf.len() {
        return Err(UNEXPECTED_END_OF_INPUT.to_py_error(unsafe { DESERIALIZATION_ERROR_TYPE }));
    }
    let bytes = &buf[*ptr..*ptr + len];
    *ptr += len;
    convert_bytes_slice(output_buf, bytes, bytes_converter)
}

fn convert_float<'a>(
    output_buf: &mut MemoryPyBytesBuffer,
    buf: &'a [u8],
    ptr: &mut usize,
) -> Result<(), *mut PyObject> {
    let rust_float = decode_f64_rust(buf, ptr)?;
    let mut buffer = ryu::Buffer::new();
    output_buf.extend_from_slice(buffer.format(rust_float).as_bytes())
}

#[inline(always)]
pub fn decode_large_number_to_string<'a, const BASE: u128>(
    output_buf: &mut MemoryPyBytesBuffer,
    buf: &'a [u8],
    ptr: &mut usize,
) -> Result<(), *mut PyObject> {
    let byte = *safe_get!(buf, *ptr);
    *ptr += 1;

    if byte != ENDING_FLAG {
        let mut buffer = itoa::Buffer::new();
        let result = output_buf.extend_from_slice(buffer.format(byte).as_bytes());
        return result;
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
        let result = output_buf.extend_from_slice(buffer.format(res).as_bytes());
        return result;
    }

    let mut res = BigUint::from(0u32);
    let base_u32 = BASE as u32;

    for &b in buf[start_ptr..end_ptr].iter().rev() {
        res *= base_u32;
        res += b as u32;
    }

    res += BASE;
    *ptr = end_ptr + 1;
    output_buf.extend_from_slice(res.to_string().as_bytes())
}

#[inline(always)]
pub fn convert_string<'a, const BASE: u128>(
    output_buf: &mut MemoryPyBytesBuffer,
    buf: &'a [u8],
    ptr: &mut usize,
    pointers: &mut PositionPointerHolder<'a>,
) -> Result<(), *mut PyObject> {
    pointers.insert_position(*ptr, BASE == 254);
    convert_string_without_inserting_pointer::<BASE>(output_buf, &buf, ptr)
}

pub fn convert_string_without_inserting_pointer<'a, const BASE: u128>(
    output_buf: &mut MemoryPyBytesBuffer,
    buf: &'a [u8],
    ptr: &mut usize,
) -> Result<(), *mut PyObject> {
    let len = decode_number_usize::<BASE>(buf, ptr)?;
    if *ptr + len > buf.len() {
        return Err(UNEXPECTED_END_OF_INPUT.to_py_error(unsafe { DESERIALIZATION_ERROR_TYPE }));
    }
    let start = *ptr;
    *ptr += len;

    if len == 0 {
        // avoid get_unchecked out of bounds for this case
        return Ok(());
    }
    let starts_with_ascii_flag =
        unsafe { *buf.get_unchecked(start) } == INVALID_UTF_8_START_BYTE_COMPACT_ASCII;
    let str = &buf[start + (starts_with_ascii_flag as usize)..*ptr];

    write_string_escaped(output_buf, str)
}

fn write_string_escaped(
    output_buf: &mut MemoryPyBytesBuffer,
    bytes: &[u8],
) -> Result<(), *mut PyObject> {
    output_buf.ensure_capacity(2 + bytes.len())?;
    output_buf.push_unchecked('"' as u8);
    inner_write_string_escaped(output_buf, bytes)?; // we don't now the exact length because some chars might need to be escaped
    output_buf.push('"' as u8)
}

fn inner_write_string_escaped(
    output_buf: &mut MemoryPyBytesBuffer,
    bytes: &[u8],
) -> Result<(), *mut PyObject> {
    const BB: &[u8] = r"\b".as_bytes();
    const TT: &[u8] = r"\t".as_bytes();
    const NN: &[u8] = r"\n".as_bytes();
    const FF: &[u8] = r"\f".as_bytes();
    const RR: &[u8] = r"\r".as_bytes();
    const QU: &[u8] = "\\\"".as_bytes();
    const BS: &[u8] = r"\\".as_bytes();
    const __: &[u8] = "".as_bytes();
    const O0: &[u8] = r"\u0000".as_bytes();
    const O1: &[u8] = r"\u0001".as_bytes();
    const O2: &[u8] = r"\u0002".as_bytes();
    const O3: &[u8] = r"\u0003".as_bytes();
    const O4: &[u8] = r"\u0004".as_bytes();
    const O5: &[u8] = r"\u0005".as_bytes();
    const O6: &[u8] = r"\u0006".as_bytes();
    const O7: &[u8] = r"\u0007".as_bytes();
    const OB: &[u8] = r"\u000b".as_bytes();
    const OE: &[u8] = r"\u000e".as_bytes();
    const OF: &[u8] = r"\u000f".as_bytes();
    const I0: &[u8] = r"\u0010".as_bytes();
    const I1: &[u8] = r"\u0011".as_bytes();
    const I2: &[u8] = r"\u0012".as_bytes();
    const I3: &[u8] = r"\u0013".as_bytes();
    const I4: &[u8] = r"\u0014".as_bytes();
    const I5: &[u8] = r"\u0015".as_bytes();
    const I6: &[u8] = r"\u0016".as_bytes();
    const I7: &[u8] = r"\u0017".as_bytes();
    const I8: &[u8] = r"\u0018".as_bytes();
    const I9: &[u8] = r"\u0019".as_bytes();
    const IA: &[u8] = r"\u001a".as_bytes();
    const IB: &[u8] = r"\u001b".as_bytes();
    const IC: &[u8] = r"\u001c".as_bytes();
    const ID: &[u8] = r"\u001d".as_bytes();
    const IE: &[u8] = r"\u001e".as_bytes();
    const IF: &[u8] = r"\u001f".as_bytes();
    static ESCAPE: [&[u8]; 256] = [
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
            let bytes_substring = &bytes[start..i];
            // if str::from_utf8(bytes_substring).is_err() {
            //     return Err(CORRUPTED_DATA.to_py_error(unsafe { DESERIALIZATION_ERROR_TYPE }))
            // };
            output_buf.extend_from_slice(bytes_substring)?;
        }
        output_buf.extend_from_slice(escape)?;
        start = i + 1;
    }
    if start == bytes.len() {
        return Ok(());
    }

    let last_substring = &bytes[start..];
    // if str::from_utf8(last_substring).is_err() {
    //     return Err(CORRUPTED_DATA.to_py_error(unsafe { DESERIALIZATION_ERROR_TYPE }))
    // };
    output_buf.extend_from_slice(last_substring)
}

// TODO: change pointers to point at positions in str (like in structured dict list) instead of pointers to buf
pub fn convert_sized_pointer<'a, const SIZE: usize>(
    output_buf: &mut MemoryPyBytesBuffer,
    buf: &'a [u8],
    ptr: &mut usize,
    pointers: &mut PositionPointerHolder<'a>,
) -> Result<(), *mut PyObject> {
    if *ptr + SIZE > buf.len() {
        return Err(UNEXPECTED_END_OF_INPUT.to_py_error(unsafe { DESERIALIZATION_ERROR_TYPE }));
    }
    let pos = get_sized_pointer_pos::<SIZE>(buf, ptr);
    convert_from_pointer_position(output_buf, buf, pointers, pos)
}

fn convert_from_pointer_position<'a>(
    output_buf: &mut MemoryPyBytesBuffer,
    buf: &'a [u8],
    pointers: &mut PositionPointerHolder<'a>,
    position: usize,
) -> Result<(), *mut PyObject> {
    match pointers.unsafe_get(position)? {
        Position { pos, is_base_254 } => {
            let mut pos = *pos;
            if *is_base_254 {
                convert_string_without_inserting_pointer::<254>(output_buf, buf, &mut pos)
            } else {
                convert_string_without_inserting_pointer::<NUMBER_BASE>(output_buf, buf, &mut pos)
            }
        }
        Str(str) => {
            let mut len = 0;
            let data = unsafe { PyUnicode_AsUTF8AndSize(*str, &mut len) };
            write_string_escaped(output_buf, unsafe {
                std::slice::from_raw_parts(data as *const u8, len as usize)
            })
        }
    }
}

pub fn convert_custom_type(
    output_buf: &mut MemoryPyBytesBuffer,
    object: *mut PyObject,
    serialization_custom_types: &Option<PyHashMap<*mut PyObject>>,
    type_identifier: PyPointer,
) -> Result<(), *mut PyObject> {
    let custom_types = if let Some(custom_types) = serialization_custom_types {
        custom_types
    } else {
        &PyHashMap::default()
    };

    if let Some(converter) = custom_types.get(&PyKey(type_identifier.as_ptr())) {
        let args = temporary_tuple_of(object)?;
        let converted_object = unsafe { PyObject_CallObject(*converter, args.as_ptr()) };
        if converted_object.is_null() {
            raise_exception_with_cause(c"Failed to deserialize custom type".as_ptr(), unsafe {
                DESERIALIZATION_ERROR_TYPE
            });
            return Err(ptr::null_mut());
        }
        let converted_object = PyPointer::new(converted_object);
        if unsafe { PyBytes_Check(converted_object.as_ptr()) } == 0 {
            Err(
                "Custom type serialization must return a bytes representation of a valid json object"
                    .to_py_error(unsafe { DESERIALIZATION_ERROR_TYPE })
            )
        } else {
            let size = unsafe { PyBytes_Size(converted_object.as_ptr()) };
            output_buf.extend_from_bytes(size as usize, converted_object.as_ptr())
        }
    } else {
        unsafe {
            let str_representation =
                PyPointer::new_w_null_check(PyObject_Str(type_identifier.as_ptr()))?;
            let rust_str_representation =
                py_str_to_rust_str(&str_representation.as_ptr())?.to_string();
            Err(
                format!(
                    "Unknown custom type. please make sure the mapping exists in both the serializing and deserializing custom types. identifier: `{}` (type: `{}`)",
                    rust_str_representation,
                    pretty_type(type_identifier.as_ptr())
                ).to_py_error(DESERIALIZATION_ERROR_TYPE)
            )
        }
    }
}

fn convert_bytes_slice(
    output_buf: &mut MemoryPyBytesBuffer,
    bytes: &[u8],
    bytes_converter: *mut PyObject,
) -> Result<(), *mut PyObject> {
    let python_bytes = PyPointer::new_w_null_check(unsafe {
        PyBytes_FromStringAndSize(bytes.as_ptr() as *const c_char, bytes.len() as Py_ssize_t)
    })?;

    let args = temporary_tuple_of(python_bytes.as_ptr())?;
    let converted_bytes = unsafe { PyObject_CallObject(bytes_converter, args.as_ptr()) };
    if converted_bytes.is_null() {
        raise_exception_with_cause(c"Failed to convert bytes".as_ptr(), unsafe {
            DESERIALIZATION_ERROR_TYPE
        });
        return Err(ptr::null_mut());
    }
    let converted_bytes = PyPointer::new(converted_bytes);
    if unsafe { PyBytes_Check(converted_bytes.as_ptr()) } == 0 {
        Err(
            "Bytes converter must return a bytes representation of a valid json object"
                .to_py_error(unsafe { DESERIALIZATION_ERROR_TYPE }),
        )
    } else {
        let size = unsafe { PyBytes_Size(converted_bytes.as_ptr()) };
        output_buf.extend_from_bytes(size as usize, converted_bytes.as_ptr())
    }
}
