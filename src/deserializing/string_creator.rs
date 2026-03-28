use crate::raise_mem_error_if_null;
use pyo3_ffi::{PyObject, PyUnicode_FromStringAndSize, PyUnicode_New, Py_ssize_t};
use std::ffi::c_char;

pub fn create_string<const IS_ASCII: bool>(
    buf_slice: &[u8],
) -> Result<*mut PyObject, *mut PyObject> {
    let py_str = unsafe {
        #[cfg(PyPy)]
        {
            raise_mem_error_if_null!(PyUnicode_FromStringAndSize(
                buf_slice.as_ptr() as *const c_char,
                buf_slice.len() as isize,
            ))
        }

        #[cfg(not(PyPy))]
        {
            if IS_ASCII {
                let py_str =
                    raise_mem_error_if_null!(PyUnicode_New(buf_slice.len() as Py_ssize_t, 127));

                let dest_ptr = pyo3_ffi::PyUnicode_DATA(py_str) as *mut u8;

                std::ptr::copy_nonoverlapping(buf_slice.as_ptr(), dest_ptr, buf_slice.len());
                py_str
            } else {
                raise_mem_error_if_null!(PyUnicode_FromStringAndSize(
                    buf_slice.as_ptr() as *const c_char,
                    buf_slice.len() as isize,
                ))
            }
        }
    };

    Ok(py_str)
}
