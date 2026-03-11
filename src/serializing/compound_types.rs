use crate::serializing::primitives::{serialize_str, try_encode_as_pointer};
use crate::serializing::py_bytes_buffer::PyBytesBuffer;
use crate::serializing::serialize;
use crate::serializing::serializing_string_cache::{Pointers, PyStringKey};
use crate::serializing::utils::{all_dict_keys_are_str, encode_number, SERIALIZATION_ERROR_TYPE};
use crate::utils::consts::{BOOL_FLAG, CONSISTENT_TYPE_LIST_FLAG, DICT_FLAG, EMPTY_DICT_FLAG, EMPTY_LIST_FLAG, INVALID_UTF_8_START_BYTE_COMPACT_ASCII, LIST_FLAG, LIST_OF_STRUCTURED_DICTS_FLAG, NULL_FLAG, NUMBER_BASE, STR_KEY_DICT_FLAG};
use crate::utils::py_helpers::ToPyErr;
use crate::utils::wrappers::{get_list_size, get_tuple_size, is_ascii, list_get_item, py_unicode_data, tuple_get_item};
use pyo3_ffi::{PyBool_Type, PyDict_Next, PyDict_Size, PyDict_Type, PyList_Type, PyObject, PyTuple_Type, PyTypeObject, PyUnicode_AsUTF8AndSize, PyUnicode_GET_LENGTH, PyUnicode_Type, Py_None, Py_True, Py_ssize_t};
use rustc_hash::FxHashMap;
use std::{ptr, slice};

#[inline(always)]
pub fn serialize_dict(obj: *mut PyObject, buffer: &mut PyBytesBuffer, pointers: &mut Pointers, str_count: &mut usize) -> Result<(), *mut PyObject>{
    let size = unsafe { PyDict_Size(obj) };
    if size == 0 {
        return buffer.push(EMPTY_DICT_FLAG);
    }
    if all_dict_keys_are_str(obj) {
        buffer.push(STR_KEY_DICT_FLAG)?;
        encode_number::<NUMBER_BASE>(buffer, size as u128)?;

        let mut pos = 0;
        let mut key: *mut PyObject = ptr::null_mut();
        let mut val: *mut PyObject = ptr::null_mut();
        while unsafe { PyDict_Next(obj, &mut pos, &mut key, &mut val) } != 0 {
            // key
            encode_dict_key(buffer, pointers, str_count, key)?;
            // value
            serialize::serialize(val, buffer, pointers, str_count)?;
        }
        return Ok(());
    }

    buffer.push(DICT_FLAG)?;
    encode_number::<NUMBER_BASE>(buffer, size as u128)?;

    let mut pos = 0;
    let mut key: *mut PyObject = ptr::null_mut();
    let mut val: *mut PyObject = ptr::null_mut();
    while unsafe { PyDict_Next(obj, &mut pos, &mut key, &mut val) } != 0 {
        unsafe {
            if (*key).ob_type == &mut PyTuple_Type {
                return Err("Invalid type for dict key: tuple".to_py_error(SERIALIZATION_ERROR_TYPE));
            }
        }
        serialize::serialize(key, buffer, pointers, str_count)?;
        serialize::serialize(val, buffer, pointers, str_count)?;
    }
    return Ok(());
}

#[inline(always)]
fn encode_dict_key(buffer: &mut PyBytesBuffer, pointers: &mut Pointers, str_count: &mut usize, key: *mut PyObject) -> Result<(), *mut PyObject>{
    let mut len = 0;
    let is_compact_ascii = is_ascii(key);
    let data = if is_compact_ascii {
        len = unsafe { PyUnicode_GET_LENGTH(key) };
        py_unicode_data(key)
    } else {
        unsafe {
            PyUnicode_AsUTF8AndSize(key, &mut len) as *const u8
        }
    };
    let encoded_as_pointer = try_encode_as_pointer(key, buffer, pointers, *str_count, len, &[NUMBER_BASE as u8 - 1])?;
    if !encoded_as_pointer {
        *str_count += 1;
        if is_compact_ascii {
            encode_number::<{ NUMBER_BASE - 1 }>(buffer, 1 + len as u128)?;
            buffer.push(INVALID_UTF_8_START_BYTE_COMPACT_ASCII)?;
        } else {
            encode_number::<{ NUMBER_BASE - 1 }>(buffer, len as u128)?;
        }
        unsafe {
            buffer.extend_from_slice(slice::from_raw_parts(
                data,
                len as usize,
            ))
        }
    } else {
        Ok(())
    }
}

fn is_consistent_type_list(obj: *mut PyObject, is_list: bool, len: Py_ssize_t) -> bool {
    let first_item = if is_list { list_get_item(obj, 0) } else { tuple_get_item(obj, 0) };
    let first_type = unsafe {
        (*first_item).ob_type
    };
    (1..len).all(|i| {
        let item = if is_list {
            list_get_item(obj, i)
        } else {
            tuple_get_item(obj, i)
        };
        unsafe { (*item).ob_type == first_type }
    })
}

