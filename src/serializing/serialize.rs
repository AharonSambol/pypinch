use crate::serializing::number_encoding::encode_python_int;
use crate::serializing::py_bytes_buffer::PyBytesBuffer;
use crate::serializing::serializing_string_cache::Pointers;
use crate::serializing::settings::{CustomType, Settings};
use crate::serializing::utils::SERIALIZATION_ERROR_TYPE;
use crate::serializing::{compound_types, custom_types, primitives};
use crate::utils::consts::{FALSE_FLAG, NULL_FLAG, NUMBER_BASE};
use crate::utils::py_helpers::{pretty_type, ToPyErr};
use pyo3_ffi::*;
use pyo3_ffi::{
    PyBool_Type, PyBytes_Type, PyDict_Type, PyFloat_Type, PyList_Type, PyLong_Type, PyObject,
    PyTuple_Type, PyUnicode_Type,
};
// todo: all_str_keys=False - if true store at the start a flag and then store all dicts without key types
#[inline(always)]
pub fn serialize(
    obj: *mut PyObject,
    buffer: &mut PyBytesBuffer,
    pointers: &mut Pointers,
    settings: &Settings,
) -> Result<(), *mut PyObject> {
    unsafe {
        let typ = (*obj).ob_type;

        if typ == &mut PyUnicode_Type {
            primitives::serialize_str(obj, buffer, pointers)
        } else if typ == &mut PyBool_Type {
            buffer.push(FALSE_FLAG - (obj == Py_True()) as u8)
        } else if typ == &mut PyLong_Type {
            encode_python_int::<NUMBER_BASE>(obj, buffer)
        } else if typ == &mut PyList_Type || typ == &mut PyTuple_Type {
            compound_types::encode_list(obj, buffer, pointers, typ, settings)
        } else if typ == &mut PyDict_Type {
            compound_types::serialize_dict(obj, buffer, pointers, settings)
        } else if typ == &mut PyFloat_Type {
            primitives::serialize_float(obj, buffer)
        } else if typ == &mut PyBytes_Type {
            primitives::serialize_bytes(obj, buffer)
        } else if obj == Py_None() {
            buffer.push(NULL_FLAG)
        } else if settings.serialize_dates && PyDateTime_Check(obj) != 0 {
            primitives::serialize_date(obj, buffer, pointers)
        } else if let Some(custom_type) = get_custom_type_mapping(settings, &typ) {
            custom_types::serialize_custom_type(obj, buffer, pointers, settings, custom_type)
        } else {
            if !settings.serialize_dates && PyDateTime_Check(obj) != 0 {
                return Err(
                    "Unexpected type: datetime, with flag serialize_dates disabled"
                        .to_py_error(SERIALIZATION_ERROR_TYPE),
                );
            }
            Err(format!("Unexpected type: {}", pretty_type(obj))
                .to_py_error(SERIALIZATION_ERROR_TYPE))
        }
    }
}

fn get_custom_type_mapping<'a>(settings: &'a Settings, typ: &*mut PyTypeObject) -> Option<&'a CustomType> {
    if let Some(custom_types) = &settings.custom_types {
        for (key, custom_type) in custom_types {
            if typ == key || (custom_type.include_subclasses && unsafe { PyType_IsSubtype(*typ, *key) } != 0) {
                return Some(custom_type);
            }

        }
    }
    None
}
