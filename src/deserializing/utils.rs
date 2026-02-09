use std::ffi::{c_long, c_ulonglong};
use pyo3_ffi::{Py_DECREF, PyLong_FromLong, PyLong_FromUnsignedLongLong, PyNumber_Add, PyNumber_Multiply, PyObject};
use crate::utils::consts::ENDING_FLAG;

#[inline(always)]
pub unsafe fn decode_number<const BASE: u128>(
    buf: &[u8],
    ptr: &mut usize,
) -> u128 {
    let b = *buf.get_unchecked(*ptr);
    *ptr += 1;
    if b != ENDING_FLAG {
        return b as u128;
    }

    let mut res: u128 = 0;
    let mut mul: u128 = 1;

    loop {
        let v = *buf.get_unchecked(*ptr);
        *ptr += 1;
        if v == ENDING_FLAG {
            return res;
        }
        res += (v as u128) * mul;
        mul *= BASE;
    }
}

#[inline(always)]
pub unsafe fn decode_large_number<const BASE: u128>(
    buf: &[u8],
    ptr: &mut usize,
) -> *mut PyObject {
    let b = *buf.get_unchecked(*ptr);
    *ptr += 1;
    if b != ENDING_FLAG {
        return PyLong_FromLong(b as c_long);
    }

    let mut num_length = 1;
    let mut temp_ptr = 0;
    loop {
        if *buf.get_unchecked(*ptr + temp_ptr) == ENDING_FLAG {
            break
        }
        num_length += 1;
        temp_ptr += 1;
    }
    // 64 is the amount of bytes in a c_longlong
    if num_length <= 8 {
        *ptr -= 1;
        return PyLong_FromUnsignedLongLong(decode_number::<BASE>(buf, ptr) as c_ulonglong);
    }


    let mut res: u128 = 0;
    let mut mul: u128 = 1;
    for _ in 0..8 {
        let byte = *buf.get_unchecked(*ptr);
        *ptr += 1;
        res += (byte as u128) * mul;
        mul *= BASE;
    }


    let mut result = PyLong_FromUnsignedLongLong(res as c_ulonglong);
    let mut mul = PyLong_FromUnsignedLongLong(mul as c_ulonglong);
    let base_as_long = PyLong_FromLong(BASE as c_long);
    
    loop {
        let v = *buf.get_unchecked(*ptr);
        *ptr += 1;
        if v == ENDING_FLAG {
            Py_DECREF(mul);
            Py_DECREF(base_as_long);

            return result;
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

// #[inline(always)]
// unsafe fn decode_number<const BASE: u128>(
//     buf: &mut *const [u8],
// ) -> u128 {
//     let b = buf[0];
//     *buf = buf.add(1);
//     if b != ENDING_FLAG {
//         return b as u128;
//     }
//
//
//     let mut res: u128 = 0;
//     let mut mul: u128 = 1;
//
//     loop {
//         let v = buf[0];
//         *buf = buf.add(1);
//         if v == ENDING_FLAG {
//             return res;
//         }
//         res += (v as u128) * mul;
//         mul *= BASE;
//     }
// }
