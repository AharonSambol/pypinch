use std::ffi::c_char;
use pyo3_ffi::{Py_DECREF, Py_INCREF, Py_None, Py_ssize_t, PyBytes_FromStringAndSize, PyExc_TypeError, PyFloat_FromDouble, PyList_New, PyNumber_Negative, PyObject, PyTuple_New};
use rustc_hash::FxHashMap;
use crate::deserializing::compound_types::decode_bool_list;
use crate::deserializing::primitives::{decode_f64, decode_string};
use crate::deserializing::string_cache::StringCache;
use crate::deserializing::utils::{decode_large_number, decode_number__py_ssize_t};
use crate::utils::consts::{NEGATIVE_NUMBER_SIGN, NUMBER_BASE, NULL_FLAG, BOOL_FLAG, INT_FLAG, BYTES_FLAG, STR_FLAG, FLOAT_FLAG};
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
    let typ = *buf.get_unchecked(*ptr);
    *ptr += 1;
    let len = decode_number__py_ssize_t::<NUMBER_BASE>(buf, ptr);

    Ok(match typ {
        NULL_FLAG => decode_null_list(use_tuples, len),
        // todo all these can create tuples instead of lists
        BOOL_FLAG => decode_bool_list(buf, ptr, len),
        INT_FLAG => decode_int_list(buf, ptr, len),
        BYTES_FLAG => decode_bytes_list(buf, ptr, len),
        STR_FLAG => decode_str_list(buf, ptr, pointers, string_cache, str_count, len),
        FLOAT_FLAG => decode_floats_list(buf, ptr, len),
        _ => {
            return Err("unexpected consistent list type".to_py_error(PyExc_TypeError));
        }
    })
}

#[inline(always)]
unsafe fn decode_floats_list(buf: &[u8], ptr: &mut usize, len: Py_ssize_t) -> *mut PyObject {
    let list = PyList_New(len);
    for i in 0..len {
        let float = decode_f64(buf, ptr);
        list_set_item(list, i, PyFloat_FromDouble(float));
    }
    list
}

#[inline(always)]
unsafe fn decode_str_list<'a>(buf: &'a [u8], ptr: &mut usize, pointers: &mut FxHashMap<usize, *mut PyObject>, string_cache: &mut StringCache<'a>, str_count: &mut usize, len: Py_ssize_t) -> *mut PyObject {
    let list = PyList_New(len);
    for i in 0..len {
        let str = decode_string(
            buf,
            ptr,
            pointers,
            string_cache,
            str_count,
        );
        list_set_item(list, i, str);
    }
    list
}

#[inline(always)]
unsafe fn decode_bytes_list(buf: &[u8], ptr: &mut usize, len: Py_ssize_t) -> *mut PyObject {
    let list = PyList_New(len);
    for i in 0..len {
        let bytes_len = decode_number__py_ssize_t::<NUMBER_BASE>(buf, ptr);
        let bytes = PyBytes_FromStringAndSize(
            buf.as_ptr().add(*ptr) as *const c_char,
            bytes_len,
        );
        list_set_item(list, i, bytes);
        *ptr += bytes_len as usize;
    }
    list
}

#[inline(always)]
unsafe fn decode_int_list(buf: &[u8], ptr: &mut usize, len: Py_ssize_t) -> *mut PyObject {
    let list = PyList_New(len);
    for i in 0..len {
        let is_negative_number = *buf.get_unchecked(*ptr) == NEGATIVE_NUMBER_SIGN as u8;
        if is_negative_number {
            *ptr += 1;
            let num = decode_large_number::<{ NUMBER_BASE - 1 }>(buf, ptr);
            list_set_item(list, i, PyNumber_Negative(num));   // todo support larger numbers
            Py_DECREF(num);
        } else {
            let num = decode_large_number::<{ NUMBER_BASE - 1 }>(buf, ptr);
            list_set_item(list, i, num);
        }
    }
    list
}

// todo: turn use_tuples into <const>?
#[inline(always)]
unsafe fn decode_null_list(use_tuples: bool, len: Py_ssize_t) -> *mut PyObject {
    let none = Py_None();

    if use_tuples {
        let tuple = PyTuple_New(len);
        for i in 0..len {
            Py_INCREF(none);
            tuple_set_item(tuple, i, none);
        }
        tuple
    } else {
        let list = PyList_New(len);
        for i in 0..len {
            Py_INCREF(none);
            list_set_item(list, i, none);
        }
        list
    }
}
