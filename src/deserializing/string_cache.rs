use std::ffi::c_char;
use pyo3_ffi::{Py_INCREF, PyObject, PyUnicode_FromStringAndSize};
use rustc_hash::FxHashMap;

pub struct StringCache<'a> {
    cache: FxHashMap<&'a [u8], *mut PyObject>,
}

impl<'a> StringCache<'a> {
    pub fn new() -> Self {
        Self { cache: FxHashMap::default() }
    }

    pub unsafe fn get_or_create(&mut self, buf_slice: &'a [u8]) -> *mut PyObject {
        if let Some(&py_str) = self.cache.get(buf_slice) {
            Py_INCREF(py_str);
            return py_str;
        }

        let py_str = PyUnicode_FromStringAndSize(
            buf_slice.as_ptr() as *const c_char,
            buf_slice.len() as isize,
        );

        self.cache.insert(buf_slice, py_str);
        py_str
    }
}