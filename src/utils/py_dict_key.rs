use pyo3_ffi::{PyObject, PyObject_Hash, PyObject_RichCompareBool, Py_EQ};
use rustc_hash::FxHashMap;
use std::hash::{Hash, Hasher};

#[derive(Copy, Clone)]
pub struct PyKey(pub *mut PyObject);

impl Hash for PyKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        unsafe {
            let hash = PyObject_Hash(self.0);
            state.write_isize(hash);
        }
    }
}

impl PartialEq for PyKey {
    fn eq(&self, other: &Self) -> bool {
        unsafe { PyObject_RichCompareBool(self.0, other.0, Py_EQ) == 1 }
    }
}

impl Eq for PyKey {}

pub type PyHashMap<T> = FxHashMap<PyKey, T>;
