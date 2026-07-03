use crate::deserializing::compound_types::deserialize_dict_key;
use crate::deserializing::custom_types::deserialize_custom_type;
use crate::deserializing::deserialize::deserialize_object;
use crate::deserializing::pointer_holders::position_pointer_holder::PositionPointerHolder;
use crate::deserializing::primitives::{decode_bytes, decode_f64, decode_f64_rust, decode_false, decode_null, decode_string, decode_true};
use crate::deserializing::utils::{decode_number_py_ssize_t, decode_number_usize, skip_number, DESERIALIZATION_ERROR_TYPE};
use crate::utils::consts::{
    AMOUNT_OF_USED_FLAGS, ASCII_STR_FLAG, BOOL_FLAG, BYTES_FLAG, CONSISTENT_TYPE_LIST_FLAG,
    CUSTOM_TYPE_FLAG, DICT_FLAG, EMPTY_BYTES_FLAG, EMPTY_DICT_FLAG, EMPTY_LIST_FLAG,
    EMPTY_STR_FLAG, FALSE_FLAG, FLOAT_FLAG, LIST_FLAG, LIST_OF_STRUCTURED_DICTS_FLAG,
    NEGATIVE_INT_FLAG, NULL_FLAG, POINTER_FLAG, POINTER_FLAG_1BYTE, POINTER_FLAG_2BYTE,
    POINTER_FLAG_3BYTE, POINTER_FLAG_4BYTE, POSITIVE_INT_FLAG, STR_FLAG, STR_KEY_DICT_FLAG,
    TRUE_FLAG,
};
use crate::utils::consts::{LEFTMOST_BIT_MASK, MIGHT_BE_ASCII, NUMBER_BASE};
use crate::utils::py_dict_key::PyHashMap;
use crate::utils::py_helpers::{compare_objects, pretty_type, py_str_to_rust_str, rust_bool_to_py_bool, to_py_str, ToPyErr};
use crate::utils::safe_py_pointer::PyPointer;
use crate::{safe_get, safe_new_py_dict};
use pyo3_ffi::{
    PyDict_SetItem, PyExc_TypeError, PyObject, PyObject_IsTrue, Py_False, Py_True,
};

pub enum PathPart {
    Index(usize),
    Key(*mut PyObject),
}

macro_rules! index_out_of_range_template {
    () => {
        "Index out of range, index is `{}` but list is of len `{}`"
    };
}
macro_rules! key_not_in_dict_template {
    () => {
        "Key not found, key: `{}` (type `{}`)"
    };
}

