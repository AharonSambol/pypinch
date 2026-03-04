use std::ptr;

use pyo3_ffi::{Py_None, Py_ssize_t, Py_True, PyBool_Type, PyDict_Next, PyDict_Size, PyList_Type, PyLong_Type, PyObject, PyTypeObject, PyUnicode_AsUTF8AndSize, PyUnicode_Type};

use crate::serializing::primitives::try_encode_as_pointer;
use crate::serializing::serialize;
use crate::serializing::serialize::Pointers;
use crate::serializing::utils::{all_dict_keys_are_str, encode_number};
use crate::utils::consts::{BOOL_FLAG, CONSISTENT_TYPE_LIST_FLAG, DICT_FLAG, EMPTY_DICT_FLAG, EMPTY_LIST_FLAG, LIST_FLAG, NOT_A_STR_BUT_A_POINTER_FLAG, NULL_FLAG, NUMBER_BASE, STR_KEY_DICT_FLAG};
use crate::utils::wrappers::{get_list_size, get_tuple_size, list_get_item, tuple_get_item};

#[inline(always)]
pub unsafe fn serialize_dict(obj: *mut PyObject, buffer: &mut Vec<u8>, pointers: &mut Pointers, str_count: &mut usize) -> Result<(), *mut PyObject>{
    let size = PyDict_Size(obj);
    if size == 0 {
        buffer.push(EMPTY_DICT_FLAG);
        return Ok(());
    }
    if all_dict_keys_are_str(obj) {
        // TODO: !!!!!!!!!!
        buffer.push(STR_KEY_DICT_FLAG);
        encode_number::<NUMBER_BASE>(buffer, size as u128);

        let mut pos = 0;
        let mut key: *mut PyObject = ptr::null_mut();
        let mut val: *mut PyObject = ptr::null_mut();
        while PyDict_Next(obj, &mut pos, &mut key, &mut val) != 0 {
            // key
            encode_dict_key(buffer, pointers, str_count, key);
            // value
            serialize::serialize(val, buffer, pointers, str_count)?;
        }
        return Ok(());
    }

    buffer.push(DICT_FLAG);
    encode_number::<NUMBER_BASE>(buffer, size as u128);

    let mut pos = 0;
    let mut key: *mut PyObject = ptr::null_mut();
    let mut val: *mut PyObject = ptr::null_mut();
    while PyDict_Next(obj, &mut pos, &mut key, &mut val) != 0 {
        serialize::serialize(key, buffer, pointers, str_count)?;
        serialize::serialize(val, buffer, pointers, str_count)?;
    }
    return Ok(());
}

#[inline(always)]
unsafe fn encode_dict_key(buffer: &mut Vec<u8>, pointers: &mut Pointers, str_count: &mut usize, key: *mut PyObject) {
    let mut len = 0;
    let data = PyUnicode_AsUTF8AndSize(key, &mut len);
    let encoded_as_pointer = try_encode_as_pointer(&key, buffer, pointers, *str_count, len, &NOT_A_STR_BUT_A_POINTER_FLAG);
    if !encoded_as_pointer {
        *str_count += 1;
        encode_number::<NUMBER_BASE>(buffer, len as u128);
        buffer.extend_from_slice(std::slice::from_raw_parts(
            data as *const u8,
            len as usize,
        ));
    }
}

unsafe fn is_consistent_type_list(obj: *mut PyObject, is_list: bool, len: Py_ssize_t) -> bool {
    let first_type = (*if is_list { list_get_item(obj, 0) } else { tuple_get_item(obj, 0) }).ob_type;
    for i in 1..len {
        let item = if is_list {
            list_get_item(obj, i)
        } else {
            tuple_get_item(obj, i)
        };
        if (*item).ob_type != first_type {
            return false
        }
    }
    true
}

pub unsafe fn encode_list(obj: *mut PyObject, buffer: &mut Vec<u8>, pointers: &mut Pointers, str_count: &mut usize, typ: *mut PyTypeObject) -> Result<(), *mut PyObject> {
    let is_list = typ == &mut PyList_Type;
    let len = if is_list {
        get_list_size(obj)
    } else {
        get_tuple_size(obj)
    };
    if len == 0 {
        buffer.push(EMPTY_LIST_FLAG);
        return Ok(());
    }

    if is_consistent_type_list(obj, is_list, len) {
        let first_item = if is_list { list_get_item(obj, 0) } else { tuple_get_item(obj, 0) };
        if first_item == Py_None() {
            buffer.push(CONSISTENT_TYPE_LIST_FLAG);
            buffer.push(NULL_FLAG);
            encode_number::<NUMBER_BASE>(buffer, len as u128);
            return Ok(());
        }
        let first_type = (*first_item).ob_type;
        if first_type == &mut PyUnicode_Type {
            // todo?
            // don't do anything special
        } else if first_type == &mut PyBool_Type {
            encode_bool_list(obj, buffer, is_list, len);
            return Ok(())
        } else if first_type == &mut PyLong_Type {
            // buffer.push(CONSISTENT_TYPE_LIST_FLAG);
            // buffer.push(INT_FLAG);
            // encode_number::<NUMBER_BASE>(buffer, len as u128);
            // for i in 0..len {
            //     let item = if is_list {
            //         list_get_item(obj, i)
            //     } else {
            //         tuple_get_item(obj, i)
            //     };
            //
            //     if check_if_python_number_is_negative(item) {
            //         buffer.push((NUMBER_BASE - 1) as u8);
            //         let negative_item = PyNumber_Negative(item);
            //         encode_python_int::<{NUMBER_BASE-1}, true>(negative_item, buffer);
            //         Py_DECREF(negative_item);
            //     } else {
            //         encode_python_int::<{NUMBER_BASE-1}, true>(item, buffer);
            //     }
            // }
            // return Ok(())
        }
    }

    serialize_normal_list(obj, buffer, pointers, is_list, len, str_count)
}

#[inline(always)]
unsafe fn encode_bool_list(obj: *mut PyObject, buffer: &mut Vec<u8>, is_list: bool, len: isize) {
    buffer.push(CONSISTENT_TYPE_LIST_FLAG);
    buffer.push(BOOL_FLAG);
    encode_number::<NUMBER_BASE>(buffer, len as u128);

    let mut byte: u8 = 0;
    let mut n: u8 = 0;

    for i in 0..len {
        let item = if is_list {
            list_get_item(obj, i)
        } else {
            tuple_get_item(obj, i)
        };
        byte = (byte << 1) | ((item == Py_True()) as u8);
        n += 1;

        if n == 8 {
            buffer.push(byte);
            byte = 0;
            n = 0;
        }
    }

    if n != 0 {
        buffer.push(byte << (8 - n));
    }
}

#[inline(always)]
unsafe fn serialize_normal_list(
    obj: *mut PyObject, buf: &mut Vec<u8>, pointers: &mut Pointers, is_list: bool, len: Py_ssize_t, str_count: &mut usize
) -> Result<(), *mut PyObject>{
    buf.push(LIST_FLAG);
    encode_number::<NUMBER_BASE>(buf, len as u128);
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