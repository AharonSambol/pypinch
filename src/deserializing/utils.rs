use std::ffi::{c_long, c_ulonglong};
use pyo3_ffi::{Py_DECREF, Py_ssize_t, PyLong_FromLong, PyLong_FromUnsignedLongLong, PyNumber_Add, PyNumber_Multiply, PyObject};
use crate::safe_get;
use crate::utils::consts::ENDING_FLAG;

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
#[inline(always)]
pub unsafe fn decode_number_usize<const BASE: u128>(
    buf: &[u8],
    ptr: &mut usize,
) -> Result<usize, *mut PyObject> {
    _decode_number!(buf, ptr, BASE as usize, usize)
}

#[inline(always)]
pub unsafe fn decode_number_py_ssize_t<const BASE: u128>(
    buf: &[u8],
    ptr: &mut usize,
) -> Result<Py_ssize_t, *mut PyObject> {
    _decode_number!(buf, ptr, BASE as Py_ssize_t, Py_ssize_t)
}

#[inline(always)]
pub unsafe fn decode_number_c_ulonglong<const BASE: u128>(
    buf: &[u8],
    ptr: &mut usize,
) -> Result<c_ulonglong, *mut PyObject> {
    _decode_number!(buf, ptr, BASE as c_ulonglong, c_ulonglong)
}

#[inline(always)]
pub unsafe fn decode_large_number<const BASE: u128>(
    buf: &[u8],
    ptr: &mut usize,
) -> Result<*mut PyObject, *mut PyObject> {
    let b = *safe_get!(buf, *ptr);
    *ptr += 1;
    if b != ENDING_FLAG {
        return Ok(PyLong_FromLong(b as c_long));
    }

    let mut num_length = 1;
    let mut temp_ptr = 0;
    loop {
        if *safe_get!(buf, *ptr + temp_ptr) == ENDING_FLAG {
            break
        }
        num_length += 1;
        temp_ptr += 1;
    }
    let bytes_in_c_ulonglong = c_ulonglong::BITS / 8;
    if num_length <= bytes_in_c_ulonglong {
        *ptr -= 1;
        return Ok(PyLong_FromUnsignedLongLong(decode_number_c_ulonglong::<BASE>(buf, ptr)?));
    }


    let mut res: u128 = BASE;
    let mut mul: u128 = 1;
    for _ in 0..bytes_in_c_ulonglong {
        let byte = *safe_get!(buf, *ptr);
        *ptr += 1;
        res += (byte as u128) * mul;
        mul *= BASE;
    }


    let mut result = PyLong_FromUnsignedLongLong(res as c_ulonglong);
    let mut mul = PyLong_FromUnsignedLongLong(mul as c_ulonglong);
    let base_as_long = PyLong_FromLong(BASE as c_long);
    
    loop {
        let v = *safe_get!(buf, *ptr);
        *ptr += 1;
        if v == ENDING_FLAG {
            Py_DECREF(mul);
            Py_DECREF(base_as_long);

            return Ok(result);
        }
        let cur_byte_as_long = PyLong_FromLong(v as c_long);
        let tmp = PyNumber_Multiply(cur_byte_as_long, mul);
        Py_DECREF(cur_byte_as_long);
        let new_result = PyNumber_Add(result, tmp);
        Py_DECREF(tmp);
        Py_DECREF(result);
        result = new_result;
        
        let tmp = PyNumber_Multiply(mul, base_as_long);
        Py_DECREF(mul);
        mul = tmp;
    }
}