pub fn lazy_deserialize(
    buf: &[u8],
    ptr: &mut usize,
    pointers: &mut PositionPointerHolder,
    use_tuples: bool,
    custom_types: &Option<PyHashMap<*mut PyObject>>,
    dont_load: bool,
    include_falsy: bool,
    mut path_to_load: &[PathPart],
) -> Result<*mut PyObject, *mut PyObject> {
    if path_to_load.is_empty() {
        if dont_load {
            if !include_falsy && is_falsy(buf, ptr, pointers, custom_types, use_tuples)? {
                return Ok(decode_false())
            }
            return Ok(decode_true());
        }
        return deserialize_object(buf, ptr, pointers, use_tuples, custom_types);
    }
    let indexer = &path_to_load[0];
    path_to_load = &path_to_load[1..];
    let flag = *safe_get!(buf, *ptr);
    *ptr += 1;
    match indexer {
        PathPart::Index(index) => match flag {
            EMPTY_LIST_FLAG => Err(format!(index_out_of_range_template!(), index, 0)
                .to_py_error(unsafe { DESERIALIZATION_ERROR_TYPE })),
            LIST_FLAG => {
                let len = decode_number_usize::<NUMBER_BASE>(buf, ptr)?;
                if *index >= len {
                    return Err(format!(index_out_of_range_template!(), index, len)
                        .to_py_error(unsafe { DESERIALIZATION_ERROR_TYPE }));
                }
                for _ in 0..*index {
                    skip_object(buf, ptr, pointers)?;
                }
                lazy_deserialize(
                    buf,
                    ptr,
                    pointers,
                    use_tuples,
                    custom_types,
                    dont_load,
                    include_falsy,
                    path_to_load,
                )
            }
            CONSISTENT_TYPE_LIST_FLAG => {
                lazy_deserialize_consistent_type_list(buf, ptr, *index, path_to_load, pointers, dont_load, include_falsy)
            }
            LIST_OF_STRUCTURED_DICTS_FLAG => {
                let list_length = decode_number_usize::<NUMBER_BASE>(buf, ptr)?;
                let dict_length = decode_number_usize::<NUMBER_BASE>(buf, ptr)?;
                if *index >= list_length {
                    return Err(format!(index_out_of_range_template!(), index, list_length)
                        .to_py_error(unsafe { DESERIALIZATION_ERROR_TYPE }));
                }
                if !path_to_load.is_empty() {
                    let next_indexer = &path_to_load[0];
                    path_to_load = &path_to_load[1..];
                    match next_indexer {
                        PathPart::Index(_) => Err("Invalid path, expected `list` but found `dict`"
                            .to_py_error(unsafe { DESERIALIZATION_ERROR_TYPE })),
                        PathPart::Key(next_indexer) => {
                            let mut checking_keys = true;
                            let mut key_index = None;
                            for i in 0..dict_length {
                                if !checking_keys {
                                    skip_object(buf, ptr, pointers)?; // skip key
                                    skip_object(buf, ptr, pointers)?; // skip value
                                    continue;
                                }
                                let key = PyPointer::new(deserialize_object(
                                    buf,
                                    ptr,
                                    pointers,
                                    use_tuples,
                                    custom_types,
                                )?);
                                if compare_objects(key.as_ptr(), *next_indexer) {
                                    if *index == 0 {
                                        return lazy_deserialize(
                                            buf,
                                            ptr,
                                            pointers,
                                            use_tuples,
                                            custom_types,
                                            dont_load,
                                            include_falsy,
                                            path_to_load,
                                        );
                                    }
                                    key_index = Some(i);
                                    checking_keys = false; // no more need to deserialize the keys for comparing them
                                }
                                skip_object(buf, ptr, pointers)?;
                            }

                            match key_index {
                                Some(key_index) => {
                                    for _ in 0..((*index - 1) * dict_length + key_index) {
                                        skip_object(buf, ptr, pointers)?;
                                    }
                                    lazy_deserialize(
                                        buf,
                                        ptr,
                                        pointers,
                                        use_tuples,
                                        custom_types,
                                        dont_load,
                                        include_falsy,
                                        path_to_load,
                                    )
                                }
                                None => Err(format!(
                                    key_not_in_dict_template!(),
                                    py_str_to_rust_str(&to_py_str(*next_indexer)?.as_ptr())?,
                                    pretty_type(*next_indexer)
                                )
                                .to_py_error(unsafe { DESERIALIZATION_ERROR_TYPE })),
                            }
                        }
                    }
                } else {
                    if dont_load {
                        return Ok(decode_true());
                    }
                    if *index == 0 {
                        let dict = safe_new_py_dict!();
                        for _ in 0..dict_length {
                            let key = PyPointer::new(deserialize_object(
                                buf,
                                ptr,
                                pointers,
                                use_tuples,
                                custom_types,
                            )?);
                            let value = PyPointer::new(deserialize_object(
                                buf,
                                ptr,
                                pointers,
                                use_tuples,
                                custom_types,
                            )?);
                            unsafe {
                                PyDict_SetItem(dict, key.as_ptr(), value.as_ptr());
                            }
                        }
                        return Ok(dict);
                    }
                    let mut keys = Vec::with_capacity(dict_length);
                    for _ in 0..dict_length {
                        let key = PyPointer::new(deserialize_object(
                            buf,
                            ptr,
                            pointers,
                            use_tuples,
                            custom_types,
                        )?);
                        skip_object(buf, ptr, pointers)?;
                        keys.push(key);
                    }
                    for _ in 0..(*index - 1) * dict_length {
                        skip_object(buf, ptr, pointers)?;
                    }
                    let dict = safe_new_py_dict!();
                    for key in keys {
                        let value = PyPointer::new(deserialize_object(
                            buf,
                            ptr,
                            pointers,
                            use_tuples,
                            custom_types,
                        )?);
                        unsafe {
                            PyDict_SetItem(dict, key.as_ptr(), value.as_ptr());
                        }
                    }
                    Ok(dict)
                }
            }
            _ => Err(format!(
                "Invalid path, expected `list` but found `{}`",
                flag_to_type_name(flag)?
            )
            .to_py_error(unsafe { DESERIALIZATION_ERROR_TYPE })),
        },
        PathPart::Key(key) => match flag {
            EMPTY_DICT_FLAG => Err(format!(
                key_not_in_dict_template!(),
                py_str_to_rust_str(&to_py_str(*key)?.as_ptr())?,
                pretty_type(*key)
            )
            .to_py_error(unsafe { DESERIALIZATION_ERROR_TYPE })),
            DICT_FLAG => {
                let len = decode_number_usize::<NUMBER_BASE>(buf, ptr)?;
                for _ in 0..len {
                    let dict_key = PyPointer::new(deserialize_object(
                        buf,
                        ptr,
                        pointers,
                        use_tuples,
                        custom_types,
                    )?);
                    if compare_objects(*key, dict_key.as_ptr()) {
                        return lazy_deserialize(
                            buf,
                            ptr,
                            pointers,
                            use_tuples,
                            custom_types,
                            dont_load,
                            include_falsy,
                            path_to_load,
                        );
                    }
                    skip_object(buf, ptr, pointers)?;
                }

                Err(format!(
                    key_not_in_dict_template!(),
                    py_str_to_rust_str(&to_py_str(*key)?.as_ptr())?,
                    pretty_type(*key)
                )
                .to_py_error(unsafe { DESERIALIZATION_ERROR_TYPE }))
            }
            STR_KEY_DICT_FLAG => {
                let len = decode_number_usize::<NUMBER_BASE>(buf, ptr)?;
                for _ in 0..len {
                    let dict_key = PyPointer::new(deserialize_dict_key(buf, ptr, pointers)?);
                    if compare_objects(dict_key.as_ptr(), *key) {
                        return lazy_deserialize(
                            buf,
                            ptr,
                            pointers,
                            use_tuples,
                            custom_types,
                            dont_load,
                            include_falsy,
                            path_to_load,
                        );
                    }
                    skip_object(buf, ptr, pointers)?;
                }

                Err(format!(
                    key_not_in_dict_template!(),
                    py_str_to_rust_str(&to_py_str(*key)?.as_ptr())?,
                    pretty_type(*key)
                )
                .to_py_error(unsafe { DESERIALIZATION_ERROR_TYPE }))
            }
            _ => Err(format!(
                "Invalid path, expected `dict` but found `{}`",
                flag_to_type_name(flag)?
            )
            .to_py_error(unsafe { DESERIALIZATION_ERROR_TYPE })),
        },
    }
}

