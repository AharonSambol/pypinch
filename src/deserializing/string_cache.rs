use std::ffi::c_char;
use pyo3_ffi::{Py_INCREF, Py_ssize_t, PyObject, PyUnicode_DATA, PyUnicode_FromStringAndSize, PyUnicode_New};
use rustc_hash::FxHashMap;
use crate::raise_mem_error_if_null;

pub struct StringCache<'a> {
    cache: FxHashMap<&'a [u8], *mut PyObject>,
}

impl<'a> StringCache<'a> {
    pub fn new() -> Self {
        Self { cache: FxHashMap::default() }
    }

    pub unsafe fn get_or_create<const IS_ASCII: bool>(&mut self, buf_slice: &'a [u8]) -> Result<*mut PyObject, *mut PyObject> {
        if let Some(&py_str) = self.cache.get(buf_slice) {
            Py_INCREF(py_str);
            return Ok(py_str);
        }

        let py_str = if IS_ASCII {
            let py_str = raise_mem_error_if_null!(PyUnicode_New(buf_slice.len() as Py_ssize_t, 127));

            let dest_ptr = PyUnicode_DATA(py_str) as *mut u8;

            std::ptr::copy_nonoverlapping(buf_slice.as_ptr(), dest_ptr, buf_slice.len());
            py_str
        } else {
            raise_mem_error_if_null!(PyUnicode_FromStringAndSize(
                buf_slice.as_ptr() as *const c_char,
                buf_slice.len() as isize,
            ))
        };

        self.cache.insert(buf_slice, py_str);
        Ok(py_str)
    }
}