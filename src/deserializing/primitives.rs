use pyo3_ffi::{
    PyBytes_FromStringAndSize, PyFloat_FromDouble, PyNumber_Negative, PyObject, Py_DECREF,
    Py_False, Py_INCREF, Py_None, Py_True,
};
use std::ffi::c_char;

use crate::deserializing::string_creator::create_string;
use crate::deserializing::utils::DESERIALIZATION_ERROR_TYPE;
use crate::deserializing::utils::{
    decode_large_number, decode_number_py_ssize_t, decode_number_usize,
};
use crate::utils::consts::{
    IsAscii, CORRUPTED_DATA, INVALID_UTF_8_START_BYTE_COMPACT_ASCII, MIGHT_BE_ASCII, NOT_ASCII,
    NUMBER_BASE, UNEXPECTED_END_OF_INPUT, YES_ASCII,
};
use crate::utils::py_helpers::ToPyErr;
use crate::{raise_mem_error_if_null, safe_get};

#[inline(always)]
pub fn decode_bytes(buf: &[u8], ptr: &mut usize) -> Result<*mut PyObject, *mut PyObject> {
    let len = decode_number_py_ssize_t::<NUMBER_BASE>(buf, ptr)?;
    let bytes = unsafe {
        raise_mem_error_if_null!(PyBytes_FromStringAndSize(
            buf.as_ptr().add(*ptr) as *const c_char,
            len,
        ))
    };
    *ptr += len as usize;
    Ok(bytes)
}

pub fn decode_pointer(
    buf: &[u8],
    ptr: &mut usize,
    pointers: &mut Vec<*mut PyObject>,
) -> Result<*mut PyObject, *mut PyObject> {
    let pos = decode_number_usize::<NUMBER_BASE>(buf, ptr)?;
    let res = *safe_get!(pointers, pos, CORRUPTED_DATA);
    unsafe {
        Py_INCREF(res);
    }
    Ok(res)
}

#[inline(always)]
pub fn decode_sized_pointer<const SIZE: usize>(
    buf: &[u8],
    ptr: &mut usize,
    pointers: &mut Vec<*mut PyObject>,
) -> Result<*mut PyObject, *mut PyObject> {
    if *ptr + SIZE > buf.len() {
        return Err(UNEXPECTED_END_OF_INPUT.to_py_error(unsafe { DESERIALIZATION_ERROR_TYPE }));
    }
    let pos = unsafe {
        match SIZE {
            1 => {
                let pos = *buf.get_unchecked(*ptr) as usize;
                *ptr += 1;
                pos
            }
            2 => {
                let buf_pointer = buf.as_ptr().add(*ptr) as *const u16;
                *ptr += 2;
                u16::from_be(std::ptr::read_unaligned(buf_pointer)) as usize
            }
            3 => {
                let pos = (*buf.get_unchecked(*ptr) as usize) << 16
                    | (*buf.get_unchecked(*ptr + 1) as usize) << 8
                    | *buf.get_unchecked(*ptr + 2) as usize;
                *ptr += 3;
                pos
            }
            4 => {
                let buf_pointer = buf.as_ptr().add(*ptr) as *const u32;
                *ptr += 4;
                u32::from_be(std::ptr::read_unaligned(buf_pointer)) as usize
            }
            _ => unreachable!(),
        }
    };
    let res = *safe_get!(pointers, pos, CORRUPTED_DATA);
    unsafe {
        Py_INCREF(res);
    }
    Ok(res)
}

#[inline(always)]
pub fn decode_null() -> *mut PyObject {
    unsafe {
        let none = Py_None();
        Py_INCREF(none);
        none
    }
}

#[inline(always)]
pub fn decode_false() -> *mut PyObject {
    unsafe {
        let f = Py_False();
        Py_INCREF(f);
        f
    }
}

#[inline(always)]
pub fn decode_true() -> *mut PyObject {
    unsafe {
        let t = Py_True();
        Py_INCREF(t);
        t
    }
}

#[inline(always)]
pub fn decode_negative_int(buf: &[u8], ptr: &mut usize) -> Result<*mut PyObject, *mut PyObject> {
    let num = decode_large_number::<NUMBER_BASE>(buf, ptr)?;
    unsafe {
        let res = raise_mem_error_if_null!(PyNumber_Negative(num));
        Py_DECREF(num);
        Ok(res)
    }
}

#[inline(always)]
pub fn decode_string<'a, const IS_ASCII: IsAscii, const BASE: u128>(
    buf: &'a [u8],
    ptr: &mut usize,
    pointers: &mut Vec<*mut PyObject>,
    str_count: &mut usize,
) -> Result<*mut PyObject, *mut PyObject> {
    let len = decode_number_usize::<BASE>(buf, ptr)?;
    if *ptr + len > buf.len() {
        return Err(UNEXPECTED_END_OF_INPUT.to_py_error(unsafe { DESERIALIZATION_ERROR_TYPE }));
    }
    let string = match IS_ASCII {
        YES_ASCII => create_string::<true>(&buf[*ptr..*ptr + len])?,
        NOT_ASCII => create_string::<false>(&buf[*ptr..*ptr + len])?,
        MIGHT_BE_ASCII => {
            if unsafe { *buf.get_unchecked(*ptr) } == INVALID_UTF_8_START_BYTE_COMPACT_ASCII {
                create_string::<true>(&buf[*ptr + 1..*ptr + len])?
            } else {
                create_string::<false>(&buf[*ptr..*ptr + len])?
            }
        }
        _ => unreachable!(),
    };
    *ptr += len;
    pointers.insert(*str_count, string);
    *str_count += 1;
    Ok(string)
}

pub fn decode_f64(buf: &[u8], ptr: &mut usize) -> Result<*mut PyObject, *mut PyObject> {
    unsafe {
        let float_pointer = buf.as_ptr().add(*ptr) as *const u64;
        *ptr += 8;
        let float = f64::from_bits(u64::from_be(std::ptr::read_unaligned(float_pointer)));
        let py_float = raise_mem_error_if_null!(PyFloat_FromDouble(float));
        Ok(py_float)
    }
}
