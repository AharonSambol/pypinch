use crate::raise_mem_error_if_null;
use crate::serializing::py_bytes_buffer::PyBytesBuffer;
use crate::serializing::serializing_string_cache::{Pointers, PyStringKey};
use crate::serializing::utils::{
    encode_number, ISO_FORMAT_FUNC, SERIALIZATION_ERROR_TYPE,
};
use crate::utils::consts::{ASCII_STR_FLAG, BYTES_FLAG, EMPTY_BYTES_FLAG, EMPTY_STR_FLAG, FLOAT_FLAG, NUMBER_BASE, POINTER_FLAG, POINTER_FLAG_1BYTE, POINTER_FLAG_2BYTE, POINTER_FLAG_3BYTE, POINTER_FLAG_4BYTE, STR_FLAG};
use crate::utils::py_helpers::{temporary_tuple_of, ToPyErr};
use crate::utils::wrappers::{is_ascii, py_unicode_data};
use pyo3_ffi::{PyBytes_AsString, PyBytes_Size, PyFloatObject, PyObject, PyObject_CallObject, PyUnicode_AsUTF8AndSize, PyUnicode_GET_LENGTH};
use std::collections::hash_map::Entry;
use std::slice;

#[inline(always)]
pub fn serialize_bytes(
    obj: *mut PyObject,
    buffer: &mut PyBytesBuffer,
) -> Result<(), *mut PyObject> {
    let size = unsafe { PyBytes_Size(obj) };
    let data = raise_mem_error_if_null!(unsafe { PyBytes_AsString(obj) });

    if size == 0 {
        buffer.push(EMPTY_BYTES_FLAG)
    } else {
        buffer.push(BYTES_FLAG)?;
        encode_number::<NUMBER_BASE>(buffer, size as u128)?;
        buffer.extend_from_slice(unsafe { slice::from_raw_parts(data as *const u8, size as usize) })
    }
}

#[inline(always)]
pub fn serialize_float(
    obj: *mut PyObject,
    buffer: &mut PyBytesBuffer,
) -> Result<(), *mut PyObject> {
    let value = unsafe { (*(obj as *mut PyFloatObject)).ob_fval };
    buffer.push(FLOAT_FLAG)?;
    buffer.extend_from_slice(&value.to_be_bytes())
}

#[inline(always)]
pub fn serialize_str(
    obj: *mut PyObject,
    buffer: &mut PyBytesBuffer,
    pointers: &mut Pointers,
) -> Result<(), *mut PyObject> {
    let mut len: isize = 0;
    if is_ascii(obj) {
        let len = unsafe { PyUnicode_GET_LENGTH(obj) } as usize;
        if len == 0 {
            return buffer.push(EMPTY_STR_FLAG);
        }
        if let Some(pointer) = try_get_as_pointer(obj, pointers)? {
            encode_pointer(buffer, pointer)?;
            return Ok(());
        }
        // Skip the PyASCIIObject header
        let data_ptr = py_unicode_data(obj);

        buffer.push(ASCII_STR_FLAG)?;
        encode_number::<NUMBER_BASE>(buffer, len as u128)?;
        return buffer.extend_from_slice(unsafe { slice::from_raw_parts(data_ptr, len) });
    }

    let data = unsafe { PyUnicode_AsUTF8AndSize(obj, &mut len) };

    if len == 0 {
        // not sure if this is possible
        return buffer.push(EMPTY_STR_FLAG);
    }

    if let Some(pointer) = try_get_as_pointer(obj, pointers)? {
        encode_pointer(buffer, pointer)?;
        return Ok(());
    }
    buffer.push(STR_FLAG)?;
    encode_number::<NUMBER_BASE>(buffer, len as u128)?;
    buffer.extend_from_slice(unsafe { slice::from_raw_parts(data as *const u8, len as usize) })
}

fn encode_pointer(buffer: &mut PyBytesBuffer, pointer: u128) -> Result<(), *mut PyObject> {
    if pointer < 2u128.pow(8) {
        buffer.extend_from_slice(&[POINTER_FLAG_1BYTE, pointer as u8])?;
    } else if pointer < 2u128.pow(16) {
        buffer.push(POINTER_FLAG_2BYTE)?;
        buffer.extend_from_slice(&(pointer as u16).to_be_bytes())?;
    } else if pointer < 2u128.pow(24) {
        buffer.extend_from_slice(&[
            POINTER_FLAG_3BYTE,
            (pointer >> 16) as u8,
            (pointer >> 8 & 0b11111111) as u8,
            (pointer & 0b11111111) as u8,
        ])?;
    } else if pointer < 2u128.pow(32) {
        buffer.push(POINTER_FLAG_4BYTE)?;
        buffer.extend_from_slice(&(pointer as u32).to_be_bytes())?;
    } else {
        buffer.push(POINTER_FLAG)?;
        encode_number::<NUMBER_BASE>(buffer, pointer)?;
    }
    Ok(())
}

#[inline(always)]
pub fn try_get_as_pointer(
    str: *mut PyObject,
    pointers: &mut Pointers,
) -> Result<Option<u128>, *mut PyObject> {
    let amount_of_pointers = pointers.len();
    match pointers.entry(PyStringKey::new(str)) {
        Entry::Occupied(entry) => {
            return Ok(Some((*entry.get()) as u128));
        }
        Entry::Vacant(entry) => {
            entry.insert(amount_of_pointers);
        }
    }
    Ok(None)
}

pub fn serialize_date(
    obj: *mut PyObject,
    buffer: &mut PyBytesBuffer,
    pointers: &mut Pointers,
) -> Result<(), *mut PyObject> {
    unsafe {
        let args = temporary_tuple_of(obj)?;
        let py_iso_formatted_date = PyObject_CallObject(ISO_FORMAT_FUNC, args.as_ptr());
        if py_iso_formatted_date.is_null() {
            return Err("Failed to serialize date".to_py_error(SERIALIZATION_ERROR_TYPE));
        }
        serialize_str(py_iso_formatted_date, buffer, pointers)
    }
}
