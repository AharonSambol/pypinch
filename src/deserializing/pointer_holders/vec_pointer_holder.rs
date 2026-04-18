use crate::deserializing::pointer_holders::pointer_holder::PointerHolder;
use crate::safe_get;
use crate::utils::consts::CORRUPTED_DATA;
use pyo3_ffi::{PyObject, Py_DECREF, Py_INCREF};

pub struct VecPointerHolder {
    vec: Vec<*mut PyObject>,
}

impl PointerHolder for VecPointerHolder {
    fn safe_get(&self, position: usize) -> Result<*mut PyObject, *mut PyObject> {
        let str = *safe_get!(self.vec, position, CORRUPTED_DATA);
        unsafe { Py_INCREF(str); }
        Ok(str)
    }

    fn insert(&mut self, object: *mut PyObject) {
        unsafe { Py_INCREF(object); }
        self.vec.push(object);
    }
}

impl VecPointerHolder {
    pub fn new() -> VecPointerHolder {
        VecPointerHolder { vec: Vec::new() }
    }
}

impl Drop for VecPointerHolder {
    fn drop(&mut self) {
        for i in self.vec.iter() {
            unsafe {
                Py_DECREF(*i);
            }
        }
    }
}