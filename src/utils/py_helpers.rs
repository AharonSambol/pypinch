use std::{ptr, slice};
use std::ffi::CString;
use pyo3_ffi::{Py_DECREF, Py_ssize_t, PyByteArray_AsString, PyByteArray_Size, PyByteArray_Type, PyBytes_AsString, PyBytes_Size, PyErr_SetString, PyImport_Import, PyObject, PyObject_GetAttrString, PyUnicode_AsUTF8AndSize, PyUnicode_CompareWithASCIIString, PyUnicode_FromString};

use crate::{py_string_format, raise_mem_error_if_null};


#[inline(always)]
pub unsafe fn compare_str(py_str: *mut PyObject, rust_str: &[u8]) -> bool {
    PyUnicode_CompareWithASCIIString(
        py_str,
        rust_str.as_ptr() as *const _,
    ) == 0
}

pub unsafe fn py_str_to_rust_str(py_str: &*mut PyObject) -> Result<&str, *mut PyObject> {
    let mut size: Py_ssize_t = 0;
    let c_ptr = raise_mem_error_if_null!(PyUnicode_AsUTF8AndSize(*py_str, &mut size));
    Ok(str::from_utf8_unchecked(slice::from_raw_parts(c_ptr as *const u8, size as usize)))
}

pub unsafe fn convert_py_buffer_into_bytes_slice(buffer: &*mut PyObject) -> Result<&[u8], *mut PyObject> {
    let buffer = *buffer;
    if (*buffer).ob_type == &mut PyByteArray_Type {
        let len = PyByteArray_Size(buffer) as usize;
        let data_ptr = raise_mem_error_if_null!(PyByteArray_AsString(buffer)) as *const u8;
        Ok(slice::from_raw_parts(data_ptr, len))
    } else {
        let len = PyBytes_Size(buffer) as usize;
        let data_ptr = raise_mem_error_if_null!(PyBytes_AsString(buffer)) as *const u8;
        Ok(slice::from_raw_parts(data_ptr, len))
    }
}

pub unsafe fn import_object_from_python(module_name: &str, object_name: &str) -> *mut PyObject {
    let module_name = CString::new(module_name).unwrap();
    let class_name = CString::new(object_name).unwrap();
    let py_mod_path = PyUnicode_FromString(module_name.as_ptr());
    let module = PyImport_Import(py_mod_path);

    Py_DECREF(py_mod_path);
    let object = PyObject_GetAttrString(module, class_name.as_ptr());

    Py_DECREF(module);
    object
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