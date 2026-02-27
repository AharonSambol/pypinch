use std::{ptr, slice};

use pyo3_ffi::{Py_ssize_t, PyByteArray_AsString, PyByteArray_Size, PyByteArray_Type, PyBytes_AsString, PyBytes_Size, PyErr_SetString, PyObject, PyUnicode_AsUTF8AndSize, PyUnicode_CompareWithASCIIString};

use crate::py_string_format;

pub unsafe fn compare_str(py_str: *mut PyObject, rust_str: &[u8]) -> bool {
    PyUnicode_CompareWithASCIIString(
        py_str,
        rust_str.as_ptr() as *const _,
    ) == 0
}

pub unsafe fn py_str_to_rust_str(py_str: &*mut PyObject) -> &str {
    let mut size: Py_ssize_t = 0;
    let c_ptr = PyUnicode_AsUTF8AndSize(*py_str, &mut size);
    str::from_utf8_unchecked(slice::from_raw_parts(c_ptr as *const u8, size as usize))
}

pub unsafe fn convert_py_buffer_into_bytes_slice(buffer: &*mut PyObject) -> &[u8] {
    let buffer = *buffer;
    if (*buffer).ob_type == &mut PyByteArray_Type {
        let len = PyByteArray_Size(buffer) as usize;
        let data_ptr = PyByteArray_AsString(buffer) as *const u8;
        slice::from_raw_parts(data_ptr, len)
    } else {
        let len = PyBytes_Size(buffer) as usize;
        let data_ptr = PyBytes_AsString(buffer) as *const u8;
        slice::from_raw_parts(data_ptr, len)
    }
}

pub trait ToPyErr<T> {
    unsafe fn to_py_error(&self, typ: *mut PyObject) -> *mut PyObject;
}
impl ToPyErr<String> for String {
    unsafe fn to_py_error(&self, typ: *mut PyObject) -> *mut PyObject {
        PyErr_SetString(typ, py_string_format!(self));
        ptr::null_mut()
    }
}

impl ToPyErr<&str> for &str {
    unsafe fn to_py_error(&self, typ: *mut PyObject) -> *mut PyObject {
        self.to_string().to_py_error(typ)
    }
}