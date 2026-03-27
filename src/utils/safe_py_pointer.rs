use pyo3_ffi::*;

pub struct PyPointer {
    ptr: *mut PyObject,
}

impl PyPointer {
    pub fn new(ptr: *mut PyObject) -> Self {
        Self { ptr }
    }

    pub fn new_w_null_check(ptr: *mut PyObject) -> Result<Self, *mut PyObject> {
        if ptr.is_null() {
            return Err(unsafe { PyErr_NoMemory() });
        }
        Ok(Self { ptr })
    }

    pub fn as_ptr(&self) -> *mut PyObject {
        self.ptr
    }
}

impl Drop for PyPointer {
    fn drop(&mut self) {
        unsafe {
            Py_DECREF(self.ptr);
        }
    }
}