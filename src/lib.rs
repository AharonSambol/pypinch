#![allow(static_mut_refs)]
#![allow(unused_unsafe)]

use std::ffi::CString;
use std::os::raw::c_char;
use std::ptr;

use crate::deserializing::deserialize::deserialize_object;
use crate::serializing::py_bytes_buffer::PyBytesBuffer;
use crate::serializing::serialize::serialize;
use crate::serializing::settings::Settings;
use crate::serializing::utils::{CUSTOM_TYPE_CLASS, EMPTY_BYTES, EMPTY_STRING, EMPTY_TUPLE, ISO_FORMAT_FUNC, SERIALIZATION_ERROR_TYPE};
use crate::utils::consts::HEADER;
use crate::utils::py_helpers::{
    compare_str, convert_py_buffer_into_bytes_slice, import_object_from_python, py_str_to_rust_str,
    ToPyErr,
};
use crate::utils::wrappers::{gc_disable, gc_enabled, is_gc_enabled, tuple_get_item};
use deserializing::utils::DESERIALIZATION_ERROR_TYPE;
use pyo3_ffi::*;
use rustc_hash::FxHashMap;
use utils::custom_type_loaders;
mod deserializing;
mod serializing;
mod utils;

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
        ml_doc: "serializes pinch\0".as_ptr().cast::<c_char>(),
    },
    PyMethodDef {
        ml_name: "load_bytes\0".as_ptr().cast::<c_char>(),
        ml_meth: PyMethodDefPointer {
            PyCFunctionFastWithKeywords: load_bytes,
        },
        ml_flags: METH_FASTCALL | METH_KEYWORDS,
        ml_doc: "deserializes pinch\0".as_ptr().cast::<c_char>(),
    },
    // A zeroed PyMethodDef to mark the end of the array.
    PyMethodDef::zeroed(),
];

// The module initialization function
#[allow(non_snake_case)]
#[no_mangle]
pub unsafe extern "C" fn PyInit__pypinch() -> *mut PyObject {
    EMPTY_TUPLE = PyTuple_New(0);
    EMPTY_STRING = PyUnicode_New(0, 127);
    EMPTY_BYTES = PyBytes_FromStringAndSize(ptr::null(), 0);
    DESERIALIZATION_ERROR_TYPE =
        import_object_from_python("pypinch.exceptions", "DeserializationError");
    SERIALIZATION_ERROR_TYPE =
        import_object_from_python("pypinch.exceptions", "SerializationError");
    CUSTOM_TYPE_CLASS =
        import_object_from_python("pypinch.serialize.settings", "CustomType");

    PyDateTime_IMPORT();
    let iso_format_py_string = CString::new("isoformat").unwrap();
    ISO_FORMAT_FUNC = PyObject_GetAttr(
        (*PyDateTimeAPI()).DateTimeType as *mut PyObject,
        PyUnicode_FromString(iso_format_py_string.as_ptr()),
    );
    if EMPTY_TUPLE.is_null()
        || EMPTY_STRING.is_null()
        || EMPTY_BYTES.is_null()
        || DESERIALIZATION_ERROR_TYPE.is_null()
        || SERIALIZATION_ERROR_TYPE.is_null()
        || CUSTOM_TYPE_CLASS.is_null()
    {
        return PyErr_NoMemory();
    }
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
    let mut serialize_dates: bool = false;
    let mut custom_types = None;
    // TODO
    let mut allow_non_string_keys: bool = true;

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
            } else if compare_str(key, b"custom_types\0") {
                let value = *args.offset(nargs + i);

                let custom_types_dict = match custom_type_loaders::parse_dumps_custom_types_dict(value) {
                    Ok(value) => value,
                    Err(value) => return value,
                };
                custom_types = Some(custom_types_dict);
            } else {
                let rust_str = py_str_to_rust_str(&key);
                return if let Ok(rust_str) = rust_str {
                    format!(
                        "dump_bytes() got an unexpected keyword argument '{}'",
                        rust_str
                    ).to_py_error(PyExc_TypeError)
                } else {
                    PyErr_NoMemory()
                }
            }
        }
    }

    let num_args = PyVectorcall_NARGS(nargs as usize);
    let obj = if let Some(obj) = obj {
        if num_args != 0 {
            return "dump_bytes() got multiple values for argument 'obj'"
                .to_py_error(PyExc_TypeError);
        }
        obj
    } else {
        if num_args != 1 {
            return format!(
                "dump_bytes() expected exactly 1 positional argument, but {num_args} were provided"
            )
            .to_py_error(PyExc_TypeError);
        }
        *args
    };
    let mut buf = match PyBytesBuffer::with_capacity(8) {
        Ok(buf) => buf,
        Err(err) => return err,
    };

    buf.extend_from_slice(b"<o>");
    let mut pointers = FxHashMap::default();
    let result = serialize(
        obj,
        &mut buf,
        &mut pointers,
        &mut 0,
        &Settings {
            serialize_dates,
            custom_types,
        },
    );
    if let Err(error) = result {
        return error;
    }

    buf.finish()
}

