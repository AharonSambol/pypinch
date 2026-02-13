use std::collections::HashMap;
use std::ffi::c_char;
use pyo3_ffi::{Py_INCREF, PyObject, PyUnicode_FromStringAndSize};

pub struct StringCache {
    cache: HashMap<Vec<u8>, *mut PyObject>,
}

impl StringCache {
    pub fn new() -> Self {
        Self { cache: HashMap::new() }
    }

    pub unsafe fn get_or_create(&mut self, buf_slice: &[u8]) -> *mut PyObject {
        if let Some(&py_str) = self.cache.get(buf_slice) {
            Py_INCREF(py_str);
            return py_str;
        }

        let py_str = PyUnicode_FromStringAndSize(
            buf_slice.as_ptr() as *const c_char,
            buf_slice.len() as isize,
        );

        self.cache.insert(buf_slice.to_vec(), py_str);
        py_str
    }
}