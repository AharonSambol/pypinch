use std::ffi::c_long;

use pyo3_ffi::{Py_INCREF, PyDict_New, PyFloat_FromDouble, PyList_New, PyLong_FromLong, PyObject};
use rustc_hash::FxHashMap;

use crate::deserializing::compound_types::{decode_dict, decode_list, decode_str_key_dict};
use crate::deserializing::consistent_typed_list::decode_consistent_type_list;
use crate::deserializing::primitives::{decode_bytes, decode_f64, decode_false, decode_negative_int, decode_null, decode_pointer, decode_string, decode_true};
use crate::deserializing::string_cache::StringCache;
use crate::deserializing::utils::decode_large_number;
use crate::serializing::utils::{EMPTY_BYTES, EMPTY_STRING, EMPTY_TUPLE};
use crate::utils::consts::{AMOUNT_OF_USED_FLAGS, BYTES_FLAG, CONSISTENT_TYPE_LIST_FLAG, DICT_FLAG, EMPTY_BYTES_FLAG, EMPTY_DICT_FLAG, EMPTY_LIST_FLAG, EMPTY_STR_FLAG, FALSE_FLAG, FLOAT_FLAG, LIST_FLAG, NEGATIVE_INT_FLAG, NULL_FLAG, NUMBER_BASE, POINTER_FLAG, POSITIVE_INT_FLAG, STR_FLAG, STR_KEY_DICT_FLAG, TRUE_FLAG};

// todo add necessary checks so it never crashes completely
pub unsafe fn deserialize_object<'a>(
    buf: &'a [u8],
    ptr: &mut usize,
    pointers: &mut FxHashMap<usize, *mut PyObject>,
    use_tuples: bool,
    string_cache: &mut StringCache<'a>,
    str_count: &mut usize,
) -> Result<*mut PyObject, *mut PyObject> {
    let flag = *buf.get_unchecked(*ptr);
    *ptr += 1;
    Ok(match flag {
        POSITIVE_INT_FLAG => decode_large_number::<NUMBER_BASE>(buf, ptr),
        NEGATIVE_INT_FLAG => decode_negative_int(buf, ptr),
        FLOAT_FLAG => PyFloat_FromDouble(decode_f64(buf, ptr)),
        STR_FLAG => decode_string(buf, ptr, pointers, string_cache, str_count),
        TRUE_FLAG => decode_true(),
        FALSE_FLAG => decode_false(),
        NULL_FLAG => decode_null(),
        POINTER_FLAG => decode_pointer(buf, ptr, pointers),
        BYTES_FLAG => decode_bytes(buf, ptr),
        CONSISTENT_TYPE_LIST_FLAG => decode_consistent_type_list(buf, ptr, pointers, use_tuples, string_cache, str_count)?,
        DICT_FLAG => decode_dict(buf, ptr, pointers, use_tuples, string_cache, str_count)?,
        STR_KEY_DICT_FLAG => decode_str_key_dict(buf, ptr, pointers, use_tuples, string_cache, str_count)?,
        EMPTY_BYTES_FLAG => { Py_INCREF(EMPTY_BYTES); EMPTY_BYTES },
        EMPTY_DICT_FLAG => PyDict_New(),
        EMPTY_LIST_FLAG => if use_tuples { Py_INCREF(EMPTY_TUPLE); EMPTY_TUPLE } else { PyList_New(0) },
        EMPTY_STR_FLAG => { Py_INCREF(EMPTY_STRING); EMPTY_STRING },
        LIST_FLAG => decode_list(buf, ptr, pointers, use_tuples, string_cache, str_count)?,
        _ => PyLong_FromLong((flag - AMOUNT_OF_USED_FLAGS) as c_long),
    })
}