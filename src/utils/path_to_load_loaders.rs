use crate::deserializing::lazy_deserialize::PathPart;
use crate::serializing::utils::IDX_CLASS;
use crate::utils::py_helpers::ToPyErr;
use crate::utils::safe_py_pointer::PyPointer;
use crate::utils::wrappers::{get_list_size, list_get_item};
use pyo3_ffi::{PyExc_TypeError, PyList_Check, PyLong_AsUnsignedLongLong, PyLong_FromLong, PyObject, PyObject_GetAttr, PyObject_RichCompareBool, PyTypeObject, PyUnicode_FromString};

pub unsafe fn parse_path_to_load(
    obj: *mut PyObject,
) -> Result<Vec<PathPart>, *mut PyObject> {
    if PyList_Check(obj) == 0 {
        return Err("path_to_load must be a list".to_py_error(PyExc_TypeError));
    }

    let index_name =
        PyPointer::new_w_null_check(PyUnicode_FromString(b"index\0".as_ptr() as _))?;

    let list_len = get_list_size(obj);
    let mut path_to_load = Vec::with_capacity(list_len as usize);
    for i in 0..list_len {
        let item = list_get_item(obj, i);

        if (*item).ob_type == IDX_CLASS as *mut PyTypeObject {
            let index =
                PyPointer::new_w_null_check(PyObject_GetAttr(item, index_name.as_ptr()))?;
            let is_negative = PyObject_RichCompareBool(
                index.as_ptr(),
                PyPointer::new_w_null_check(PyLong_FromLong(0))?.as_ptr(),
                pyo3_ffi::Py_LT,
            ) == 1;
            if is_negative {
                return Err("index must not be negative".to_py_error(PyExc_TypeError));
            }
            let rust_index = unsafe { PyLong_AsUnsignedLongLong(index.as_ptr()) } as usize;
            path_to_load.push(PathPart::Index(rust_index));
        } else {
            path_to_load.push(PathPart::Key(item));
        }
    }
    Ok(path_to_load)
}
