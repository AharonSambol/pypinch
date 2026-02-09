use std::ptr;

use pyo3_ffi::{PyDict_Next, PyObject, PyUnicode_Type};

use crate::utils::consts::NUMBER_BASE;


#[inline(always)]
pub unsafe fn encode_number<const BASE: u128>(buf: &mut Vec<u8>, mut number: u128) {
    if number < BASE {
        buf.push(number as u8);
    } else {
        buf.push(NUMBER_BASE as u8);
        while number != 0 {
            let remainder = number % BASE;
            number /= BASE;
            buf.push(remainder as u8);
        }
        buf.push(NUMBER_BASE as u8);
    }
}

pub unsafe fn all_dict_keys_are_str(obj: *mut PyObject) -> bool {
    let mut pos = 0;
    let mut key: *mut PyObject = ptr::null_mut();
    let mut val: *mut PyObject = ptr::null_mut();
    while PyDict_Next(obj, &mut pos, &mut key, &mut val) != 0 {
        if (*val).ob_type != &mut PyUnicode_Type {
            return false
        }
    }
    true
}

