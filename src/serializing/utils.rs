use std::ptr;

use pyo3_ffi::{PyDict_Next, PyObject, PyUnicode_Type};
use crate::serializing::py_bytes_buffer::PyBytesBuffer;
use crate::utils::consts::NUMBER_BASE;

pub static mut EMPTY_TUPLE: *mut PyObject = ptr::null_mut();
pub static mut EMPTY_STRING: *mut PyObject = ptr::null_mut();
pub static mut EMPTY_BYTES: *mut PyObject = ptr::null_mut();
pub static mut SERIALIZATION_ERROR_TYPE: *mut PyObject = ptr::null_mut();

const ENCODED_NUMBER_LIMITS: [u128; 18] = [
    254,
    255,
    255 + 255 - 1,
    255*255 + 255 - 1,
    255*255*255 + 255 - 1,
    255*255*255*255 + 255 - 1,
    255*255*255*255*255 + 255 - 1,
    255*255*255*255*255*255 + 255 - 1,
    255*255*255*255*255*255*255 + 255 - 1,
    255*255*255*255*255*255*255*255 + 255 - 1,
    255*255*255*255*255*255*255*255*255 + 255 - 1,
    255*255*255*255*255*255*255*255*255*255 + 255 - 1,
    255*255*255*255*255*255*255*255*255*255*255 + 255 - 1,
    255*255*255*255*255*255*255*255*255*255*255*255 + 255 - 1,
    255*255*255*255*255*255*255*255*255*255*255*255*255 + 255 - 1,
    255*255*255*255*255*255*255*255*255*255*255*255*255*255 + 255 - 1,
    255*255*255*255*255*255*255*255*255*255*255*255*255*255*255 + 255 - 1,
    255*255*255*255*255*255*255*255*255*255*255*255*255*255*255*255 + 255 - 1,
];

#[inline(always)]
pub unsafe fn encode_number<const BASE: u128>(buf: &mut PyBytesBuffer, mut number: u128) -> Result<(), *mut PyObject> {
    if number < BASE {
        buf.push(number as u8)
    } else {
        buf.push(NUMBER_BASE as u8)?;
        number -= BASE;
        while number != 0 {
            let remainder = number % BASE;
            number /= BASE;
            buf.push(remainder as u8)?;
        }
        buf.push(NUMBER_BASE as u8)
    }
}

#[inline(always)]
pub unsafe fn all_dict_keys_are_str(obj: *mut PyObject) -> bool {
    let mut pos = 0;
    let mut key: *mut PyObject = ptr::null_mut();
    let mut val: *mut PyObject = ptr::null_mut();
    while PyDict_Next(obj, &mut pos, &mut key, &mut val) != 0 {
        if (*key).ob_type != &mut PyUnicode_Type {
            return false
        }
    }
    true
}

#[inline(always)]
pub unsafe fn predict_encoded_number_length(number: u128) -> usize {
    let mut predicted_digits = 1;
    for limit in ENCODED_NUMBER_LIMITS {
        if number <= limit {
            break
        }
        predicted_digits += 1;
    }
    predicted_digits
}