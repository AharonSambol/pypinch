use std::ffi::{c_char};
use pyo3_ffi::*;
use rustc_hash::FxHashMap;
use std::ptr;
use crate::utils::consts::{BOOL_FLAG, BYTES_FLAG, CONSISTENT_TYPE_LIST_FLAG, DICT_FLAG, EMPTY_BYTES_FLAG, EMPTY_DICT_FLAG, EMPTY_LIST_FLAG, EMPTY_STR_FLAG, ENDING_FLAG, FALSE_FLAG, FLOAT_FLAG, LIST_FLAG, NEGATIVE_INT_FLAG, NULL_FLAG, NUMBER_BASE, POINTER_FLAG, POSITIVE_INT_FLAG, small_int, STR_FLAG, STR_KEY_DICT_FLAG, TRUE_FLAG};


// todo const base (its usually NUMBER_BASE)
#[inline(always)]
unsafe fn encode_number(buf: &mut Vec<u8>, mut number: u128, base: u8) {
    if number < base as u128 {
        buf.push(number as u8);
    } else {
        buf.push(NUMBER_BASE);
        while number != 0 {
            let remainder = number % (base as u128);
            number /= base as u128;
            buf.push(remainder as u8);
        }
        buf.push(NUMBER_BASE);
    }
}

