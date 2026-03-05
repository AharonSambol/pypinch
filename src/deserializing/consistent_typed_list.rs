use std::ffi::c_char;
use pyo3_ffi::{Py_DECREF, Py_False, Py_INCREF, Py_None, Py_ssize_t, Py_True, PyBytes_FromStringAndSize, PyExc_TypeError, PyFloat_FromDouble, PyList_New, PyNumber_Negative, PyObject, PyTuple_New};
use rustc_hash::FxHashMap;
use crate::deserializing::primitives::{decode_f64, decode_string};
use crate::deserializing::string_cache::StringCache;
use crate::deserializing::utils::{decode_large_number, decode_number_py_ssize_t};
use crate::safe_get;
use crate::utils::consts::{MIGHT_BE_ASCII, NEGATIVE_NUMBER_SIGN, NUMBER_BASE, NULL_FLAG, BOOL_FLAG, INT_FLAG, BYTES_FLAG, STR_FLAG, FLOAT_FLAG, LEFTMOST_BIT_MASK};
use crate::utils::py_helpers::ToPyErr;
use crate::utils::wrappers::{list_set_item, tuple_set_item};

#[inline(always)]
pub unsafe fn decode_consistent_type_list<'a>(
    buf: &'a [u8],
    ptr: &mut usize,
    pointers: &mut FxHashMap<usize, *mut PyObject>,
    use_tuples: bool,
    string_cache: &mut StringCache<'a>,
    str_count: &mut usize,
) -> Result<*mut PyObject, *mut PyObject> {
    let typ = *safe_get!(buf, *ptr);
    *ptr += 1;
    let len = decode_number_py_ssize_t::<NUMBER_BASE>(buf, ptr)?;

    match typ {
        NULL_FLAG => decode_null_list(use_tuples, len),
        // todo all these can create tuples instead of lists
        BOOL_FLAG => decode_bool_list(use_tuples, buf, ptr, len),
        INT_FLAG => decode_int_list(use_tuples, buf, ptr, len),
        BYTES_FLAG => decode_bytes_list(use_tuples, buf, ptr, len),
        STR_FLAG => decode_str_list(use_tuples, buf, ptr, pointers, string_cache, str_count, len),
        FLOAT_FLAG => decode_floats_list(use_tuples, buf, ptr, len),
        _ => {
            return Err("unexpected consistent list type".to_py_error(PyExc_TypeError));
        }
    }
}

unsafe fn decode_floats_list(use_tuples: bool, buf: &[u8], ptr: &mut usize, len: Py_ssize_t) -> Result<*mut PyObject, *mut PyObject> {
    let list = if use_tuples { PyTuple_New(len) } else { PyList_New(len) };
    for i in 0..len {
        let float = decode_f64(buf, ptr);
        let py_float = PyFloat_FromDouble(float);
        if use_tuples {
            tuple_set_item(list, i, py_float);
        } else {
            list_set_item(list, i, py_float);
        }
    }
    Ok(list)
}

unsafe fn decode_str_list<'a>(
    use_tuples: bool,
    buf: &'a [u8],
    ptr: &mut usize,
    pointers: &mut FxHashMap<usize, *mut PyObject>,
    string_cache: &mut StringCache<'a>,
    str_count: &mut usize,
    len: Py_ssize_t
) -> Result<*mut PyObject, *mut PyObject> {
    let list = if use_tuples { PyTuple_New(len) } else { PyList_New(len) };
    for i in 0..len {
        let str = decode_string::<MIGHT_BE_ASCII, NUMBER_BASE>(
            buf,
            ptr,
            pointers,
            string_cache,
            str_count,
        )?;
        if use_tuples {
            tuple_set_item(list, i, str);
        } else {
            list_set_item(list, i, str);
        }
    }
    Ok(list)
}

unsafe fn decode_bytes_list(use_tuples: bool, buf: &[u8], ptr: &mut usize, len: Py_ssize_t) -> Result<*mut PyObject, *mut PyObject> {
    let list = if use_tuples { PyTuple_New(len) } else { PyList_New(len) };
    for i in 0..len {
        let bytes_len = decode_number_py_ssize_t::<NUMBER_BASE>(buf, ptr)?;
        let bytes = PyBytes_FromStringAndSize(
            buf.as_ptr().add(*ptr) as *const c_char,
            bytes_len,
        );
        if use_tuples {
            tuple_set_item(list, i, bytes);
        } else {
            list_set_item(list, i, bytes);
        }
        *ptr += bytes_len as usize;
    }
    Ok(list)
}

unsafe fn decode_int_list(use_tuples: bool, buf: &[u8], ptr: &mut usize, len: Py_ssize_t) -> Result<*mut PyObject, *mut PyObject> {
    let list = if use_tuples { PyTuple_New(len) } else { PyList_New(len) };
    for i in 0..len {
        let is_negative_number = *safe_get!(buf, *ptr) == NEGATIVE_NUMBER_SIGN as u8;
        if is_negative_number {
            *ptr += 1;
            let num = decode_large_number::<{ NUMBER_BASE - 1 }>(buf, ptr)?;
            let negative_num = PyNumber_Negative(num);
            if use_tuples {
                tuple_set_item(list, i, negative_num);
            } else {
                list_set_item(list, i, negative_num);
            }
            Py_DECREF(num);
        } else {
            let num = decode_large_number::<{ NUMBER_BASE - 1 }>(buf, ptr)?;
            if use_tuples {
                tuple_set_item(list, i, num);
            } else {
                list_set_item(list, i, num);
            }
        }
    }
    Ok(list)
}

unsafe fn decode_null_list(use_tuples: bool, len: Py_ssize_t) -> Result<*mut PyObject, *mut PyObject> {
    let none = Py_None();
    let list = if use_tuples { PyTuple_New(len) } else { PyList_New(len) };
    for i in 0..len {
        Py_INCREF(none);
        if use_tuples {
            tuple_set_item(list, i, none);
        } else {
            list_set_item(list, i, none)
        }
    }
    Ok(list)
}

pub unsafe fn decode_bool_list(
    use_tuples: bool,
    buf: &[u8],
    ptr: &mut usize,
    length: Py_ssize_t,
) -> Result<*mut PyObject, *mut PyObject> {
    /*
    same as: math.ceil(length / NUMBER_OF_BITS_IN_BYTE)
    the `>> 3` is like dividing by 8 (8 is `1000` in binary)
    the + 7 is like rounding up
     */
    let amount_of_bytes = ((length as usize) + 7) >> 3;
    let list = if use_tuples { PyTuple_New(length) } else { PyList_New(length) };

    let mut pos = 0;
    let table = [Py_True(), Py_False()];
    for i in 0..amount_of_bytes {
        let mut byte = *safe_get!(buf, *ptr + i);
        for _ in 0..8 {
            let obj = table[((byte & LEFTMOST_BIT_MASK) == 0) as usize];
            Py_INCREF(obj);
            if use_tuples {
                tuple_set_item(list, pos, obj);
            } else {
                list_set_item(list, pos, obj);
            }
            pos += 1;
            if pos == length {
                break;
            }
            byte <<= 1;
        }
    }
    *ptr += amount_of_bytes;
    Ok(list)
}