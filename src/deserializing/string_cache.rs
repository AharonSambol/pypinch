use std::ffi::c_char;
use pyo3_ffi::{Py_INCREF, Py_ssize_t, PyObject, PyUnicode_DATA, PyUnicode_FromStringAndSize, PyUnicode_New};
use rustc_hash::FxHashMap;

pub struct StringCache<'a> {
    cache: FxHashMap<&'a [u8], *mut PyObject>,
}

impl<'a> StringCache<'a> {
    pub fn new() -> Self {
        Self { cache: FxHashMap::default() }
    }

    pub unsafe fn get_or_create<const IS_ASCII: bool>(&mut self, buf_slice: &'a [u8]) -> *mut PyObject {
        if let Some(&py_str) = self.cache.get(buf_slice) {
            Py_INCREF(py_str);
            return py_str;
        }

        let py_str = if IS_ASCII {
            let py_str = PyUnicode_New(buf_slice.len() as Py_ssize_t, 127);

            // if py_string.is_null() {
            //     return std::ptr::null_mut();
            // }

            let dest_ptr = PyUnicode_DATA(py_str) as *mut u8;

            std::ptr::copy_nonoverlapping(buf_slice.as_ptr(), dest_ptr, buf_slice.len());
            py_str
        } else {
            PyUnicode_FromStringAndSize(
                buf_slice.as_ptr() as *const c_char,
                buf_slice.len() as isize,
            )
        };

        self.cache.insert(buf_slice, py_str);
        py_str
    }
}