#[inline(always)]
pub unsafe fn serialize(
    obj: *mut PyObject,
    buffer: &mut Vec<u8>,
    pointers: &mut Option<&mut FxHashMap<*mut PyObject, usize>>,
) {
    let typ = (*obj).ob_type;

    if typ == &mut PyUnicode_Type {
        let mut len: isize = 0;
        let data = PyUnicode_AsUTF8AndSize(obj, &mut len);

        if let Some(pmap) = pointers {
            if let Some(&pos) = pmap.get(&obj) {
                let mut temp_buf = Vec::new();
                temp_buf.push(POINTER_FLAG);
                encode_number(&mut temp_buf, pos as u128, NUMBER_BASE);
                if temp_buf.len() <= (len + 1) as usize {
                    buffer.extend(temp_buf);
                    return;
                }
            } else if len > 0 {
                pmap.insert(obj, buffer.len() + 1);
            }
        }

        if len == 0 {
            buffer.push(EMPTY_STR_FLAG);
        } else {
            buffer.push(STR_FLAG);
            encode_number(buffer, len as u128, NUMBER_BASE);
            buffer.extend_from_slice(std::slice::from_raw_parts(
                data as *const u8,
                len as usize,
            ));
        }
        return;
    }

    if typ == &mut PyBool_Type {
        buffer.push(if obj == Py_True() { TRUE_FLAG } else { FALSE_FLAG });
        return;
    }

    if typ == &mut PyLong_Type {
        encode_python_int(obj, buffer, NUMBER_BASE);
        return;
    }


    if obj == Py_None() {
        buffer.push(NULL_FLAG);
        return;
    }

    if typ == &mut PyList_Type || typ == &mut PyTuple_Type {
        let is_list = typ == &mut PyList_Type;
        let len = if is_list {
            PyList_Size(obj)
        } else {
            PyTuple_Size(obj)
        };
        if len == 0 {
            buffer.push(EMPTY_LIST_FLAG);
            return;
        }
        unsafe fn is_consistent_type_list(obj: *mut PyObject, is_list: bool, len: Py_ssize_t) -> bool {
            let first_type = (*if is_list { PyList_GetItem(obj, 0) } else { PyTuple_GetItem(obj, 0) }).ob_type;
            for i in 1..len {
                let item = if is_list {
                    PyList_GetItem(obj, i)
                } else {
                    PyTuple_GetItem(obj, i)
                };
                if (*item).ob_type != first_type {
                    return false
                }
            }
            true
        }
        if is_consistent_type_list(obj, is_list, len) {
            let first_item = if is_list { PyList_GetItem(obj, 0) } else { PyTuple_GetItem(obj, 0) };
            if first_item == Py_None() {
                buffer.push(CONSISTENT_TYPE_LIST_FLAG);
                buffer.push(NULL_FLAG);
                encode_number(buffer, len as u128, NUMBER_BASE);
                return;
            }
            let first_type = (*first_item).ob_type;
            if first_type == &mut PyUnicode_Type && pointers.is_some() {
                serialize_normal_list(obj, buffer, pointers, is_list, len);
                return;
            }
            // else if first_type == &mut PyLong_Type {}
            else if first_type == &mut PyBool_Type {
                buffer.push(CONSISTENT_TYPE_LIST_FLAG);
                buffer.push(BOOL_FLAG);
                encode_number(buffer, len as u128, NUMBER_BASE);

                let mut byte: u8 = 0;
                let mut n: u8 = 0;

                for i in 0..len {
                    let item = if is_list {
                        PyList_GetItem(obj, i)
                    } else {
                        PyTuple_GetItem(obj, i)
                    };
                    byte = (byte << 1) | ((item == Py_True()) as u8);
                    n += 1;

                    if n == 8 {
                        buffer.push(byte);
                        byte = 0;
                        n = 0;
                    }
                }

                if n != 0 {
                    buffer.push(byte << (8 - n));
                }
                return;
            }
            // if first_type == &mut PyLong_Type {
            //     buffer.push(CONSISTENT_TYPE_LIST_FLAG);
            //     buffer.push(INT_FLAG);
            //     encode_number(buffer, len as u128, NUMBER_BASE);
            //     for i in 0..len {
            //         let item = if is_list {
            //             PyList_GetItem(obj, i)
            //         } else {
            //             PyTuple_GetItem(obj, i)
            //         };
            //         serialize(item, buf, pointers);
            //         if item <= 0 {
            //             buffer.push(NUMBER_BASE - 1);
            //             // todo ignore sign
            //             encode_python_int(obj, buffer, NUMBER_BASE-1);
            //         } else {
            //             encode_python_int(obj, buffer, NUMBER_BASE-1);
            //         }
            //     }
            //
            // }
        }

        serialize_normal_list(obj, buffer, pointers, is_list, len);
        return;
    }

    // dict
    if typ == &mut PyDict_Type {
        let size = PyDict_Size(obj);
        if size == 0 {
            buffer.push(EMPTY_DICT_FLAG);
            return;
        }
        if pointers.is_none() && all_dick_keys_are_str(obj) {
            buffer.push(STR_KEY_DICT_FLAG);
            encode_number(buffer, size as u128, NUMBER_BASE);

            let mut pos = 0;
            let mut key: *mut PyObject = ptr::null_mut();
            let mut val: *mut PyObject = ptr::null_mut();
            while PyDict_Next(obj, &mut pos, &mut key, &mut val) != 0 {
                // key
                let mut len: isize = 0;
                let data = PyUnicode_AsUTF8AndSize(key, &mut len);
                encode_number(buffer, len as u128, NUMBER_BASE);
                buffer.extend_from_slice(std::slice::from_raw_parts(
                    data as *const u8,
                    len as usize,
                ));
                // value
                serialize(val, buffer, pointers);
            }
            return;
        }


        buffer.push(DICT_FLAG);
        encode_number(buffer, size as u128, NUMBER_BASE);

        let mut pos = 0;
        let mut key: *mut PyObject = ptr::null_mut();
        let mut val: *mut PyObject = ptr::null_mut();
        while PyDict_Next(obj, &mut pos, &mut key, &mut val) != 0 {
            serialize(key, buffer, pointers);
            serialize(val, buffer, pointers);
        }
        return;
    }


    if typ == &mut PyFloat_Type {
        let value = (*(obj as *mut PyFloatObject)).ob_fval;
        buffer.push(FLOAT_FLAG);
        buffer.extend_from_slice(&value.to_be_bytes());
        return;
    }

    if typ == &mut PyBytes_Type {
        let size = PyBytes_Size(obj);
        let data = PyBytes_AsString(obj);

        if size == 0 {
            buffer.push(EMPTY_BYTES_FLAG);
        } else {
            buffer.push(BYTES_FLAG);
            encode_number(buffer, size as u128, 255);
            buffer.extend_from_slice(std::slice::from_raw_parts(
                data as *const u8,
                size as usize,
            ));
        }
        return;
    }

    let name = (*typ).tp_name;

    PyErr_Format(
        PyExc_TypeError,
        b"Unsupported type: %s\0".as_ptr() as *const c_char,
        name,
    );
    // PyErr_SetString(PyExc_TypeError, format!("Unsupported type::{}\0", ).as_bytes().as_ptr() as _);
}

