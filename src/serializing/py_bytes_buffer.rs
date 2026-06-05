use std::{ptr, slice};

use crate::raise_mem_error_if_null;
use crate::utils::safe_py_pointer::PyPointer;
use pyo3_ffi::{PyBytes_AS_STRING, PyBytes_AsString, PyBytes_FromStringAndSize, PyErr_NoMemory, PyObject, PyObject_CallMethod, Py_DecRef, Py_IncRef, Py_None, _PyBytes_Resize};

pub trait PyBytesBuffer {
    fn push(&mut self, byte: u8) -> Result<(), *mut PyObject>;

    fn extend_from_slice(
        &mut self,
        slice: &[u8],
    ) -> Result<(), *mut PyObject>;

    fn extend_from_bytes(
        &mut self,
        size: usize,
        py_bytes: *mut PyObject,
    ) -> Result<(), *mut PyObject>;

    fn finish(self) -> *mut PyObject;
}

pub struct RawBytesBuffer {
    obj: *mut PyObject,
    data_ptr: *mut u8,
    len: usize,
    cap: usize,
}

impl RawBytesBuffer {
    pub fn with_capacity(cap: usize) -> Result<Self, *mut PyObject> {
        let cap = cap.max(8);

        let obj = unsafe {
            PyBytes_FromStringAndSize(ptr::null(), cap as isize)
        };

        if obj.is_null() {
            return Err(unsafe { PyErr_NoMemory() });
        }

        Ok(Self {
            obj,
            data_ptr: unsafe { PyBytes_AS_STRING(obj) as *mut u8 },
            len: 0,
            cap,
        })
    }

    #[inline(always)]
    pub fn ensure_capacity(
        &mut self,
        additional: usize,
    ) -> Result<(), *mut PyObject> {
        let required = self.len + additional;
        if required <= self.cap {
            return Ok(());
        }

        self.cap = required.max(self.cap * 2);
        unsafe {
            if _PyBytes_Resize(&mut self.obj, self.cap as isize) < 0 {
                return Err(PyErr_NoMemory());
            }
            self.data_ptr = PyBytes_AS_STRING(self.obj) as *mut u8;
        }
        Ok(())
    }

    #[inline(always)]
    pub fn push_unchecked(&mut self, byte: u8) {
        unsafe {
            *self.data_ptr.add(self.len) = byte;
        }
        self.len += 1;
    }

    #[inline(always)]
    pub fn extend_from_slice_unchecked(
        &mut self,
        slice: &[u8],
    ) {
        unsafe {
            ptr::copy_nonoverlapping(
                slice.as_ptr(),
                self.data_ptr.add(self.len),
                slice.len(),
            );
        }

        self.len += slice.len();
    }


    pub fn finish(mut self) -> *mut PyObject {
        unsafe { self.shrink_bytes(); }
        self.obj
    }

    unsafe fn shrink_bytes(&mut self) {
        if self.len != self.cap {
            _PyBytes_Resize(&mut self.obj, self.len as isize);
            self.data_ptr = PyBytes_AS_STRING(self.obj) as *mut u8;
        }
    }

    fn take_bytes(&mut self) -> *mut PyObject {
        unsafe { self.shrink_bytes(); }
        self.cap = self.len;
        self.len = 0;
        self.obj
    }
}


pub struct MemoryPyBytesBuffer {
    inner: RawBytesBuffer,
}

impl MemoryPyBytesBuffer {
    pub fn with_capacity(
        cap: usize,
    ) -> Result<Self, *mut PyObject> {
        Ok(Self {
            inner: RawBytesBuffer::with_capacity(cap)?,
        })
    }
}

impl PyBytesBuffer for MemoryPyBytesBuffer {
    #[inline(always)]
    fn push(
        &mut self,
        byte: u8,
    ) -> Result<(), *mut PyObject> {
        if self.inner.len >= self.inner.cap { // attempt to make hot path faster
            self.inner.ensure_capacity(1)?;
        }
        self.inner.push_unchecked(byte);
        Ok(())
    }

    #[inline(always)]
    fn extend_from_slice(
        &mut self,
        slice: &[u8],
    ) -> Result<(), *mut PyObject> {
        self.inner.ensure_capacity(slice.len())?;
        self.inner.extend_from_slice_unchecked(slice);
        Ok(())
    }

