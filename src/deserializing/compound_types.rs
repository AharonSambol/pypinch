use pyo3_ffi::{Py_DECREF, Py_INCREF, Py_ssize_t, PyDict_SetItem, PyObject};
use rustc_hash::FxHashMap;

use crate::{safe_get, safe_new_py_dict, safe_new_py_list};
use crate::deserializing::deserialize::deserialize_object;
use crate::deserializing::primitives::decode_string;
use crate::deserializing::deserializing_string_cache::StringCache;
use crate::deserializing::utils::{decode_number_py_ssize_t, decode_number_usize, DESERIALIZATION_ERROR_TYPE};
use crate::utils::consts::{CORRUPTED_DATA, MIGHT_BE_ASCII, NUMBER_BASE};
use crate::utils::py_helpers::{pretty_type, ToPyErr};
use crate::utils::wrappers::{list_set_item, tuple_set_item};

#[inline(always)]
pub fn decode_list<'a>(
    buf: &'a [u8],
    ptr: &mut usize,
    pointers: &mut FxHashMap<usize, *mut PyObject>,
    use_tuples: bool,
    string_cache: &mut StringCache<'a>,
    str_count: &mut usize,
) -> Result<*mut PyObject, *mut PyObject> {
    let len = decode_number_py_ssize_t::<NUMBER_BASE>(buf, ptr)?;

    if use_tuples {
        let tup = safe_new_py_list!(len, true);
        for i in 0..len {
            let obj = deserialize_object(buf, ptr, pointers, use_tuples, string_cache, str_count)?;
            tuple_set_item(tup, i, obj);
        }
        Ok(tup)
    } else {
        let list = safe_new_py_list!(len, false);
        for i in 0..len {
            let obj = deserialize_object(buf, ptr, pointers, use_tuples, string_cache, str_count)?;
            list_set_item(list, i, obj);
        }
        Ok(list)
    }
}

#[inline(always)]
pub fn decode_str_key_dict<'a>(
    buf: &'a [u8],
    ptr: &mut usize,
    pointers: &mut FxHashMap<usize, *mut PyObject>,
    use_tuples: bool,
    string_cache: &mut StringCache<'a>,
    str_count: &mut usize,
) -> Result<*mut PyObject, *mut PyObject> {
    let len = decode_number_usize::<NUMBER_BASE>(buf, ptr)?;
    let dict = safe_new_py_dict!();
    for _ in 0..len {
        let key = deserialize_dict_key(buf, ptr, pointers, string_cache, str_count)?;
        let value = deserialize_object(buf, ptr, pointers, use_tuples, string_cache, str_count)?;
        unsafe {
            PyDict_SetItem(dict, key, value);
            Py_DECREF(key);
            Py_DECREF(value);
        }
    }
    Ok(dict)
}

fn deserialize_dict_key<'a>(buf: &'a [u8], ptr: &mut usize, pointers: &mut FxHashMap<usize, *mut PyObject>, string_cache: &mut StringCache<'a>, str_count: &mut usize) -> Result<*mut PyObject, *mut PyObject> {
    if *safe_get!(buf, *ptr) == NUMBER_BASE as u8 - 1 {
        *ptr += 1;
        let position = decode_number_usize::<NUMBER_BASE>(buf, ptr)?;
        let res = *safe_get!(pointers, &position, CORRUPTED_DATA);
        unsafe { Py_INCREF(res); }
        Ok(res)
    } else {
        decode_string::<MIGHT_BE_ASCII, { NUMBER_BASE - 1 }>(buf, ptr, pointers, string_cache, str_count)
    }
}

#[inline(always)]
pub fn decode_dict<'a>(
    buf: &'a [u8],
    ptr: &mut usize,
    pointers: &mut FxHashMap<usize, *mut PyObject>,
    use_tuples: bool,
    string_cache: &mut StringCache<'a>,
    str_count: &mut usize,
) -> Result<*mut PyObject, *mut PyObject> {
    let len = decode_number_usize::<NUMBER_BASE>(buf, ptr)?;
    let dict = safe_new_py_dict!();
    for _ in 0..len {
        let key = deserialize_object(buf, ptr, pointers, use_tuples, string_cache, str_count)?;
        let value = deserialize_object(buf, ptr, pointers, use_tuples, string_cache, str_count)?;
        unsafe {
            if PyDict_SetItem(dict, key, value) != 0 {
                return Err(format!("Invalid type for a key: {}", pretty_type(key)).to_py_error(DESERIALIZATION_ERROR_TYPE));
            }
            Py_DECREF(key);
            Py_DECREF(value);
        }
    }
    Ok(dict)
}

pub fn decode_list_of_structured_dicts<'a>(
    buf: &'a [u8],
    ptr: &mut usize,
    pointers: &mut FxHashMap<usize, *mut PyObject>,
    use_tuples: bool,
    string_cache: &mut StringCache<'a>,
    str_count: &mut usize,
) -> Result<*mut PyObject, *mut PyObject> {
    let list_len = decode_number_py_ssize_t::<NUMBER_BASE>(buf, ptr)?;
    let list = safe_new_py_list!(list_len, use_tuples);

    // first dict:
    let dict_len = decode_number_usize::<NUMBER_BASE>(buf, ptr)?;
    let first_dict = safe_new_py_dict!();
    let mut keys = Vec::with_capacity(dict_len);
    for _ in 0..dict_len {
        let key = deserialize_object(buf, ptr, pointers, use_tuples, string_cache, str_count)?;
        let value = deserialize_object(buf, ptr, pointers, use_tuples, string_cache, str_count)?;
        unsafe {
            PyDict_SetItem(first_dict, key, value);
            Py_DECREF(value);
        }
        keys.push(key);
    }
    if use_tuples { tuple_set_item(list, 0, first_dict); } else { list_set_item(list, 0, first_dict); }

    // the rest of the dicts:
    for i in 1usize..list_len as usize {
        let dict = safe_new_py_dict!();
        for key_index in 0..dict_len {
            let value = deserialize_object(buf, ptr, pointers, use_tuples, string_cache, str_count)?;
            unsafe {
                PyDict_SetItem(dict, keys[key_index], value);
                Py_DECREF(value);
            }
        }
        if use_tuples { tuple_set_item(list, i as Py_ssize_t, dict); } else { list_set_item(list, i as Py_ssize_t, dict); }
    }

    // free the keys - PyDict_SetItem doesnt steal the reference
    for key in keys {
        unsafe { Py_DECREF(key); }
    }

    Ok(list)
}