unsafe fn all_dick_keys_are_str(obj: *mut PyObject) -> bool {
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

unsafe fn encode_python_int(obj: *mut PyObject, buffer: &mut Vec<u8>, base: u8) {
    let mut overflow = 0;
    let longlong = PyLong_AsLongLongAndOverflow(obj, &mut overflow);

    if overflow == 0 {
        if let Some(byte) = small_int(longlong) {
            buffer.push(byte);
        } else if longlong >= 0 {
            buffer.push(POSITIVE_INT_FLAG);
            encode_number(buffer, longlong as u128, base);
        } else {
            buffer.push(NEGATIVE_INT_FLAG);
            encode_number(buffer, (-longlong) as u128, base);
        }
        return;
    }

    // Huge integer path
    encode_pylong_big(buffer, obj, base as u32);
}

unsafe fn serialize_normal_list(obj: *mut PyObject, buf: &mut Vec<u8>, pointers: &mut Option<&mut FxHashMap<*mut PyObject, usize>>, is_list: bool, len: Py_ssize_t) {
    buf.push(LIST_FLAG);
    encode_number(buf, len as u128, NUMBER_BASE);
    for i in 0..len {
        let item = if is_list {
            PyList_GetItem(obj, i)
        } else {
            PyTuple_GetItem(obj, i)
        };
        serialize(item, buf, pointers);
    }
}

#[inline(always)]
unsafe fn encode_pylong_big(
    buf: &mut Vec<u8>,
    obj: *mut PyObject,
    base: u32
) {
    let nbits = _PyLong_NumBits(obj);
    let nbytes = (nbits + 7) / 8 + 1; // +1 to preserve sign bit

    let mut bytes = Vec::<u8>::with_capacity(nbytes);
    bytes.set_len(nbytes);

    // signed = 1 → two's complement
    let rc = _PyLong_AsByteArray(
        obj as *mut PyLongObject,
            bytes.as_mut_ptr(),
        nbytes,
        0, // big-endian
        1, // signed
    );
    if rc != 0 {
        PyErr_SetString(
            PyExc_RuntimeError,
            b"Failed to extract PyLong bytes\0".as_ptr() as _,
        );
        return;
    }

    // Determine sign from MSB
    let is_negative = (bytes[0] & 0x80) != 0;

    buf.push(if is_negative {
        NEGATIVE_INT_FLAG
    } else {
        POSITIVE_INT_FLAG
    });

    // If negative, convert from two's complement to magnitude
    if is_negative {
        twos_complement_inplace(&mut bytes);
    }

    encode_base_from_bytes(buf, &bytes, base);
}

#[inline(always)]
fn twos_complement_inplace(bytes: &mut [u8]) {
    // invert
    for b in bytes.iter_mut() {
        *b = !*b;
    }

    // add 1
    for b in bytes.iter_mut().rev() {
        let (v, carry) = b.overflowing_add(1);
        *b = v;
        if !carry {
            break;
        }
    }
}

#[inline(always)]
fn encode_base_from_bytes(buf: &mut Vec<u8>, bytes: &[u8], base: u32) {
    // Working copy (big-endian base-256 number)
    let mut work = bytes.to_vec();

    buf.push(ENDING_FLAG);

    while !work.is_empty() {
        let mut carry: u32 = 0;

        for b in work.iter_mut() {
            let v = (carry << 8) | (*b as u32);
            *b = (v / base) as u8;
            carry = v % base;
        }

        // carry is the remainder
        buf.push(carry as u8);

        // Trim leading zero bytes
        while !work.is_empty() && work[0] == 0 {
            work.remove(0);
        }
    }

    buf.push(ENDING_FLAG);
}
