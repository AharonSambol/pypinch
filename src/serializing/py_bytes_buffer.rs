use std::ptr;

use pyo3_ffi::{_PyBytes_Resize, PyBytes_AS_STRING, PyBytes_FromStringAndSize, PyErr_NoMemory, PyObject};
use crate::raise_mem_error_if_null;

pub struct PyBytesBuffer {
    obj: *mut PyObject,
    len: usize,
    cap: usize,
}

impl PyBytesBuffer {
    pub unsafe fn with_capacity(cap: usize) -> Result<Self, *mut PyObject> {
        let obj = raise_mem_error_if_null!(PyBytes_FromStringAndSize(ptr::null(), cap as isize));

        Ok(Self {
            obj,
            len: 0,
            cap: if cap <= 0 { 8 } else { cap },
        })
    }

    #[inline(always)]
    unsafe fn data_ptr(&self) -> *mut u8 {
        PyBytes_AS_STRING(self.obj) as *mut u8
    }

    #[inline]
    unsafe fn ensure_capacity(&mut self, additional: usize) -> bool {
        let required = self.len + additional;
        if required <= self.cap {
            return true;
        }

        self.cap = required.max(self.cap * 2);
        _PyBytes_Resize(&mut self.obj, self.cap as isize) >= 0
    }

    #[inline]
    pub unsafe fn push(&mut self, byte: u8) -> Result<(), *mut PyObject> {
        if !self.ensure_capacity(1) {
            return Err(PyErr_NoMemory());
        }

        *self.data_ptr().add(self.len) = byte;
        self.len += 1;
        Ok(())
    }

    #[inline]
    pub unsafe fn extend_from_slice(&mut self, slice: &[u8]) -> Result<(), *mut PyObject> {
        if !self.ensure_capacity(slice.len()) {
            return Err(PyErr_NoMemory());
        }

        ptr::copy_nonoverlapping(
            slice.as_ptr(),
            self.data_ptr().add(self.len),
            slice.len(),
        );

        self.len += slice.len();
        Ok(())
    }

    pub unsafe fn finish(mut self) -> *mut PyObject {
        if self.len != self.cap {
            _PyBytes_Resize(&mut self.obj, self.len as isize);
        }

        self.obj
    }
}