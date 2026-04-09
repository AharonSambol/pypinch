use pyo3_ffi::PyObject;

pub trait PointerHolder {
    fn safe_get(&self, position: usize) -> Result<*mut PyObject, *mut PyObject>;
    fn insert(&mut self, object: *mut PyObject);
}