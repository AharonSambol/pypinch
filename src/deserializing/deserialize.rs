use std::ffi::c_long;

use pyo3_ffi::{PyLong_FromLong, PyObject, PyObject_CallObject, PyObject_Str, PyTuple_New, Py_DECREF, Py_INCREF};

use crate::deserializing::compound_types::{
    decode_dict, decode_list, decode_list_of_structured_dicts, decode_str_key_dict,
};
use crate::deserializing::consistent_typed_list::decode_consistent_type_list;
use crate::deserializing::primitives::{decode_bytes, decode_f64, decode_false, decode_negative_int, decode_null, decode_pointer, decode_sized_pointer, decode_string, decode_true};
use crate::deserializing::utils::{decode_large_number, DESERIALIZATION_ERROR_TYPE};
use crate::serializing::utils::{EMPTY_BYTES, EMPTY_STRING, EMPTY_TUPLE, SERIALIZATION_ERROR_TYPE};
use crate::utils::consts::{
    AMOUNT_OF_USED_FLAGS, ASCII_STR_FLAG, BYTES_FLAG, CONSISTENT_TYPE_LIST_FLAG, CUSTOM_TYPE_FLAG,
    DICT_FLAG, EMPTY_BYTES_FLAG, EMPTY_DICT_FLAG, EMPTY_LIST_FLAG, EMPTY_STR_FLAG, FALSE_FLAG,
    FLOAT_FLAG, LIST_FLAG, LIST_OF_STRUCTURED_DICTS_FLAG, NEGATIVE_INT_FLAG, NOT_ASCII, NULL_FLAG,
    NUMBER_BASE, POINTER_FLAG, POINTER_FLAG_1BYTE, POINTER_FLAG_2BYTE, POINTER_FLAG_3BYTE,
    POINTER_FLAG_4BYTE, POSITIVE_INT_FLAG, STR_FLAG, STR_KEY_DICT_FLAG, TRUE_FLAG, YES_ASCII,
};
use crate::utils::py_dict_key::{PyHashMap, PyKey};
use crate::utils::py_helpers::{pretty_type, py_str_to_rust_str, ToPyErr};
use crate::utils::wrappers::tuple_set_item;
use crate::{raise_mem_error_if_null, safe_get, safe_new_py_dict, safe_new_py_list};

pub fn deserialize_object<'a>(
    buf: &'a [u8],
    ptr: &mut usize,
    pointers: &mut Vec<*mut PyObject>,
    use_tuples: bool,
    str_count: &mut usize,
    custom_types: &Option<PyHashMap<*mut PyObject>>,
) -> Result<*mut PyObject, *mut PyObject> {
    let flag = *safe_get!(buf, *ptr);

    *ptr += 1;
    match flag {
        POSITIVE_INT_FLAG => decode_large_number::<NUMBER_BASE>(buf, ptr),
        NEGATIVE_INT_FLAG => decode_negative_int(buf, ptr),
        FLOAT_FLAG => decode_f64(buf, ptr),
        STR_FLAG => {
            decode_string::<NOT_ASCII, NUMBER_BASE>(buf, ptr, pointers, str_count)
        }
        ASCII_STR_FLAG => {
            decode_string::<YES_ASCII, NUMBER_BASE>(buf, ptr, pointers, str_count)
        }
        TRUE_FLAG => Ok(decode_true()),
        FALSE_FLAG => Ok(decode_false()),
        NULL_FLAG => Ok(decode_null()),
        POINTER_FLAG => decode_pointer(buf, ptr, pointers),
        POINTER_FLAG_1BYTE => decode_sized_pointer::<1>(buf, ptr, pointers),
        POINTER_FLAG_2BYTE => decode_sized_pointer::<2>(buf, ptr, pointers),
        POINTER_FLAG_3BYTE => decode_sized_pointer::<3>(buf, ptr, pointers),
        POINTER_FLAG_4BYTE => decode_sized_pointer::<4>(buf, ptr, pointers),
        BYTES_FLAG => decode_bytes(buf, ptr),
        CONSISTENT_TYPE_LIST_FLAG => {
            decode_consistent_type_list(buf, ptr, pointers, use_tuples, str_count)
        }
        DICT_FLAG => decode_dict(buf, ptr, pointers, use_tuples, str_count, custom_types),
        STR_KEY_DICT_FLAG => {
            decode_str_key_dict(buf, ptr, pointers, use_tuples, str_count, custom_types)
        }
        EMPTY_BYTES_FLAG => unsafe {
            Py_INCREF(EMPTY_BYTES);
            Ok(EMPTY_BYTES)
        },
        EMPTY_DICT_FLAG => Ok(safe_new_py_dict!()),
        EMPTY_LIST_FLAG => {
            if use_tuples {
                unsafe {
                    Py_INCREF(EMPTY_TUPLE);
                    Ok(EMPTY_TUPLE)
                }
            } else {
                Ok(safe_new_py_list!(0, false))
            }
        }
        EMPTY_STR_FLAG => unsafe {
            Py_INCREF(EMPTY_STRING);
            Ok(EMPTY_STRING)
        },
        LIST_FLAG => decode_list(buf, ptr, pointers, use_tuples, str_count, custom_types),
        LIST_OF_STRUCTURED_DICTS_FLAG => {
            decode_list_of_structured_dicts(buf, ptr, pointers, use_tuples, str_count, custom_types)
        }
        CUSTOM_TYPE_FLAG => {
            let type_identifier = deserialize_object(buf, ptr, pointers, use_tuples, str_count, custom_types)?;
            let serialized_object = deserialize_object(buf, ptr, pointers, use_tuples, str_count, custom_types)?;
            unsafe {
                println!("{} {}", custom_types.as_ref().unwrap().len(), py_str_to_rust_str(&PyObject_Str(type_identifier))?);
            }
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
                    Err(
                        "Failed to deserialize custom type".to_py_error(unsafe { SERIALIZATION_ERROR_TYPE })
                    )
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
        _ => unsafe {
            Ok(raise_mem_error_if_null!(PyLong_FromLong(
                (flag - AMOUNT_OF_USED_FLAGS) as c_long
            )))
        },
    }
}
