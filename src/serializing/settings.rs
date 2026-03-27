use crate::utils::safe_py_pointer::PyPointer;
use pyo3_ffi::PyTypeObject;
use std::collections::HashMap;

pub struct Settings {
    pub serialize_dates: bool,
    pub custom_types: Option<HashMap<*mut PyTypeObject, CustomType>>,
}

pub struct CustomType {
    pub identifier: PyPointer,
    pub converter: PyPointer,
}