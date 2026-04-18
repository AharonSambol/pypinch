use crate::utils::consts::ENDING_FLAG;
use crate::utils::safe_py_pointer::PyPointer;
use crate::{raise_mem_error_if_null, safe_get};
use pyo3_ffi::{
    PyLong_FromLong, PyLong_FromUnsignedLongLong, PyNumber_Add, PyNumber_Multiply, PyObject, Py_ssize_t,
};
use std::ffi::{c_long, c_ulonglong};
use std::ptr;

pub static mut DESERIALIZATION_ERROR_TYPE: *mut PyObject = ptr::null_mut();

macro_rules! _decode_number {
    ($buf:expr, $ptr:expr, $base:expr, $type:ty) => {{
        let byte = *safe_get!($buf, *$ptr);
        *$ptr += 1;

        if byte != ENDING_FLAG {
            return Ok(byte as $type);
        }
        let mut res = $base;
        let mut mul = 1;

        loop {
            let byte = *safe_get!($buf, *$ptr);
            *$ptr += 1;
            if byte == ENDING_FLAG {
                break Ok(res);
            }
            res += (byte as $type) * mul;
            mul *= $base;
        }
    }};
}

pub fn skip_number(buf: &[u8], ptr: &mut usize) -> Result<(), *mut PyObject> {
    let byte = *safe_get!(buf, *ptr);
    *ptr += 1;

    if byte != ENDING_FLAG {
        return Ok(());
    }
    loop {
        let byte = *safe_get!(buf, *ptr);
        *ptr += 1;
        if byte == ENDING_FLAG {
            break Ok(());
        }
    }
}

#[inline(always)]
pub fn decode_number_usize<const BASE: u128>(
    buf: &[u8],
    ptr: &mut usize,
) -> Result<usize, *mut PyObject> {
    _decode_number!(buf, ptr, BASE as usize, usize)
}

#[inline(always)]
pub fn decode_number_py_ssize_t<const BASE: u128>(
    buf: &[u8],
    ptr: &mut usize,
) -> Result<Py_ssize_t, *mut PyObject> {
    _decode_number!(buf, ptr, BASE as Py_ssize_t, Py_ssize_t)
}

#[inline(always)]
pub fn decode_number_c_ulonglong<const BASE: u128>(
    buf: &[u8],
    ptr: &mut usize,
) -> Result<c_ulonglong, *mut PyObject> {
    _decode_number!(buf, ptr, BASE as c_ulonglong, c_ulonglong)
}

#[inline(always)]
pub fn decode_large_number<const BASE: u128>(
    buf: &[u8],
    ptr: &mut usize,
) -> Result<*mut PyObject, *mut PyObject> {
    let byte = *safe_get!(buf, *ptr);
    *ptr += 1;
    if byte != ENDING_FLAG {
        return Ok(raise_mem_error_if_null!(unsafe {
            PyLong_FromLong(byte as c_long)
        }));
    }

    let mut num_length = 1;
    let mut temp_ptr = 0;
    loop {
        if *safe_get!(buf, *ptr + temp_ptr) == ENDING_FLAG {
            break;
        }
        num_length += 1;
        temp_ptr += 1;
    }
    let bytes_in_c_ulonglong = c_ulonglong::BITS / 8;
    if num_length <= bytes_in_c_ulonglong {
        *ptr -= 1;
        let res = decode_number_c_ulonglong::<BASE>(buf, ptr)?;
        unsafe {
            return Ok(raise_mem_error_if_null!(PyLong_FromUnsignedLongLong(res)));
        }
    }

    let mut res: u128 = BASE;
    let mut mul: u128 = 1;
    for _ in 0..bytes_in_c_ulonglong {
        let byte = *safe_get!(buf, *ptr);
        *ptr += 1;
        res += (byte as u128) * mul;
        mul *= BASE;
    }

    unsafe {
        let mut result = PyPointer::new_w_null_check(PyLong_FromUnsignedLongLong(res as c_ulonglong))?;
        let mut mul = PyPointer::new_w_null_check(PyLong_FromUnsignedLongLong(mul as c_ulonglong))?;
        let base_as_long = PyPointer::new_w_null_check(PyLong_FromLong(BASE as c_long))?;

        loop {
            let byte = *safe_get!(buf, *ptr);
            *ptr += 1;
            if byte == ENDING_FLAG {
                return Ok(result.release());
            }
            let cur_byte_as_long = PyPointer::new_w_null_check(PyLong_FromLong(byte as c_long))?;
            let tmp = PyPointer::new_w_null_check(PyNumber_Multiply(cur_byte_as_long.as_ptr(), mul.as_ptr()))?;
            let new_result = PyPointer::new_w_null_check(PyNumber_Add(result.as_ptr(), tmp.as_ptr()))?;
            result = new_result;

            let tmp = PyPointer::new_w_null_check(PyNumber_Multiply(mul.as_ptr(), base_as_long.as_ptr()))?;
            mul = tmp;
        }
    }
}
