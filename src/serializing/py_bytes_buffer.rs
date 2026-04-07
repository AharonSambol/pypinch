use std::ptr;

use crate::raise_mem_error_if_null;
use pyo3_ffi::{
    PyBytes_AS_STRING, PyBytes_FromStringAndSize, PyErr_NoMemory, PyObject, _PyBytes_Resize,
};

pub struct PyBytesBuffer {
    obj: *mut PyObject,
    data_ptr: *mut u8,
    len: usize,
    cap: usize,
}

impl PyBytesBuffer {
    pub fn with_capacity(cap: usize) -> Result<Self, *mut PyObject> {
        let obj = raise_mem_error_if_null!(unsafe {
            PyBytes_FromStringAndSize(ptr::null(), cap as isize)
        });

        Ok(Self {
            obj,
            data_ptr: unsafe { PyBytes_AS_STRING(obj) as *mut u8 },
            len: 0,
            cap: if cap <= 0 { 8 } else { cap },
        })
    }

    #[inline]
    fn ensure_capacity(&mut self, additional: usize) -> bool {
        let required = self.len + additional;
        if required <= self.cap {
            return true;
        }

        self.cap = required.max(self.cap * 2);
        unsafe {
            let succeeded = _PyBytes_Resize(&mut self.obj, self.cap as isize) >= 0;
            self.data_ptr = PyBytes_AS_STRING(self.obj) as *mut u8;
            succeeded
        }
    }

    #[inline]
    pub fn push(&mut self, byte: u8) -> Result<(), *mut PyObject> {
        if self.len < self.cap {
            if !self.ensure_capacity(1) {
                return Err(unsafe { PyErr_NoMemory() });
            }
        }

        unsafe {
            *self.data_ptr.add(self.len) = byte;
        }
        self.len += 1;
        Ok(())
    }

    #[inline]
    pub fn extend_from_slice(&mut self, slice: &[u8]) -> Result<(), *mut PyObject> {
        if !self.ensure_capacity(slice.len()) {
            return Err(unsafe { PyErr_NoMemory() });
        }

        unsafe {
            ptr::copy_nonoverlapping(slice.as_ptr(), self.data_ptr.add(self.len), slice.len());
        }

        self.len += slice.len();
        Ok(())
    }

    pub fn finish(mut self) -> *mut PyObject {
        if self.len != self.cap {
            unsafe {
                _PyBytes_Resize(&mut self.obj, self.len as isize);
            }
        }

        self.obj
    }
}
