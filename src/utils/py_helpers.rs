use crate::utils::safe_py_pointer::PyPointer;
use crate::utils::wrappers::tuple_set_item;
use crate::{py_string_format, raise_mem_error_if_null};
use pyo3_ffi::{PyByteArray_AsString, PyByteArray_Size, PyByteArray_Type, PyBytes_AsString, PyBytes_Size, PyErr_SetString, PyImport_Import, PyObject, PyObject_GetAttrString, PyObject_Repr, PyObject_Str, PyObject_Type, PyTuple_New, PyUnicode_AsUTF8, PyUnicode_AsUTF8AndSize, PyUnicode_CompareWithASCIIString, PyUnicode_FromString, Py_INCREF, Py_ssize_t};
use std::ffi::{CStr, CString};
use std::{ptr, slice};

#[inline(always)]
pub fn compare_str(py_str: *mut PyObject, rust_str: &[u8]) -> bool {
    unsafe { PyUnicode_CompareWithASCIIString(py_str, rust_str.as_ptr() as *const _) == 0 }
}

pub fn py_str_to_rust_str(py_str: &*mut PyObject) -> Result<&str, *mut PyObject> {
    let mut size: Py_ssize_t = 0;
    unsafe {
        let c_ptr = raise_mem_error_if_null!(PyUnicode_AsUTF8AndSize(*py_str, &mut size));
        Ok(str::from_utf8_unchecked(slice::from_raw_parts(
            c_ptr as *const u8,
            size as usize,
        )))
    }
}

pub fn convert_py_buffer_into_bytes_slice(buffer: &*mut PyObject) -> Result<&[u8], *mut PyObject> {
    let buffer = *buffer;
    unsafe {
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
}

pub fn import_object_from_python(module_name: &str, object_name: &str) -> *mut PyObject {
    let module_name = if let Ok(x) = CString::new(module_name) { x } else {
        return ptr::null_mut()
    };
    let class_name = if let Ok(x) = CString::new(object_name) { x } else {
        return ptr::null_mut()
    };
    unsafe {
        let py_mod_path = match PyPointer::new_w_null_check(PyUnicode_FromString(module_name.as_ptr())) {
            Ok(py_mod_path) => py_mod_path,
            Err(_) => return ptr::null_mut(),
        };

        let module = match PyPointer::new_w_null_check(PyImport_Import(py_mod_path.as_ptr())) {
            Ok(module) => module,
            Err(_) => return ptr::null_mut(),
        };
        PyObject_GetAttrString(module.as_ptr(), class_name.as_ptr())
    }
}

pub fn pretty_type(object: *mut PyObject) -> String {
    unsafe {
        let type_ptr = PyPointer::new(PyObject_Type(object));
        if type_ptr.as_ptr().is_null() {
            return "Error".to_string();
        }

        let repr_ptr = PyPointer::new(PyObject_Repr(type_ptr.as_ptr()));

        if repr_ptr.as_ptr().is_null() {
            return "Error".to_string();
        }

        let c_ptr = PyUnicode_AsUTF8(repr_ptr.as_ptr());
        CStr::from_ptr(c_ptr).to_string_lossy().into_owned()
    }
}

pub fn to_py_str(object: *mut PyObject) -> Result<PyPointer, *mut PyObject> {
    unsafe {
        PyPointer::new_w_null_check(PyObject_Str(object))
    }
}

pub fn temporary_tuple_of(object: *mut PyObject) -> Result<PyPointer, *mut PyObject> {
    unsafe {
        let tuple = PyPointer::new_w_null_check(PyTuple_New(1))?;
        Py_INCREF(object);
        tuple_set_item(tuple.as_ptr(), 0, object);
        Ok(tuple)
    }
}

pub trait ToPyErr<T> {
    fn to_py_error(&self, typ: *mut PyObject) -> *mut PyObject;
}
impl ToPyErr<String> for String {
    fn to_py_error(&self, typ: *mut PyObject) -> *mut PyObject {
        unsafe {
            PyErr_SetString(typ, py_string_format!(self));
        }
        ptr::null_mut()
    }
}

impl ToPyErr<&str> for &str {
    fn to_py_error(&self, typ: *mut PyObject) -> *mut PyObject {
        self.to_string().to_py_error(typ)
    }
}