fn is_falsy(
    buf: &[u8],
    ptr: &mut usize,
    pointers: &mut PositionPointerHolder,
    custom_types: &Option<PyHashMap<*mut PyObject>>,
    use_tuples: bool,
) -> Result<bool, *mut PyObject> {
    let flag = *safe_get!(buf, *ptr);
    *ptr += 1;
    match flag {
        EMPTY_STR_FLAG | EMPTY_LIST_FLAG | FALSE_FLAG | NULL_FLAG | EMPTY_BYTES_FLAG
        | EMPTY_DICT_FLAG => Ok(true),
        AMOUNT_OF_USED_FLAGS => Ok(true), // zero
        FLOAT_FLAG => Ok(matches!(decode_f64_rust(buf, ptr), Ok(0.0))),
        CUSTOM_TYPE_FLAG => {
            let deserialized_obj = deserialize_custom_type(
                buf,
                ptr,
                pointers,
                use_tuples,
                custom_types,
            )?;
            Ok(unsafe { PyObject_IsTrue(deserialized_obj.as_ptr()) } == 0)
        },
        NEGATIVE_INT_FLAG
        | POSITIVE_INT_FLAG
        | STR_FLAG
        | ASCII_STR_FLAG
        | TRUE_FLAG
        | POINTER_FLAG
        | POINTER_FLAG_1BYTE
        | POINTER_FLAG_2BYTE
        | POINTER_FLAG_3BYTE
        | POINTER_FLAG_4BYTE
        | BYTES_FLAG
        | CONSISTENT_TYPE_LIST_FLAG
        | DICT_FLAG
        | LIST_FLAG
        | LIST_OF_STRUCTURED_DICTS_FLAG
        | STR_KEY_DICT_FLAG => Ok(false),
        _ => Ok(false),
    }
}

