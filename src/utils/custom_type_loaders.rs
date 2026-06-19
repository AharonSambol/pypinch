use crate::serializing::settings::CustomType;
use crate::serializing::utils::CUSTOM_TYPE_CLASS;
use crate::utils::py_dict_key::{PyHashMap, PyKey};
use crate::utils::py_helpers::ToPyErr;
use crate::utils::safe_py_pointer::PyPointer;
use pyo3_ffi::{
    PyCallable_Check, PyDict_Check, PyDict_Next, PyExc_TypeError, PyObject, PyObject_GetAttr,
    PyTypeObject, PyType_Check, PyUnicode_FromString, Py_IsTrue,
};
use std::collections::HashMap;
use std::ptr;

pub unsafe fn parse_dumps_custom_types_dict(
    obj: *mut PyObject,
) -> Result<HashMap<*mut PyTypeObject, CustomType>, *mut PyObject> {
    if PyDict_Check(obj) == 0 {
        return Err("custom_types must be a dict".to_py_error(PyExc_TypeError));
    }
    let identifier_name =
        PyPointer::new_w_null_check(PyUnicode_FromString(b"identifier\0".as_ptr() as _))?;
    let converter_name =
        PyPointer::new_w_null_check(PyUnicode_FromString(b"converter\0".as_ptr() as _))?;
    let include_subclasses_name =
        PyPointer::new_w_null_check(PyUnicode_FromString(b"include_subclasses\0".as_ptr() as _))?;
    let one_way_name =
        PyPointer::new_w_null_check(PyUnicode_FromString(b"one_way\0".as_ptr() as _))?;

    let mut custom_types_dict = HashMap::new();
    let mut pos = 0;
    let mut key: *mut PyObject = ptr::null_mut();
    let mut value: *mut PyObject = ptr::null_mut();
    while PyDict_Next(obj, &mut pos, &mut key, &mut value) != 0 {
        if PyType_Check(key) == 0 {
            return Err("custom_types key must be a valid type".to_py_error(PyExc_TypeError));
        }
        if (*value).ob_type != CUSTOM_TYPE_CLASS as *mut PyTypeObject {
            return Err(
                "custom_types value must be of type CustomType".to_py_error(PyExc_TypeError)
            );
        }

        let identifier =
            PyPointer::new_w_null_check(PyObject_GetAttr(value, identifier_name.as_ptr()))?;
        let converter =
            PyPointer::new_w_null_check(PyObject_GetAttr(value, converter_name.as_ptr()))?;
        let include_subclasses_pointer = PyPointer::new_w_null_check(
            PyObject_GetAttr(value, include_subclasses_name.as_ptr())
        )?;
        let one_way_pointer = PyPointer::new_w_null_check(
            PyObject_GetAttr(value, one_way_name.as_ptr())
        )?;
        let include_subclasses = Py_IsTrue(include_subclasses_pointer.as_ptr()) != 0;
        let one_way = Py_IsTrue(one_way_pointer.as_ptr()) != 0;

        custom_types_dict.insert(
            key as *mut PyTypeObject,
            CustomType {
                identifier,
                converter,
                include_subclasses,
                one_way,
            },
        );
    }
    Ok(custom_types_dict)
}

pub unsafe fn parse_loads_custom_types_dict(
    obj: *mut PyObject,
) -> Result<PyHashMap<*mut PyObject>, *mut PyObject> {
    if PyDict_Check(obj) == 0 {
        return Err("custom_types must be a dict".to_py_error(PyExc_TypeError));
    }

    let mut custom_types_dict = PyHashMap::<*mut PyObject>::default();
    let mut pos = 0;
    let mut key: *mut PyObject = ptr::null_mut();
    let mut value: *mut PyObject = ptr::null_mut();
    while PyDict_Next(obj, &mut pos, &mut key, &mut value) != 0 {
        if PyCallable_Check(value) == 0 {
            return Err("custom_types value must be a function".to_py_error(PyExc_TypeError));
        }
        custom_types_dict.insert(PyKey(key), value);
    }
    Ok(custom_types_dict)
}