    #[inline(always)]
    fn extend_from_bytes(
        &mut self,
        size: usize,
        py_bytes: *mut PyObject,
    ) -> Result<(), *mut PyObject> {
        let slice = unsafe {
            let data = raise_mem_error_if_null!(PyBytes_AsString(py_bytes));
            slice::from_raw_parts(data as *const u8, size)
        };
        self.extend_from_slice(slice)
    }

    fn finish(self) -> *mut PyObject {
        self.inner.finish()
    }
}


pub struct FilePyBytesBuffer {
    inner: RawBytesBuffer,

    writer: *mut PyObject,

    flush_threshold: usize,
    direct_write_threshold: usize,
}

impl FilePyBytesBuffer {
    pub fn with_writer(
        cap: usize,
        writer: *mut PyObject,
        flush_threshold: usize,
        direct_write_threshold: usize,
    ) -> Result<Self, *mut PyObject> {
        unsafe {
            Py_IncRef(writer);
        }

        Ok(Self {
            inner: RawBytesBuffer::with_capacity(cap)?,
            writer,
            flush_threshold,
            direct_write_threshold,
        })
    }

    #[inline(always)]
    fn flush(&mut self) -> Result<(), *mut PyObject> {
        if self.inner.len == 0 {
            return Ok(());
        }

        let py_bytes = self.inner.take_bytes();
        self.write_bytes(py_bytes)
    }

    #[inline(always)]
    fn write_bytes(
        &mut self,
        py_bytes: *mut PyObject,
    ) -> Result<(), *mut PyObject> {
        unsafe {
            let result = PyObject_CallMethod(
                self.writer,
                c"write".as_ptr(),
                c"O".as_ptr(),
                py_bytes,
            );

            if result.is_null() {
                return Err(ptr::null_mut());
            }

            Py_DecRef(result);
        }
        Ok(())
    }

    fn extend_from_small_slice(
        &mut self,
        slice: &[u8],
    ) -> Result<(), *mut PyObject> {
        self.inner.ensure_capacity(slice.len())?;
        self.inner.extend_from_slice_unchecked(slice);

        if self.inner.len >= self.flush_threshold {
            self.flush()?;
        }
        Ok(())
    }

}

impl PyBytesBuffer for FilePyBytesBuffer {
    #[inline(always)]
    fn push(
        &mut self,
        byte: u8,
    ) -> Result<(), *mut PyObject> {
        if self.inner.len >= self.inner.cap { // attempt to make hot path faster
            self.inner.ensure_capacity(1)?;
        }

        self.inner.push_unchecked(byte);
        if self.inner.len >= self.flush_threshold {
            self.flush()?;
        }
        Ok(())
    }

    #[inline(always)]
    fn extend_from_slice(
        &mut self,
        slice: &[u8],
    ) -> Result<(), *mut PyObject> {
        if slice.len() >= self.direct_write_threshold {
            self.flush()?;
            let py_bytes = unsafe {
                PyPointer::new_w_null_check(PyBytes_FromStringAndSize(
                    slice.as_ptr() as *const i8,
                    slice.len() as isize,
                ))?
            };
            self.write_bytes(py_bytes.as_ptr())?;
            return Ok(());
        }

        self.extend_from_small_slice(slice)
    }

    #[inline(always)]
    fn extend_from_bytes(
        &mut self,
        size: usize,
        py_bytes: *mut PyObject,
    ) -> Result<(), *mut PyObject> {
        if size >= self.direct_write_threshold {
            self.flush()?;
            self.write_bytes(py_bytes)?;
            return Ok(());
        }

        let slice = unsafe {
            let data = raise_mem_error_if_null!(PyBytes_AsString(py_bytes));
            slice::from_raw_parts(data as *const u8, size as usize)
        };

        self.extend_from_small_slice(slice)
    }


    fn finish(mut self) -> *mut PyObject {
        let flush_result = self.flush();
        if flush_result.is_err() {
            return ptr::null_mut();
        }

        unsafe {
            Py_IncRef(Py_None());
            Py_None()
        }
    }
}

impl Drop for FilePyBytesBuffer {
    fn drop(&mut self) {
        unsafe {
            Py_DecRef(self.writer);
        }
    }
}