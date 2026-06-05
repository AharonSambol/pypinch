use crate::serializing::py_bytes_buffer::PyBytesBuffer;
use crate::serializing::serialize;
use crate::serializing::serializing_string_cache::Pointers;
use crate::serializing::settings::{CustomType, Settings};
use crate::serializing::utils::SERIALIZATION_ERROR_TYPE;
use crate::utils::consts::CUSTOM_TYPE_FLAG;
use crate::utils::py_helpers::{temporary_tuple_of, ToPyErr};
use pyo3_ffi::{PyObject, PyObject_CallObject};

pub fn serialize_custom_type<Buffer: PyBytesBuffer>(
    obj: *mut PyObject,
    buffer: &mut Buffer,
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

    let args = temporary_tuple_of(obj)?;
    let converted_object = unsafe { PyObject_CallObject(custom_type.converter.as_ptr(), args.as_ptr()) };
    if converted_object.is_null() {
        return Err(
            "Failed to serialize custom type".to_py_error(unsafe { SERIALIZATION_ERROR_TYPE })
        );
    }
    serialize::serialize(converted_object, buffer, pointers, settings)
}