use pyo3_ffi::{Py_DECREF, Py_INCREF, PyDict_SetItem, PyObject};
use rustc_hash::FxHashMap;

use crate::{safe_get, safe_new_py_dict, safe_new_py_list};
use crate::deserializing::deserialize::deserialize_object;
use crate::deserializing::primitives::decode_string;
use crate::deserializing::string_cache::StringCache;
use crate::deserializing::utils::{decode_number_py_ssize_t, decode_number_usize};
use crate::utils::consts::{CORRUPTED_DATA, MIGHT_BE_ASCII, NUMBER_BASE};
use crate::utils::wrappers::{list_set_item, tuple_set_item};

#[inline(always)]
pub unsafe fn decode_list<'a>(
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
pub unsafe fn decode_str_key_dict<'a>(
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
        PyDict_SetItem(dict, key, value);
        Py_DECREF(key);
        Py_DECREF(value);
    }
    Ok(dict)
}

unsafe fn deserialize_dict_key<'a>(buf: &'a [u8], ptr: &mut usize, pointers: &mut FxHashMap<usize, *mut PyObject>, string_cache: &mut StringCache<'a>, str_count: &mut usize) -> Result<*mut PyObject, *mut PyObject> {
    if *safe_get!(buf, *ptr) == NUMBER_BASE as u8 - 1 {
        *ptr += 1;
        let position = decode_number_usize::<NUMBER_BASE>(buf, ptr)?;
        let res = *safe_get!(pointers, &position, CORRUPTED_DATA);
        Py_INCREF(res);
        Ok(res)
    } else {
        decode_string::<MIGHT_BE_ASCII, { NUMBER_BASE - 1 }>(buf, ptr, pointers, string_cache, str_count)
    }
}

#[inline(always)]
pub unsafe fn decode_dict<'a>(
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
        PyDict_SetItem(dict, key, value);
        Py_DECREF(key);
        Py_DECREF(value);
    }
    Ok(dict)
}
