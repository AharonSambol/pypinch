use pyo3_ffi::{PyDict_SetItem, PyObject, Py_ssize_t};

use crate::deserializing::deserialize::deserialize_object;
use crate::deserializing::pointer_holders::pointer_holder::PointerHolder;
use crate::deserializing::primitives::decode_string;
use crate::deserializing::utils::{
    decode_number_py_ssize_t, decode_number_usize, DESERIALIZATION_ERROR_TYPE,
};
use crate::utils::consts::{MIGHT_BE_ASCII, NUMBER_BASE};
use crate::utils::py_dict_key::PyHashMap;
use crate::utils::py_helpers::{pretty_type, ToPyErr};
use crate::utils::safe_py_pointer::PyPointer;
use crate::utils::wrappers::{list_set_item, tuple_set_item};
use crate::{safe_get, safe_new_py_dict, safe_new_py_list};

#[inline(always)]
pub fn decode_list<'a, P: PointerHolder>(
    buf: &'a [u8],
    ptr: &mut usize,
    pointers: &mut P,
    use_tuples: bool,
    custom_types: &Option<PyHashMap<*mut PyObject>>,
) -> Result<*mut PyObject, *mut PyObject> {
    let len = decode_number_py_ssize_t::<NUMBER_BASE>(buf, ptr)?;

    if use_tuples {
        let tup = safe_new_py_list!(len, true);
        for i in 0..len {
            let obj = deserialize_object(buf, ptr, pointers, use_tuples, custom_types)?;
            tuple_set_item(tup, i, obj);
        }
        Ok(tup)
    } else {
        let list = safe_new_py_list!(len, false);
        for i in 0..len {
            let obj = deserialize_object(buf, ptr, pointers, use_tuples, custom_types)?;
            list_set_item(list, i, obj);
        }
        Ok(list)
    }
}

#[inline(always)]
pub fn decode_str_key_dict<'a, P: PointerHolder>(
    buf: &'a [u8],
    ptr: &mut usize,
    pointers: &mut P,
    use_tuples: bool,
    custom_types: &Option<PyHashMap<*mut PyObject>>,
) -> Result<*mut PyObject, *mut PyObject> {
    let len = decode_number_usize::<NUMBER_BASE>(buf, ptr)?;
    let dict = safe_new_py_dict!();
    for _ in 0..len {
        let key = PyPointer::new(deserialize_dict_key(buf, ptr, pointers)?);
        let value = PyPointer::new(deserialize_object(buf, ptr, pointers, use_tuples, custom_types)?);
        unsafe {
            PyDict_SetItem(dict, key.as_ptr(), value.as_ptr());
        }
    }
    Ok(dict)
}

pub fn deserialize_dict_key<'a, P: PointerHolder>(
    buf: &'a [u8],
    ptr: &mut usize,
    pointers: &mut P,
) -> Result<*mut PyObject, *mut PyObject> {
    if *safe_get!(buf, *ptr) == NUMBER_BASE as u8 - 1 {
        *ptr += 1;
        let position = decode_number_usize::<NUMBER_BASE>(buf, ptr)?;
        pointers.safe_get(position)
    } else {
        decode_string::<MIGHT_BE_ASCII, { NUMBER_BASE - 1 }, P>(
            buf,
            ptr,
            pointers,
        )
    }
}

#[inline(always)]
pub fn decode_dict<'a, P: PointerHolder>(
    buf: &'a [u8],
    ptr: &mut usize,
    pointers: &mut P,
    use_tuples: bool,
    custom_types: &Option<PyHashMap<*mut PyObject>>,
) -> Result<*mut PyObject, *mut PyObject> {
    let len = decode_number_usize::<NUMBER_BASE>(buf, ptr)?;
    let dict = safe_new_py_dict!();
    for _ in 0..len {
        let key = PyPointer::new(deserialize_object(buf, ptr, pointers, use_tuples, custom_types)?);
        let value = PyPointer::new(deserialize_object(buf, ptr, pointers, use_tuples, custom_types)?);
        unsafe {
            if PyDict_SetItem(dict, key.as_ptr(), value.as_ptr()) != 0 {
                return Err(format!("Invalid type for a key: {}", pretty_type(key.as_ptr()))
                    .to_py_error(DESERIALIZATION_ERROR_TYPE));
            }
        }
    }
    Ok(dict)
}

pub fn decode_list_of_structured_dicts<'a, P: PointerHolder>(
    buf: &'a [u8],
    ptr: &mut usize,
    pointers: &mut P,
    use_tuples: bool,
    custom_types: &Option<PyHashMap<*mut PyObject>>,
) -> Result<*mut PyObject, *mut PyObject> {
    let list_len = decode_number_py_ssize_t::<NUMBER_BASE>(buf, ptr)?;
    let list = safe_new_py_list!(list_len, use_tuples);

    // first dict:
    let dict_len = decode_number_usize::<NUMBER_BASE>(buf, ptr)?;
    let first_dict = safe_new_py_dict!();
    let mut keys = Vec::with_capacity(dict_len);
    for _ in 0..dict_len {
        let key = PyPointer::new(deserialize_object(buf, ptr, pointers, use_tuples, custom_types)?);
        let value = PyPointer::new(deserialize_object(buf, ptr, pointers, use_tuples, custom_types)?);
        unsafe {
            PyDict_SetItem(first_dict, key.as_ptr(), value.as_ptr());
        }
        keys.push(key);
    }
    if use_tuples {
        tuple_set_item(list, 0, first_dict);
    } else {
        list_set_item(list, 0, first_dict);
    }

    // the rest of the dicts:
    for i in 1usize..list_len as usize {
        let dict = safe_new_py_dict!();
        for key_index in 0..dict_len {
            let value = PyPointer::new(deserialize_object(buf, ptr, pointers, use_tuples, custom_types)?);
            unsafe {
                PyDict_SetItem(dict, keys[key_index].as_ptr(), value.as_ptr());
            }
        }
        if use_tuples {
            tuple_set_item(list, i as Py_ssize_t, dict);
        } else {
            list_set_item(list, i as Py_ssize_t, dict);
        }
    }

    Ok(list)
}