pub unsafe extern "C" fn load_bytes(
    _self: *mut PyObject,
    args: *const *mut PyObject,
    nargs: Py_ssize_t,
    kwnames: *mut PyObject,
) -> *mut PyObject {
    let mut buffer = None;
    let mut custom_types = None;
    let mut use_tuples: bool = false;
    let mut stop_gc: bool = false;
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
                stop_gc = PyObject_IsTrue(value) == 1;
            } else if compare_str(key, b"ignore_extra_data\0") {
                let value = *args.offset(nargs + i);
                ignore_extra_data = PyObject_IsTrue(value) == 1;
            } else if compare_str(key, b"custom_types\0") {
                let value = *args.offset(nargs + i);

                let custom_types_dict = match custom_type_loaders::parse_loads_custom_types_dict(value) {
                    Ok(value) => value,
                    Err(value) => return value,
                };
                custom_types = Some(custom_types_dict);
            } else {
                let rust_str = py_str_to_rust_str(&key);
                return if let Ok(rust_str) = rust_str {
                    format!(
                        "load_bytes() got an unexpected keyword argument '{}'",
                        rust_str
                    ).to_py_error(PyExc_TypeError)
                } else {
                    PyErr_NoMemory()
                }
            }
        }
    }

    let num_args = PyVectorcall_NARGS(nargs as usize);
    let buffer = if let Some(buffer) = buffer {
        if num_args != 0 {
            return "load_bytes() got multiple values for argument 'buffer'"
                .to_py_error(PyExc_TypeError);
        }
        buffer
    } else {
        if num_args != 1 {
            return format!(
                "load_bytes() expected exactly 1 positional argument, but {num_args} were provided"
            )
            .to_py_error(PyExc_TypeError);
        }
        *args
    };

    let should_enable_gc = if stop_gc {
        if is_gc_enabled() {
            gc_disable();
            true
        } else {
            false
        }
    } else {
        false
    };
    let mut pointers = vec![];
    let slice = match convert_py_buffer_into_bytes_slice(&buffer) {
        Ok(slice) => slice,
        Err(err) => {
            if should_enable_gc {
                gc_enabled();
            }
            return err;
        }
    };

    let mut pointer = HEADER.len();
    let result = deserialize_object(
        slice,
        &mut pointer,
        &mut pointers,
        use_tuples,
        &mut 0,
        &custom_types,
    );
    if should_enable_gc {
        gc_enabled();
    }
    match result {
        Ok(result_object) => {
            if !ignore_extra_data && pointer != slice.len() {
                return format!(
                    "Unexpected extra data, from position {pointer}. If you want to ignore it use the flag `ignore_extra_data`"
                ).to_py_error(DESERIALIZATION_ERROR_TYPE);
            }
            result_object
        }
        Err(err) => err,
    }
}
