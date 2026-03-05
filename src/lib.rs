#![allow(static_mut_refs)]
use std::os::raw::c_char;
use std::ptr;

use pyo3_ffi::*;
use rustc_hash::FxHashMap;
use deserializing::utils::DESERIALIZATION_ERROR_TYPE;
use crate::deserializing::deserialize::deserialize_object;
use crate::deserializing::string_cache::StringCache;
use crate::serializing::py_bytes_buffer::PyBytesBuffer;
use crate::serializing::serialize::serialize;
use crate::serializing::utils::{EMPTY_BYTES, EMPTY_STRING, EMPTY_TUPLE, SERIALIZATION_ERROR_TYPE};
use crate::utils::consts::HEADER;
use crate::utils::py_helpers::{compare_str, convert_py_buffer_into_bytes_slice, import_object_from_python, py_str_to_rust_str, ToPyErr};
use crate::utils::wrappers::tuple_get_item;

mod utils;
mod serializing;
mod deserializing;


static mut MODULE_DEF: PyModuleDef = PyModuleDef {
    m_base: PyModuleDef_HEAD_INIT,
    m_name: "_pypinch\0".as_ptr().cast::<c_char>(),
    m_doc: "A Python module written in Rust.\0"
        .as_ptr()
        .cast::<c_char>(),
    m_size: 0,
    m_methods: unsafe { METHODS.as_mut_ptr().cast() },
    m_slots: ptr::null_mut(),
    m_traverse: None,
    m_clear: None,
    m_free: None,
};

static mut METHODS: [PyMethodDef; 3] = [
    PyMethodDef {
        ml_name: "dump_bytes\0".as_ptr().cast::<c_char>(),
        ml_meth: PyMethodDefPointer {
            PyCFunctionFastWithKeywords: dump_bytes,
        },
        ml_flags: METH_FASTCALL | METH_KEYWORDS,
        ml_doc: "serializes pinch\0"
            .as_ptr()
            .cast::<c_char>(),
    },
    PyMethodDef {
        ml_name: "load_bytes\0".as_ptr().cast::<c_char>(),
        ml_meth: PyMethodDefPointer {
            PyCFunctionFastWithKeywords: load_bytes,
        },
        ml_flags: METH_FASTCALL | METH_KEYWORDS,
        ml_doc: "deserializes pinch\0"
            .as_ptr()
            .cast::<c_char>(),
    },
    // A zeroed PyMethodDef to mark the end of the array.
    PyMethodDef::zeroed()
];

// The module initialization function
#[allow(non_snake_case)]
#[no_mangle]
pub unsafe extern "C" fn PyInit__pypinch() -> *mut PyObject {
    EMPTY_TUPLE = PyTuple_New(0);
    EMPTY_STRING = PyUnicode_New(0, 127);
    EMPTY_BYTES = PyBytes_FromStringAndSize(ptr::null(), 0);
    DESERIALIZATION_ERROR_TYPE = import_object_from_python("pypinch.exceptions", "DeserializationError");
    SERIALIZATION_ERROR_TYPE = import_object_from_python("pypinch.exceptions", "SerializationError");

    PyModule_Create(ptr::addr_of_mut!(MODULE_DEF))
}


