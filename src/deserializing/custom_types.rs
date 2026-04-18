use crate::deserializing::deserialize::deserialize_object;
use crate::deserializing::pointer_holders::pointer_holder::PointerHolder;
use crate::deserializing::utils::DESERIALIZATION_ERROR_TYPE;
use crate::serializing::utils::SERIALIZATION_ERROR_TYPE;
use crate::utils::py_dict_key::{PyHashMap, PyKey};
use crate::utils::py_helpers::{pretty_type, py_str_to_rust_str, ToPyErr};
use crate::utils::safe_py_pointer::PyPointer;
use crate::utils::wrappers::tuple_set_item;
use pyo3_ffi::{PyObject, PyObject_CallObject, PyObject_Str, PyTuple_New};

const FAILED_TO_DESERIALIZE_MESSAGE: &'static str = "Failed to deserialize custom type";
pub fn deserialize_custom_type<P: PointerHolder>(
    buf: &[u8],
    ptr: &mut usize,
    pointers: &mut P,
    use_tuples: bool,
    custom_types: &Option<PyHashMap<*mut PyObject>>,
) -> Result<*mut PyObject, *mut PyObject> {
    let type_identifier = PyPointer::new(deserialize_object(buf, ptr, pointers, use_tuples, custom_types)?);
    let serialized_object = deserialize_object(buf, ptr, pointers, use_tuples, custom_types)?;

    let custom_types = if let Some(custom_types) = custom_types {
        custom_types
    } else {
        &PyHashMap::default()
    };
    if let Some(converter) = custom_types.get(&PyKey(type_identifier.as_ptr())) {
        let args = PyPointer::new(unsafe { PyTuple_New(1) });
        if args.as_ptr().is_null() {
            return Err(FAILED_TO_DESERIALIZE_MESSAGE.to_py_error(unsafe { SERIALIZATION_ERROR_TYPE }));
        }
        tuple_set_item(args.as_ptr(), 0, serialized_object);
        let converted_object = unsafe { PyObject_CallObject(*converter, args.as_ptr()) };
        if converted_object.is_null() {
            Err(FAILED_TO_DESERIALIZE_MESSAGE.to_py_error(unsafe { SERIALIZATION_ERROR_TYPE }))
        } else {
            Ok(converted_object)
        }
    } else {
        unsafe {
            let str_representation = PyPointer::new_w_null_check(PyObject_Str(type_identifier.as_ptr()))?;
            let rust_str_representation = py_str_to_rust_str(&str_representation.as_ptr())?.to_string();
            Err(
                format!(
                    "unknown custom type. please provide the correct mapping when deserializing. identifier: `{}` (type: `{}`)",
                    rust_str_representation,
                    pretty_type(type_identifier.as_ptr())
                ).to_py_error(DESERIALIZATION_ERROR_TYPE)
            )
        }
    }
}
