use pyo3_ffi as ffi;
use std::ptr;

pub struct PyBytesBuffer {
    obj: *mut ffi::PyObject,
    len: usize,
    cap: usize,
}

impl PyBytesBuffer {
    pub unsafe fn with_capacity(cap: usize) -> Self {
        let obj = ffi::PyBytes_FromStringAndSize(ptr::null(), cap as isize);

        Self {
            obj,
            len: 0,
            cap: if cap <= 0 { 8 } else { cap },
        }
    }

    #[inline(always)]
    unsafe fn data_ptr(&self) -> *mut u8 {
        ffi::PyBytes_AS_STRING(self.obj) as *mut u8
    }

    #[inline]
    unsafe fn ensure_capacity(&mut self, additional: usize) -> bool {
        let required = self.len + additional;
        if required <= self.cap {
            return true;
        }

        self.cap = required.max(self.cap * 2);
        if ffi::_PyBytes_Resize(&mut self.obj, self.cap as isize) < 0 {
            return false;
        }

        true
    }

    #[inline]
    pub unsafe fn push(&mut self, byte: u8) {
        if !self.ensure_capacity(1) {
            // TODO
            todo!()
        }

        *self.data_ptr().add(self.len) = byte;
        self.len += 1;
    }

    #[inline]
    pub unsafe fn extend_from_slice(&mut self, slice: &[u8]) {
        if !self.ensure_capacity(slice.len()) {
            // TODO
            todo!()
        }

        ptr::copy_nonoverlapping(
            slice.as_ptr(),
            self.data_ptr().add(self.len),
            slice.len(),
        );

        self.len += slice.len();
    }

    pub unsafe fn finish(mut self) -> *mut ffi::PyObject {
        if self.len != self.cap {
            ffi::_PyBytes_Resize(&mut self.obj, self.len as isize);
        }

        self.obj
    }
}