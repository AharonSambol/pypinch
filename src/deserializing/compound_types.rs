use pyo3_ffi::{Py_DECREF, Py_False, Py_INCREF, Py_ssize_t, Py_True, PyDict_New, PyDict_SetItem, PyList_New, PyObject, PyTuple_New};
use rustc_hash::FxHashMap;
use crate::deserializing::deserialize::deserialize_object;
use crate::deserializing::primitives::{decode_string};
use crate::deserializing::string_cache::StringCache;
use crate::deserializing::utils::{decode_number_py_ssize_t, decode_number_usize};
use crate::utils::consts::{LEFTMOST_BIT_MASK, NOT_A_STR_BUT_A_POINTER_FLAG, NUMBER_BASE, MIGHT_BE_ASCII};
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
    let len = decode_number_py_ssize_t::<NUMBER_BASE>(buf, ptr);

    if use_tuples {
        let tup = PyTuple_New(len);
        for i in 0..len {
            let obj = deserialize_object(buf, ptr, pointers, use_tuples, string_cache, str_count)?;
            tuple_set_item(tup, i, obj);
        }
        Ok(tup)
    } else {
        let list = PyList_New(len);
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
    let len = decode_number_usize::<NUMBER_BASE>(buf, ptr);
    let dict = PyDict_New();
    for _ in 0..len {
        let key = deserialize_dict_key(buf, ptr, pointers, string_cache, str_count);
        let value = deserialize_object(buf, ptr, pointers, use_tuples, string_cache, str_count)?;
        PyDict_SetItem(dict, key, value);
        Py_DECREF(key);
        Py_DECREF(value);
    }
    Ok(dict)
}

unsafe fn deserialize_dict_key<'a>(buf: &'a [u8], ptr: &mut usize, pointers: &mut FxHashMap<usize, *mut PyObject>, string_cache: &mut StringCache<'a>, str_count: &mut usize) -> *mut PyObject {
    if buf[*ptr..*ptr + NOT_A_STR_BUT_A_POINTER_FLAG.len()] == NOT_A_STR_BUT_A_POINTER_FLAG {
        *ptr += NOT_A_STR_BUT_A_POINTER_FLAG.len();
        let position = decode_number_usize::<NUMBER_BASE>(buf, ptr);
        let res = pointers[&position];
        Py_INCREF(res);
        res
    } else {
        decode_string::<MIGHT_BE_ASCII>(buf, ptr, pointers, string_cache, str_count)
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
    let len = decode_number_usize::<NUMBER_BASE>(buf, ptr);
    let dict = PyDict_New();
    for _ in 0..len {
        let key = deserialize_object(buf, ptr, pointers, use_tuples, string_cache, str_count)?;
        let value = deserialize_object(buf, ptr, pointers, use_tuples, string_cache, str_count)?;
        PyDict_SetItem(dict, key, value);
        Py_DECREF(key);
        Py_DECREF(value);
    }
    Ok(dict)
}


#[inline(always)]
pub unsafe fn decode_bool_list(
    buf: &[u8],
    ptr: &mut usize,
    length: Py_ssize_t,
) -> *mut PyObject {
    /*
    same as: math.ceil(length / NUMBER_OF_BITS_IN_BYTE)
    the `>> 3` is like dividing by 8 (8 is `1000` in binary)
    the + 7 is like rounding up
     */
    let amount_of_bytes = ((length as usize) + 7) >> 3;
    let list = PyList_New(length);

    let mut pos = 0;
    let table = [Py_True(), Py_False()];
    for i in 0..amount_of_bytes {
        let mut byte = buf[*ptr + i];
        for _ in 0..8 {
            let obj = table[((byte & LEFTMOST_BIT_MASK) == 0) as usize];
            Py_INCREF(obj);
            list_set_item(list, pos, obj);
            pos += 1;
            if pos == length {
                break;
            }
            byte <<= 1;
        }
    }
    *ptr += amount_of_bytes;
    list
}