fn skip_object(
    buf: &[u8],
    ptr: &mut usize,
    pointers: &mut PositionPointerHolder,
) -> Result<(), *mut PyObject> {
    let flag = *safe_get!(buf, *ptr);
    *ptr += 1;
    match flag {
        POSITIVE_INT_FLAG | NEGATIVE_INT_FLAG | POINTER_FLAG => skip_number(buf, ptr),
        FLOAT_FLAG => {
            *ptr += 8;
            Ok(())
        }
        STR_FLAG | ASCII_STR_FLAG => skip_string(buf, ptr, pointers, NUMBER_BASE as u8),
        TRUE_FLAG | FALSE_FLAG | NULL_FLAG | EMPTY_BYTES_FLAG | EMPTY_DICT_FLAG
        | EMPTY_STR_FLAG | EMPTY_LIST_FLAG => Ok(()),
        POINTER_FLAG_1BYTE => {
            *ptr += 1;
            Ok(())
        }
        POINTER_FLAG_2BYTE => {
            *ptr += 2;
            Ok(())
        }
        POINTER_FLAG_3BYTE => {
            *ptr += 3;
            Ok(())
        }
        POINTER_FLAG_4BYTE => {
            *ptr += 4;
            Ok(())
        }
        BYTES_FLAG => {
            *ptr += decode_number_usize::<NUMBER_BASE>(buf, ptr)?;
            Ok(())
        }
        CONSISTENT_TYPE_LIST_FLAG => skip_consistent_type_list(buf, ptr, pointers),
        DICT_FLAG => {
            let len = decode_number_usize::<NUMBER_BASE>(buf, ptr)?;
            for _ in 0..len {
                if *safe_get!(buf, *ptr) == STR_FLAG {
                    // fast path
                    *ptr += 1;
                    skip_string(buf, ptr, pointers, NUMBER_BASE as u8)?;
                } else {
                    skip_object(buf, ptr, pointers)?;
                }
                skip_object(buf, ptr, pointers)?;
            }
            Ok(())
        }
        STR_KEY_DICT_FLAG => {
            let len = decode_number_usize::<NUMBER_BASE>(buf, ptr)?;
            for _ in 0..len {
                if *safe_get!(buf, *ptr) == (NUMBER_BASE - 1) as u8 {
                    *ptr += 1;
                    skip_number(buf, ptr)?;
                } else {
                    skip_string(buf, ptr, pointers, NUMBER_BASE as u8)?;
                }
                skip_object(buf, ptr, pointers)?;
            }
            Ok(())
        }
        LIST_FLAG => {
            let len = decode_number_usize::<NUMBER_BASE>(buf, ptr)?;
            for _ in 0..len {
                skip_object(buf, ptr, pointers)?;
            }
            Ok(())
        }
        LIST_OF_STRUCTURED_DICTS_FLAG => {
            let list_len = decode_number_usize::<NUMBER_BASE>(buf, ptr)?;
            let dict_len = decode_number_usize::<NUMBER_BASE>(buf, ptr)?;
            // first dict:
            for _ in 0..dict_len {
                skip_object(buf, ptr, pointers)?;
                skip_object(buf, ptr, pointers)?;
            }
            // rest of the dicts:
            for _ in 1..list_len {
                for _ in 0..dict_len {
                    skip_object(buf, ptr, pointers)?;
                }
            }
            Ok(())
        }
        CUSTOM_TYPE_FLAG => {
            skip_object(buf, ptr, pointers)?;
            skip_object(buf, ptr, pointers)
        }
        _ if flag < AMOUNT_OF_USED_FLAGS => {
            Err("Unexpected flag".to_py_error(unsafe { PyExc_TypeError }))
        }
        _ => Ok(()),
    }
}

