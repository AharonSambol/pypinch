use crate::deserializing::deserialize::deserialize_object;
use crate::deserializing::pointer_holders::pointer_holder::PointerHolder;
use crate::deserializing::utils::DESERIALIZATION_ERROR_TYPE;
use crate::serializing::utils::SERIALIZATION_ERROR_TYPE;
use crate::utils::py_dict_key::{PyHashMap, PyKey};
use crate::utils::py_helpers::{pretty_type, py_str_to_rust_str, ToPyErr};
use crate::utils::wrappers::tuple_set_item;
use pyo3_ffi::{PyObject, PyObject_CallObject, PyObject_Str, PyTuple_New, Py_DECREF};

pub fn deserialize_custom_type<P: PointerHolder>(
    buf: &[u8],
    ptr: &mut usize,
    pointers: &mut P,
    use_tuples: bool,
    custom_types: &Option<PyHashMap<*mut PyObject>>,
) -> Result<*mut PyObject, *mut PyObject> {
    let type_identifier = deserialize_object(buf, ptr, pointers, use_tuples, custom_types)?;
    let serialized_object = deserialize_object(buf, ptr, pointers, use_tuples, custom_types)?;

    let custom_types = if let Some(custom_types) = custom_types {
        custom_types
    } else {
        &PyHashMap::default()
    };
    if let Some(converter) = custom_types.get(&PyKey(type_identifier)) {
        let args = unsafe { PyTuple_New(1) };
        tuple_set_item(args, 0, serialized_object);
        let converted_object = unsafe { PyObject_CallObject(*converter, args) };
        if converted_object.is_null() {
            Err("Failed to deserialize custom type"
                .to_py_error(unsafe { SERIALIZATION_ERROR_TYPE }))
        } else {
            Ok(converted_object)
        }
    } else {
        unsafe {
            let str_representation = PyObject_Str(type_identifier);
            let rust_str_representation = py_str_to_rust_str(&str_representation)?.to_string();
            Py_DECREF(str_representation);
            Err(
                format!(
                    "unknown custom type. please provide the correct mapping when deserializing. identifier: `{}` (type: `{}`)",
                    rust_str_representation,
                    pretty_type(type_identifier)
                ).to_py_error(DESERIALIZATION_ERROR_TYPE)
            )
        }
    }
}
