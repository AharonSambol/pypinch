use pyo3_ffi::{Py_ssize_t, PyBytes_AsString, PyBytes_Size, PyFloatObject, PyObject, PyUnicode_AsUTF8AndSize, PyUnicode_DATA, PyUnicode_GET_LENGTH, PyUnicode_IS_COMPACT_ASCII};
use std::collections::hash_map::Entry;
use std::slice;
use crate::serializing::serialize::Pointers;
use crate::serializing::utils::{encode_number, predict_encoded_number_length};
use crate::utils::consts::{ASCII_STR_FLAG, BYTES_FLAG, EMPTY_BYTES_FLAG, EMPTY_STR_FLAG, FLOAT_FLAG, NUMBER_BASE, POINTER_FLAG, STR_FLAG};

#[inline(always)]
pub unsafe fn serialize_bytes(obj: *mut PyObject, buffer: &mut Vec<u8>) {
    let size = PyBytes_Size(obj);
    let data = PyBytes_AsString(obj);

    if size == 0 {
        buffer.push(EMPTY_BYTES_FLAG);
    } else {
        buffer.push(BYTES_FLAG);
        encode_number::<NUMBER_BASE>(buffer, size as u128);
        buffer.extend_from_slice(slice::from_raw_parts(
            data as *const u8,
            size as usize,
        ));
    }
}

#[inline(always)]
pub unsafe fn serialize_float(obj: *mut PyObject, buffer: &mut Vec<u8>) {
    let value = (*(obj as *mut PyFloatObject)).ob_fval;
    buffer.push(FLOAT_FLAG);
    buffer.extend_from_slice(&value.to_be_bytes());
}

#[inline(always)]
pub unsafe fn serialize_str(obj: *mut PyObject, buffer: &mut Vec<u8>, pointers: &mut Pointers, str_count: &mut usize) {
    let mut len: isize = 0;
    if PyUnicode_IS_COMPACT_ASCII(obj) == 1 {
        let len = PyUnicode_GET_LENGTH(obj) as usize;
        if len == 0 {
            buffer.push(EMPTY_STR_FLAG);
            return;
        }
        if try_encode_as_pointer(
            &obj,
            buffer,
            pointers,
            *str_count,
            len as Py_ssize_t,
            &[POINTER_FLAG],
        ) {
            return;
        }
        // Skip the PyASCIIObject header
        let data_ptr = PyUnicode_DATA(obj) as *const u8;

        *str_count += 1;
        buffer.push(ASCII_STR_FLAG);
        encode_number::<NUMBER_BASE>(buffer, len as u128);
        buffer.extend_from_slice(slice::from_raw_parts(data_ptr, len));
        return;
    }

    let data = PyUnicode_AsUTF8AndSize(obj, &mut len);

    if len == 0 {   // not sure if this is possible
        buffer.push(EMPTY_STR_FLAG);
        return;
    }

    if try_encode_as_pointer(&obj, buffer, pointers, *str_count, len, &[POINTER_FLAG]) {
        return;
    }
    *str_count += 1;
    buffer.push(STR_FLAG);
    encode_number::<NUMBER_BASE>(buffer, len as u128);
    buffer.extend_from_slice(slice::from_raw_parts(
        data as *const u8,
        len as usize,
    ));
}

#[inline(always)]
pub unsafe fn try_encode_as_pointer(str: &*mut PyObject, buffer: &mut Vec<u8>, pointers: &mut Pointers, str_count: usize, str_len: Py_ssize_t, pointer_flag: &[u8]) -> bool {
    match pointers.entry(*str) {
        Entry::Occupied(entry) => {
            let position = (*entry.get()) as u128;
            let predicted_pointer_length = pointer_flag.len() + predict_encoded_number_length(position);
            let predicted_str_length = 1 + str_len as usize + predict_encoded_number_length(str_len as u128);
            // todo update this in the python as well
            if predicted_pointer_length <= predicted_str_length {
                buffer.extend_from_slice(pointer_flag);
                encode_number::<NUMBER_BASE>(buffer, position);
                return true;
            }
        },
        Entry::Vacant(entry) => {
            entry.insert(str_count);
        }
    }
    false
}