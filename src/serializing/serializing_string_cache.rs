use std::hash::{Hash, Hasher};
use pyo3_ffi::{PyObject, PyObject_Hash, PyUnicode_Compare};
use rustc_hash::FxHashMap;

#[derive(Copy, Clone)]
pub struct PyStringKey(pub *mut PyObject);

impl Hash for PyStringKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        unsafe {
            let hash = PyObject_Hash(self.0);
            state.write_isize(hash);
        }
    }
}

impl PartialEq for PyStringKey {
    fn eq(&self, other: &Self) -> bool {
        unsafe {
            PyUnicode_Compare(self.0, other.0) == 0
        }
    }
}

impl Eq for PyStringKey {}

pub type Pointers = FxHashMap<PyStringKey, usize>;