#[allow(unused)]
pub unsafe extern "C" fn dump_bytes(
    _self: *mut PyObject,
    args: *const *mut PyObject,
    nargs: Py_ssize_t,
    kwnames: *mut PyObject,
) -> *mut PyObject {
    let mut obj = None;
    // TODO
    let mut allow_non_string_keys: bool = true;
    let mut serialize_dates: bool = false;

    if !kwnames.is_null() {
        let nkw = PyTuple_Size(kwnames);

        for i in 0..nkw {
            let key = tuple_get_item(kwnames, i);
            if compare_str(key, b"obj\0") {
                obj = Some(*args.offset(nargs + i));
            } else if compare_str(key, b"allow_non_string_keys\0") {
                let value = *args.offset(nargs + i);
                allow_non_string_keys = PyObject_IsTrue(value) == 1;
            } else if compare_str(key, b"serialize_dates\0") {
                let value = *args.offset(nargs + i);
                serialize_dates = PyObject_IsTrue(value) == 1;
            } else {
                return format!(
                    "dump_bytes() got an unexpected keyword argument '{}'", py_str_to_rust_str(&key)
                ).to_py_error(PyExc_TypeError);
            }
        }
    }

    let num_args = PyVectorcall_NARGS(nargs as usize);
    let obj = if let Some(obj) = obj {
        if num_args != 0 {
            return "dump_bytes() got multiple values for argument 'obj'".to_py_error(PyExc_TypeError);
        }
        obj
    } else {
        if num_args != 1 {
            return format!(
                "dump_bytes() expected exactly 1 positional argument, but {num_args} were provided"
            ).to_py_error(PyExc_TypeError);
        }
        *args
    };
    let mut buf = PyBytesBuffer::with_capacity(8);
    buf.extend_from_slice(b"<o>");
    // let mut buf = Vec::from(b"<o>");
    let mut pointers = FxHashMap::default();
    let result = serialize(obj, &mut buf, &mut pointers, &mut 0);
    if let Err(error) = result {
        return error;
    }

    // let ptr = buf.as_ptr() as *const c_char;
    // let len = buf.len() as Py_ssize_t;
    // PyBytes_FromStringAndSize(ptr, len)
    buf.finish()
}

pub unsafe extern "C" fn load_bytes(
    _self: *mut PyObject,
    args: *const *mut PyObject,
    nargs: Py_ssize_t,
    kwnames: *mut PyObject,
) -> *mut PyObject {
    let mut buffer = None;
    let mut use_tuples: bool = false;
    let mut _stop_gc: bool;
    let mut ignore_extra_data: bool = false;

    if !kwnames.is_null() {
        let nkw = PyTuple_Size(kwnames);

        for i in 0..nkw {
            let key = tuple_get_item(kwnames, i);
            if compare_str(key, b"buffer\0") {
                buffer = Some(*args.offset(nargs + i));
            } else if compare_str(key, b"use_tuples\0") {
                let value = *args.offset(nargs + i);
                use_tuples = PyObject_IsTrue(value) == 1;
            } else if compare_str(key, b"stop_gc\0") {
                let value = *args.offset(nargs + i);
                _stop_gc = PyObject_IsTrue(value) == 1;
            } else if compare_str(key, b"ignore_extra_data\0") {
                let value = *args.offset(nargs + i);
                ignore_extra_data = PyObject_IsTrue(value) == 1;
            } else {
                return format!(
                    "load_bytes() got an unexpected keyword argument '{}'", py_str_to_rust_str(&key)
                ).to_py_error(PyExc_TypeError);
            }
        }
    }

    let num_args = PyVectorcall_NARGS(nargs as usize);
    let buffer = if let Some(buffer) = buffer {
        if num_args != 0 {
            return "load_bytes() got multiple values for argument 'buffer'".to_py_error(PyExc_TypeError);
        }
        buffer
    } else {
        if num_args != 1 {
            return format!(
                "load_bytes() expected exactly 1 positional argument, but {num_args} were provided"
            ).to_py_error(PyExc_TypeError);
        }
        *args
    };


    let mut pointers = FxHashMap::default();
    let slice = convert_py_buffer_into_bytes_slice(&buffer);

    let mut string_cache = StringCache::new();
    let mut pointer = HEADER.len();
    let result = deserialize_object(slice, &mut pointer, &mut pointers, use_tuples, &mut string_cache, &mut 0);
    match result {
        Ok(result_object) => {
            if !ignore_extra_data && pointer != slice.len() {
                return format!(
                    "Unexpected extra data, from position {pointer}. If you want to ignore it use the flag `ignore_extra_data`"
                ).to_py_error(DESERIALIZATION_ERROR_TYPE)
            }
            result_object
        }
        Err(err) => err
    }

}