fn skip_consistent_type_list(
    buf: &[u8],
    ptr: &mut usize,
    pointers: &mut PositionPointerHolder,
) -> Result<(), *mut PyObject> {
    let typ = *safe_get!(buf, *ptr);
    *ptr += 1;
    let len = decode_number_usize::<NUMBER_BASE>(buf, ptr)?;

    match typ {
        NULL_FLAG => {}
        BOOL_FLAG => {
            *ptr += (len + 7) >> 3;
        }
        FLOAT_FLAG => {
            *ptr += 8 * len;
        }
        BYTES_FLAG => {
            for _ in 0..len {
                let bytes_len = decode_number_usize::<NUMBER_BASE>(buf, ptr)?;
                *ptr += bytes_len;
            }
        }
        STR_FLAG => {
            for _ in 0..len {
                skip_string(buf, ptr, pointers, NUMBER_BASE as u8)?;
            }
        }
        _ => {
            return Err(
                format!("Unexpected type flag: {typ}").to_py_error(unsafe { PyExc_TypeError })
            );
        }
    }
    Ok(())
}

fn skip_string<'a>(
    buf: &[u8],
    ptr: &mut usize,
    pointers: &mut PositionPointerHolder,
    base: u8,
) -> Result<(), *mut PyObject> {
    pointers.insert_position(*ptr, base != NUMBER_BASE as u8);
    let length = decode_number_usize::<NUMBER_BASE>(buf, ptr)?;
    *ptr += length;
    Ok(())
}

fn lazy_deserialize_consistent_type_list(
    buf: &[u8],
    ptr: &mut usize,
    index: usize,
    path_to_load: &[PathPart],
    pointers: &mut PositionPointerHolder,
    dont_load: bool,
    include_falsy: bool,
) -> Result<*mut PyObject, *mut PyObject> {
    let typ_flag = *safe_get!(buf, *ptr);
    *ptr += 1;
    let len = decode_number_usize::<NUMBER_BASE>(buf, ptr)?;
    if index >= len {
        return Err(format!(index_out_of_range_template!(), index, len)
            .to_py_error(unsafe { DESERIALIZATION_ERROR_TYPE }));
    }
    if !path_to_load.is_empty() {
        let got_type = flag_to_type_name(typ_flag)?;
        return Err(format!(
            "Invalid path, expected `{}` but found `{got_type}`",
            if let PathPart::Index(_) = path_to_load[0] {
                "list"
            } else {
                "dict"
            }
        )
        .to_py_error(unsafe { DESERIALIZATION_ERROR_TYPE }));
    }
    if dont_load {
        if !include_falsy {
            return match typ_flag {
                NULL_FLAG => Ok(decode_false()),
                BOOL_FLAG => lazy_load_bool_list(buf, ptr, index, len),
                BYTES_FLAG => lazy_load_bytes_list::<false>(buf, ptr, index),
                STR_FLAG => lazy_load_str_list::<false>(buf, ptr, index, pointers),
                FLOAT_FLAG => lazy_load_float_list::<false>(buf, ptr, index),
                _ => Err(format!("Unexpected type flag: {typ_flag}")
                    .to_py_error(unsafe { DESERIALIZATION_ERROR_TYPE })),
            }
        }
        return Ok(decode_true())
    }
    match typ_flag {
        NULL_FLAG => Ok(decode_null()),
        BOOL_FLAG => lazy_load_bool_list(buf, ptr, index, len),
        BYTES_FLAG => lazy_load_bytes_list::<true>(buf, ptr, index),
        STR_FLAG => lazy_load_str_list::<true>(buf, ptr, index, pointers),
        FLOAT_FLAG => lazy_load_float_list::<true>(buf, ptr, index),
        _ => Err(format!("Unexpected type flag: {typ_flag}")
            .to_py_error(unsafe { DESERIALIZATION_ERROR_TYPE })),
    }
}