pub fn encode_list(obj: *mut PyObject, buffer: &mut PyBytesBuffer, pointers: &mut Pointers, str_count: &mut usize, typ: *mut PyTypeObject) -> Result<(), *mut PyObject> {
    let is_list = unsafe { typ == &mut PyList_Type };
    let len = if is_list {
        get_list_size(obj)
    } else {
        get_tuple_size(obj)
    };
    if len == 0 {
        return buffer.push(EMPTY_LIST_FLAG);
    }

    if len > 1 && is_consistent_type_list(obj, is_list, len) {
        let first_item = if is_list { list_get_item(obj, 0) } else { tuple_get_item(obj, 0) };
        if first_item == unsafe { Py_None() } {
            buffer.extend_from_slice(&[CONSISTENT_TYPE_LIST_FLAG, NULL_FLAG])?;
            return encode_number::<NUMBER_BASE>(buffer, len as u128);
        }
        let first_type = unsafe { (*first_item).ob_type };
        if unsafe { first_type == &mut PyBool_Type } {
            return encode_bool_list(obj, buffer, is_list, len);
        } else if unsafe { first_type == &mut PyDict_Type } {
            if encode_structured_list(obj, buffer, pointers, str_count, is_list, len, first_item)? {
                return Ok(())
            }
        }
    }

    serialize_normal_list(obj, buffer, pointers, is_list, len, str_count)
}

fn get_dict_keys(dict: *mut PyObject) -> Option<Pointers> {
    let mut pos: Py_ssize_t = 0;
    let mut key: *mut PyObject = ptr::null_mut();
    let mut value: *mut PyObject = ptr::null_mut();
    let mut keys = FxHashMap::default();

    while unsafe { PyDict_Next(dict, &mut pos, &mut key, &mut value) } != 0 {
        if unsafe { (*key).ob_type != &mut PyUnicode_Type } {
            return None;
        }
        keys.insert(PyStringKey(key), pos as usize - 1);
    }
    Some(keys)
}
fn compare_dict_keys(dict: *mut PyObject, expected_keys: &Pointers) -> bool {
    let mut pos: Py_ssize_t = 0;
    let mut key: *mut PyObject = ptr::null_mut();
    let mut value: *mut PyObject = ptr::null_mut();
    let keys_count = unsafe { PyDict_Size(dict) };
    if keys_count as usize != expected_keys.len() { return false; }
    while unsafe { PyDict_Next(dict, &mut pos, &mut key, &mut value) } != 0 {
        if !expected_keys.contains_key(&PyStringKey(key)) {
            return false
        }
    }
    true
}

fn encode_structured_list(obj: *mut PyObject, buffer: &mut PyBytesBuffer, pointers: &mut Pointers, str_count: &mut usize, is_list: bool, len: isize, first_item: *mut PyObject) -> Result<bool, *mut PyObject> {
    let first_dict_keys = match get_dict_keys(first_item) {
        Some(keys) => keys,
        None => return Ok(false),
    };
    let len_of_dicts = first_dict_keys.len();
    let all_same_structure = (1..len).all(|i| {
        let item = if is_list {
            list_get_item(obj, i)
        } else {
            tuple_get_item(obj, i)
        };
        compare_dict_keys(item, &first_dict_keys)
    });

    if !all_same_structure {
        return Ok(false);
    }
    buffer.push(LIST_OF_STRUCTURED_DICTS_FLAG)?;
    encode_number::<NUMBER_BASE>(buffer, len as u128)?;
    encode_number::<NUMBER_BASE>(buffer, first_dict_keys.len() as u128)?;

    // first dict - normal (for structure)
    let mut pos = 0;
    let mut key: *mut PyObject = ptr::null_mut();
    let mut val: *mut PyObject = ptr::null_mut();
    while unsafe { PyDict_Next(first_item, &mut pos, &mut key, &mut val) } != 0 {
        serialize_str(key, buffer, pointers, str_count)?;
        serialize::serialize(val, buffer, pointers, str_count)?;
    }

    let mut values = Vec::with_capacity(len_of_dicts);
    unsafe {
        values.set_len(len_of_dicts);
    }
    for i in 1..len {
        let inner_dict = if is_list {
            list_get_item(obj, i)
        } else {
            tuple_get_item(obj, i)
        };

        let mut pos = 0;
        let mut key: *mut PyObject = ptr::null_mut();
        let mut val: *mut PyObject = ptr::null_mut();
        while unsafe { PyDict_Next(inner_dict, &mut pos, &mut key, &mut val) } != 0 {
            values[first_dict_keys[&PyStringKey(key)]] = val;
        }

        for val in &values {
            serialize::serialize(*val, buffer, pointers, str_count)?;
        }
    }
    return Ok(true);
}

#[inline(always)]
fn encode_bool_list(obj: *mut PyObject, buffer: &mut PyBytesBuffer, is_list: bool, len: isize) -> Result<(), *mut PyObject> {
    buffer.extend_from_slice(&[CONSISTENT_TYPE_LIST_FLAG, BOOL_FLAG])?;
    encode_number::<NUMBER_BASE>(buffer, len as u128)?;

    let mut byte: u8 = 0;
    let mut number_of_bits: u8 = 0;

    for i in 0..len {
        let item = if is_list {
            list_get_item(obj, i)
        } else {
            tuple_get_item(obj, i)
        };
        byte = (byte << 1) | ((item == unsafe { Py_True() }) as u8);
        number_of_bits += 1;

        if number_of_bits == 8 {
            buffer.push(byte)?;
            byte = 0;
            number_of_bits = 0;
        }
    }

    if number_of_bits != 0 {
        buffer.push(byte << (8 - number_of_bits))
    } else {
        Ok(())
    }
}

#[inline(always)]
fn serialize_normal_list(
    obj: *mut PyObject, buf: &mut PyBytesBuffer, pointers: &mut Pointers, is_list: bool, len: Py_ssize_t, str_count: &mut usize
) -> Result<(), *mut PyObject>{
    buf.push(LIST_FLAG)?;
    encode_number::<NUMBER_BASE>(buf, len as u128)?;
    for i in 0..len {
        let item = if is_list {
            list_get_item(obj, i)
        } else {
            tuple_get_item(obj, i)
        };
        serialize::serialize(item, buf, pointers, str_count)?;
    }
    Ok(())
}