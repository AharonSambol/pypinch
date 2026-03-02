use std::ffi::c_char;

use pyo3_ffi::{Py_DECREF, Py_False, Py_INCREF, Py_None, Py_True, PyBytes_FromStringAndSize, PyNumber_Negative, PyObject};
use rustc_hash::FxHashMap;

use crate::deserializing::string_cache::StringCache;
use crate::deserializing::utils::{decode_large_number, decode_number__py_ssize_t, decode_number__usize};
use crate::utils::consts::NUMBER_BASE;


#[inline(always)]
pub unsafe fn decode_bytes(buf: &[u8], ptr: &mut usize) -> *mut PyObject {
    let len = decode_number__py_ssize_t::<NUMBER_BASE>(buf, ptr);
    let bytes = PyBytes_FromStringAndSize(
        buf.as_ptr().add(*ptr) as *const c_char,
        len,
    );
    *ptr += len as usize;
    bytes
}

#[inline(always)]
pub unsafe fn decode_pointer(buf: &[u8], ptr: &mut usize, pointers: &mut FxHashMap<usize, *mut PyObject>) -> *mut PyObject {
    let pos = decode_number__usize::<NUMBER_BASE>(buf, ptr);
    let res = pointers[&pos];
    Py_INCREF(res);
    res
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
pub unsafe fn decode_negative_int(buf: &[u8], ptr: &mut usize) -> *mut PyObject {
    let num = decode_large_number::<NUMBER_BASE>(buf, ptr);
    let res = PyNumber_Negative(num);
    Py_DECREF(num);
    res
}

#[inline(always)]
pub unsafe fn decode_string<'a>(
    buf: &'a [u8],
    ptr: &mut usize,
    pointers: &mut FxHashMap<usize, *mut PyObject>,
    string_cache: &mut StringCache<'a>,
    str_count: &mut usize,
) -> *mut PyObject {
    let len = decode_number__usize::<NUMBER_BASE>(buf, ptr);

    let string = string_cache.get_or_create(&buf[*ptr..*ptr + len]);
    *ptr += len;

    pointers.insert(*str_count, string);
    *str_count += 1;
    string
}

#[inline(always)]
pub unsafe fn decode_f64(buf: &[u8], ptr: &mut usize) -> f64 {
    let p = buf.as_ptr().add(*ptr) as *const u64;
    *ptr += 8;
    f64::from_bits(u64::from_be(std::ptr::read_unaligned(p)))
}