fn lazy_load_float_list<const INCLUDE_FALSY: bool>(
    buf: &[u8],
    ptr: &mut usize,
    index: usize,
) -> Result<*mut PyObject, *mut PyObject> {
    *ptr += 8 * index;
    if INCLUDE_FALSY {
        decode_f64(buf, ptr)
    } else {
        Ok(if let Ok(0.0) = decode_f64_rust(buf, ptr) {
            decode_false()
        } else {
            decode_true()
        })
    }
}

fn lazy_load_bytes_list<const INCLUDE_FALSY: bool>(
    buf: &[u8],
    ptr: &mut usize,
    index: usize,
) -> Result<*mut PyObject, *mut PyObject> {
    for _ in 0..index {
        let bytes_len = decode_number_usize::<NUMBER_BASE>(buf, ptr)?;
        *ptr += bytes_len;
    }
    if !INCLUDE_FALSY {
        let len = decode_number_py_ssize_t::<NUMBER_BASE>(buf, ptr)?;
        return Ok(rust_bool_to_py_bool(len != 0))
    }
    decode_bytes(buf, ptr)
}

fn lazy_load_str_list<const INCLUDE_FALSY: bool>(
    buf: &[u8],
    ptr: &mut usize,
    index: usize,
    pointers: &mut PositionPointerHolder,
) -> Result<*mut PyObject, *mut PyObject> {
    for _ in 0..index {
        skip_string(buf, ptr, pointers, NUMBER_BASE as u8)?;
    }
    if !INCLUDE_FALSY {
        let len = decode_number_py_ssize_t::<NUMBER_BASE>(buf, ptr)?;
        return Ok(rust_bool_to_py_bool(len != 0))
    }
    decode_string::<MIGHT_BE_ASCII, NUMBER_BASE, PositionPointerHolder>(buf, ptr, pointers)
}

fn lazy_load_bool_list(
    buf: &[u8],
    ptr: &mut usize,
    index: usize,
    length: usize,
) -> Result<*mut PyObject, *mut PyObject> {
    /*
    same as: math.ceil(length / NUMBER_OF_BITS_IN_BYTE)
    the `>> 3` is like dividing by 8 (8 is `1000` in binary)
    the + 7 is like rounding up
     */
    let amount_of_bytes = (length + 7) >> 3;
    let table = unsafe { [Py_True(), Py_False()] };
    for i in 0..amount_of_bytes {
        let mut byte = *safe_get!(buf, *ptr + i);
        for j in 0..8 {
            if i * 8 + j == index {
                return Ok(table[((byte & LEFTMOST_BIT_MASK) == 0) as usize]);
            }
            byte <<= 1;
        }
    }
    Err("This should be unreachable".to_py_error(unsafe { DESERIALIZATION_ERROR_TYPE }))
}

fn flag_to_type_name(flag: u8) -> Result<&'static str, *mut PyObject> {
    if flag >= AMOUNT_OF_USED_FLAGS {
        return Ok("int");
    }

    match flag {
        NULL_FLAG => Ok("None"),
        BOOL_FLAG => Ok("bool"),
        BYTES_FLAG => Ok("bytes"),
        STR_FLAG | EMPTY_STR_FLAG => Ok("str"),
        FLOAT_FLAG => Ok("float"),
        EMPTY_BYTES_FLAG => Ok("bytes"),
        TRUE_FLAG | FALSE_FLAG => Ok("bool"),
        EMPTY_LIST_FLAG | LIST_FLAG | CONSISTENT_TYPE_LIST_FLAG | LIST_OF_STRUCTURED_DICTS_FLAG => {
            Ok("list")
        }
        EMPTY_DICT_FLAG | DICT_FLAG | STR_KEY_DICT_FLAG => Ok("dict"),
        POSITIVE_INT_FLAG | NEGATIVE_INT_FLAG => Ok("int"),
        POINTER_FLAG => Ok("str"),
        ASCII_STR_FLAG => Ok("str"),
        _ => {
            Err("Corrupt data: unexpected flag".to_py_error(unsafe { DESERIALIZATION_ERROR_TYPE }))
        }
    }
}
