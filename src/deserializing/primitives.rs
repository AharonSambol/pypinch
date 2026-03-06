use std::ffi::c_char;
use pyo3_ffi::{Py_DECREF, Py_False, Py_INCREF, Py_None, Py_True, PyBytes_FromStringAndSize, PyFloat_FromDouble, PyNumber_Negative, PyObject};
use rustc_hash::FxHashMap;

use crate::deserializing::deserializing_string_cache::StringCache;
use crate::deserializing::utils::{decode_large_number, decode_number_py_ssize_t, decode_number_usize};
use crate::deserializing::utils::DESERIALIZATION_ERROR_TYPE;
use crate::{raise_mem_error_if_null, safe_get};
use crate::utils::consts::{INVALID_UTF_8_START_BYTE_COMPACT_ASCII, NUMBER_BASE, IsAscii, YES_ASCII, NOT_ASCII, MIGHT_BE_ASCII, UNEXPECTED_END_OF_INPUT, CORRUPTED_DATA};
use crate::utils::py_helpers::ToPyErr;

#[inline(always)]
pub unsafe fn decode_bytes(buf: &[u8], ptr: &mut usize) -> Result<*mut PyObject, *mut PyObject> {
    let len = decode_number_py_ssize_t::<NUMBER_BASE>(buf, ptr)?;
    let bytes = raise_mem_error_if_null!(PyBytes_FromStringAndSize(
        buf.as_ptr().add(*ptr) as *const c_char,
        len,
    ));
    *ptr += len as usize;
    Ok(bytes)
}

#[inline(always)]
pub unsafe fn decode_pointer(buf: &[u8], ptr: &mut usize, pointers: &mut FxHashMap<usize, *mut PyObject>) -> Result<*mut PyObject, *mut PyObject> {
    let pos = decode_number_usize::<NUMBER_BASE>(buf, ptr)?;
    let res = *safe_get!(pointers, &pos, CORRUPTED_DATA);
    Py_INCREF(res);
    Ok(res)
}

#[inline(always)]
pub unsafe fn decode_null() -> *mut PyObject {
    let none = Py_None();
    Py_INCREF(none);
    none
}

#[inline(always)]
pub unsafe fn decode_false() -> *mut PyObject {
    let f = Py_False();
    Py_INCREF(f);
    f
}

#[inline(always)]
pub unsafe fn decode_true() -> *mut PyObject {
    let t = Py_True();
    Py_INCREF(t);
    t
}

#[inline(always)]
pub unsafe fn decode_negative_int(buf: &[u8], ptr: &mut usize) -> Result<*mut PyObject, *mut PyObject> {
    let num = decode_large_number::<NUMBER_BASE>(buf, ptr)?;
    let res = raise_mem_error_if_null!(PyNumber_Negative(num));
    Py_DECREF(num);
    Ok(res)
}

#[inline(always)]
pub unsafe fn decode_string<'a, const IS_ASCII: IsAscii, const BASE: u128>(
    buf: &'a [u8],
    ptr: &mut usize,
    pointers: &mut FxHashMap<usize, *mut PyObject>,
    string_cache: &mut StringCache<'a>,
    str_count: &mut usize,
) -> Result<*mut PyObject, *mut PyObject> {
    let len = decode_number_usize::<BASE>(buf, ptr)?;
    if *ptr + len > buf.len() {
        return Err(UNEXPECTED_END_OF_INPUT.to_py_error(DESERIALIZATION_ERROR_TYPE))
    }
    let string = match IS_ASCII {
        YES_ASCII => string_cache.get_or_create::<true>(&buf[*ptr..*ptr + len])?,
        NOT_ASCII => string_cache.get_or_create::<false>(&buf[*ptr..*ptr + len])?,
        MIGHT_BE_ASCII => {
            if *buf.get_unchecked(*ptr) == INVALID_UTF_8_START_BYTE_COMPACT_ASCII {
                string_cache.get_or_create::<true>(&buf[*ptr + 1..*ptr + len])?
            } else {
                string_cache.get_or_create::<false>(&buf[*ptr..*ptr + len])?
            }
        },
        _ => unreachable!()
    };
    *ptr += len;
    pointers.insert(*str_count, string);
    *str_count += 1;
    Ok(string)
}

pub unsafe fn decode_f64(buf: &[u8], ptr: &mut usize) -> Result<*mut PyObject, *mut PyObject> {
    let p = buf.as_ptr().add(*ptr) as *const u64;
    *ptr += 8;
    let float = f64::from_bits(u64::from_be(std::ptr::read_unaligned(p)));
    let py_float = raise_mem_error_if_null!(PyFloat_FromDouble(float));
    Ok(py_float)
}