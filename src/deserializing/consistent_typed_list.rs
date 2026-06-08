use std::ffi::c_char;

use pyo3_ffi::{
    PyBytes_FromStringAndSize, PyExc_TypeError, PyObject, Py_False, Py_INCREF, Py_None, Py_True,
    Py_ssize_t,
};

use crate::deserializing::pointer_holders::pointer_holder::PointerHolder;
use crate::deserializing::primitives::{decode_f64, decode_string};
use crate::deserializing::utils::{decode_number_py_ssize_t, DESERIALIZATION_ERROR_TYPE};
use crate::utils::consts::{BOOL_FLAG, BYTES_FLAG, FLOAT_FLAG, LEFTMOST_BIT_MASK, MIGHT_BE_ASCII, NULL_FLAG, NUMBER_BASE, STR_FLAG, UNEXPECTED_END_OF_INPUT};
use crate::utils::py_helpers::ToPyErr;
use crate::utils::wrappers::{list_set_item, tuple_set_item};
use crate::{raise_mem_error_if_null, safe_get, safe_new_py_list};

#[inline(always)]
pub fn decode_consistent_type_list<'a, P: PointerHolder>(
    buf: &'a [u8],
    ptr: &mut usize,
    pointers: &mut P,
    use_tuples: bool,
) -> Result<*mut PyObject, *mut PyObject> {
    let typ = *safe_get!(buf, *ptr);
    *ptr += 1;
    let len = decode_number_py_ssize_t::<NUMBER_BASE>(buf, ptr)?;

    match typ {
        NULL_FLAG => decode_null_list(use_tuples, len),
        BOOL_FLAG => decode_bool_list(use_tuples, buf, ptr, len),
        BYTES_FLAG => decode_bytes_list(use_tuples, buf, ptr, len),
        STR_FLAG => decode_str_list(use_tuples, buf, ptr, pointers, len),
        FLOAT_FLAG => decode_floats_list(use_tuples, buf, ptr, len),
        _ => Err("Unexpected consistent list type".to_py_error(unsafe { PyExc_TypeError })),
    }
}

fn decode_floats_list(
    use_tuples: bool,
    buf: &[u8],
    ptr: &mut usize,
    len: Py_ssize_t,
) -> Result<*mut PyObject, *mut PyObject> {
    let list = safe_new_py_list!(len, use_tuples);
    for i in 0..len {
        let py_float = decode_f64(buf, ptr)?;
        if use_tuples {
            tuple_set_item(list, i, py_float);
        } else {
            list_set_item(list, i, py_float);
        }
    }
    Ok(list)
}

fn decode_str_list<'a, P: PointerHolder>(
    use_tuples: bool,
    buf: &'a [u8],
    ptr: &mut usize,
    pointers: &mut P,
    len: Py_ssize_t,
) -> Result<*mut PyObject, *mut PyObject> {
    let list = safe_new_py_list!(len, use_tuples);
    for i in 0..len {
        let str = decode_string::<MIGHT_BE_ASCII, NUMBER_BASE, P>(
            buf,
            ptr,
            pointers,
        )?;
        if use_tuples {
            tuple_set_item(list, i, str);
        } else {
            list_set_item(list, i, str);
        }
    }
    Ok(list)
}

fn decode_bytes_list(
    use_tuples: bool,
    buf: &[u8],
    ptr: &mut usize,
    len: Py_ssize_t,
) -> Result<*mut PyObject, *mut PyObject> {
    let list = safe_new_py_list!(len, use_tuples);
    for i in 0..len {
        let bytes_len = decode_number_py_ssize_t::<NUMBER_BASE>(buf, ptr)?;
        let bytes = unsafe {
            if bytes_len as usize + *ptr > buf.len() {
                return Err(UNEXPECTED_END_OF_INPUT.to_py_error(unsafe { DESERIALIZATION_ERROR_TYPE }));
            }
            raise_mem_error_if_null!(PyBytes_FromStringAndSize(
                buf.as_ptr().add(*ptr) as *const c_char,
                bytes_len,
            ))
        };
        if use_tuples {
            tuple_set_item(list, i, bytes);
        } else {
            list_set_item(list, i, bytes);
        }
        *ptr += bytes_len as usize;
    }
    Ok(list)
}

fn decode_null_list(use_tuples: bool, len: Py_ssize_t) -> Result<*mut PyObject, *mut PyObject> {
    let none = unsafe { Py_None() };
    let list = safe_new_py_list!(len, use_tuples);
    for i in 0..len {
        unsafe { Py_INCREF(none) };
        if use_tuples {
            tuple_set_item(list, i, none);
        } else {
            list_set_item(list, i, none)
        }
    }
    Ok(list)
}

pub fn decode_bool_list(
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
    let list = safe_new_py_list!(length, use_tuples);

    let mut pos = 0;
    let table = unsafe { [Py_True(), Py_False()] };
    for i in 0..amount_of_bytes {
        let mut byte = *safe_get!(buf, *ptr + i);
        for _ in 0..8 {
            let obj = table[((byte & LEFTMOST_BIT_MASK) == 0) as usize];
            unsafe { Py_INCREF(obj) };
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
