use pyo3_ffi::{PyASCIIObject, PyObject, PyObject_Hash, PyUnicode_Compare};
use rustc_hash::FxHashMap;
use std::hash::{Hash, Hasher};

#[derive(Copy, Clone)]
pub struct PyStringKey(*mut PyObject);

impl PyStringKey {
    pub fn new(obj: *mut PyObject) -> PyStringKey {
        // compute the hash once, so that it will be saved in python's cache
        unsafe { PyObject_Hash(obj) };
        PyStringKey(obj)
    }
}
impl Hash for PyStringKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        unsafe {
            // the hash was already computed in the constructor so it will be saved here
            let hash = (*(self.0 as *mut PyASCIIObject)).hash;
            state.write_isize(hash);
        }
    }
}

impl PartialEq for PyStringKey {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0 || unsafe { PyUnicode_Compare(self.0, other.0) == 0 }
    }
}

impl Eq for PyStringKey {}

pub type Pointers = FxHashMap<PyStringKey, usize>;
