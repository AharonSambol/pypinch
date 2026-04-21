use crate::serializing::py_bytes_buffer::PyBytesBuffer;
use crate::serializing::serialize;
use crate::serializing::serializing_string_cache::Pointers;
use crate::serializing::settings::{CustomType, Settings};
use crate::serializing::utils::SERIALIZATION_ERROR_TYPE;
use crate::utils::consts::CUSTOM_TYPE_FLAG;
use crate::utils::py_helpers::ToPyErr;
use crate::utils::wrappers::tuple_set_item;
use pyo3_ffi::{PyObject, PyObject_CallObject, PyTuple_New, Py_INCREF};

pub fn serialize_custom_type(
    obj: *mut PyObject,
    buffer: &mut PyBytesBuffer,
    pointers: &mut Pointers,
    settings: &Settings,
    custom_type: &CustomType,
) -> Result<(), *mut PyObject> {
    buffer.push(CUSTOM_TYPE_FLAG)?;
    serialize::serialize(
        custom_type.identifier.as_ptr(),
        buffer,
        pointers,
        settings,
    )?;

    let args = unsafe { PyTuple_New(1) };
    unsafe { Py_INCREF(obj); }
    tuple_set_item(args, 0, obj);
    let converted_object = unsafe { PyObject_CallObject(custom_type.converter.as_ptr(), args) };
    if converted_object.is_null() {
        return Err(
            "Failed to serialize custom type".to_py_error(unsafe { SERIALIZATION_ERROR_TYPE })
        );
    }
    serialize::serialize(converted_object, buffer, pointers, settings)
}