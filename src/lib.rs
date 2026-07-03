#![allow(static_mut_refs)]
#![allow(unused_unsafe)]

use crate::deserializing::deserialize::deserialize_object;
use crate::deserializing::lazy_deserialize::lazy_deserialize;
use crate::deserializing::pointer_holders::position_pointer_holder::PositionPointerHolder;
use crate::deserializing::pointer_holders::vec_pointer_holder::VecPointerHolder;
use crate::deserializing::primitives::decode_false;
use crate::serializing::py_bytes_buffer::{FilePyBytesBuffer, MemoryPyBytesBuffer, PyBytesBuffer};
use crate::serializing::serialize::serialize;
use crate::serializing::serializing_string_cache::Pointers;
use crate::serializing::settings::{CustomType, Settings};
use crate::serializing::utils::{
    CUSTOM_TYPE_CLASS, EMPTY_BYTES, EMPTY_STRING, EMPTY_TUPLE, IDX_CLASS, ISO_FORMAT_FUNC,
    SERIALIZATION_ERROR_TYPE,
};
use crate::utils::consts::{CORRUPTED_DATA, HEADER};
use crate::utils::custom_type_loaders::parse_loads_custom_types_dict;
use crate::utils::path_to_load_loaders::parse_path_to_load;
use crate::utils::py_helpers::{
    compare_str, convert_py_buffer_into_bytes_slice, import_object_from_python, pretty_type,
    py_str_to_rust_str, ToPyErr,
};
use crate::utils::wrappers::{gc_disable, gc_enabled, is_gc_enabled, tuple_get_item};
use deserializing::utils::DESERIALIZATION_ERROR_TYPE;
use pyo3_ffi::*;
use std::collections::HashMap;
use std::ffi::CString;
use std::os::raw::c_char;
use std::ptr;
use utils::custom_type_loaders;

mod deserializing;
mod serializing;
mod utils;

const MEBIBYTE: usize = 1024 * 1024;

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

static mut METHODS: [PyMethodDef; 5] = [
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
    PyMethodDef {
        ml_name: "lazy_load_bytes\0".as_ptr().cast::<c_char>(),
        ml_meth: PyMethodDefPointer {
            PyCFunctionFastWithKeywords: lazy_load_bytes,
        },
        ml_flags: METH_FASTCALL | METH_KEYWORDS,
        ml_doc: "lazily deserializes pinch\0".as_ptr().cast::<c_char>(),
    },
    PyMethodDef {
        ml_name: "bytes_check_if_contains\0".as_ptr().cast::<c_char>(),
        ml_meth: PyMethodDefPointer {
            PyCFunctionFastWithKeywords: bytes_check_if_contains,
        },
        ml_flags: METH_FASTCALL | METH_KEYWORDS,
        ml_doc: "lazily deserializes pinch\0".as_ptr().cast::<c_char>(),
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
    CUSTOM_TYPE_CLASS = import_object_from_python("pypinch.serialize.settings", "CustomType");
    IDX_CLASS = import_object_from_python("pypinch.deserialize.lazy_load", "Idx");

    PyDateTime_IMPORT();
    if PyDateTimeAPI().is_null() {
        return PyErr_NoMemory();
    }
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
        || IDX_CLASS.is_null()
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
    // TODO:
    let mut allow_non_string_keys: bool = true;
    let mut writer = None;
    let mut flush_threshold = 10 * MEBIBYTE;
    let mut direct_write_threshold = 5 * MEBIBYTE;

    if !kwnames.is_null() {
        let nkw = PyTuple_Size(kwnames);

        for i in 0..nkw {
            let key = tuple_get_item(kwnames, i);
            let value = *args.offset(nargs + i);
            if compare_str(key, b"obj\0") {
                obj = Some(value);
            } else if compare_str(key, b"allow_non_string_keys\0") {
                allow_non_string_keys = PyObject_IsTrue(value) == 1;
            } else if compare_str(key, b"serialize_dates\0") {
                serialize_dates = PyObject_IsTrue(value) == 1;
            } else if compare_str(key, b"writer\0") {
                writer = Some(value);
            } else if compare_str(key, b"flush_threshold\0") {
                if PyNumber_Check(value) != 1 {
                    return format!(
                        "Expected flush_threshold to be of type `int` but got `{}`",
                        pretty_type(value)
                    )
                    .to_py_error(PyExc_TypeError);
                }

                flush_threshold = unsafe { PyLong_AsSize_t(value) } as usize;
                if flush_threshold == usize::MAX && !PyErr_Occurred().is_null() {
                    return format!(
                            "Expected flush_threshold to be a positive integer smaller than 2**{} (max: {})",
                        usize::BITS,
                        usize::MAX,
                    ).to_py_error(PyExc_TypeError);
                }
            } else if compare_str(key, b"direct_write_threshold\0") {
                if PyNumber_Check(value) != 1 {
                    return format!(
                        "Expected direct_write_threshold to be of type `int` but got `{}`",
                        pretty_type(value)
                    )
                    .to_py_error(PyExc_TypeError);
                }
                direct_write_threshold = unsafe { PyLong_AsSize_t(value) } as usize;
                if direct_write_threshold == usize::MAX && !PyErr_Occurred().is_null() {
                    return format!(
                        "Expected direct_write_threshold to be a positive integer smaller than 2**{} (max: {})",
                        usize::BITS,
                        usize::MAX,
                    ).to_py_error(PyExc_TypeError);
                }
            } else if compare_str(key, b"custom_types\0") {
                let custom_types_dict =
                    match custom_type_loaders::parse_dumps_custom_types_dict(value) {
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
                    )
                    .to_py_error(PyExc_TypeError)
                } else {
                    PyErr_NoMemory()
                };
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
    let mut pointers = Pointers::new();

    match writer {
        Some(writer) => {
            match FilePyBytesBuffer::with_writer(8, writer, flush_threshold, direct_write_threshold)
            {
                Ok(buf) => call_serialize(serialize_dates, custom_types, obj, pointers, buf),
                Err(err) => err,
            }
        }
        None => match MemoryPyBytesBuffer::with_capacity(8) {
            Ok(buf) => call_serialize(serialize_dates, custom_types, obj, pointers, buf),
            Err(err) => err,
        },
    }
}

#[inline(always)]
fn call_serialize<Buffer: PyBytesBuffer>(
    serialize_dates: bool,
    custom_types: Option<HashMap<*mut PyTypeObject, CustomType>>,
    obj: *mut PyObject,
    mut pointers: Pointers,
    mut buf: Buffer,
) -> *mut PyObject {
    _ = buf.extend_from_slice(b"<o>");

    let result = serialize(
        obj,
        &mut buf,
        &mut pointers,
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
            let value = *args.offset(nargs + i);
            if compare_str(key, b"buffer\0") {
                if PyBytes_Check(value) != 1 && PyByteArray_Check(value) != 1 {
                    return format!(
                        "buffer must be of type `bytes` or `bytearray` but got `{}`",
                        pretty_type(value)
                    )
                    .to_py_error(PyExc_TypeError);
                }
                buffer = Some(value);
            } else if compare_str(key, b"use_tuples\0") {
                use_tuples = PyObject_IsTrue(value) == 1;
            } else if compare_str(key, b"stop_gc\0") {
                stop_gc = PyObject_IsTrue(value) == 1;
            } else if compare_str(key, b"ignore_extra_data\0") {
                ignore_extra_data = PyObject_IsTrue(value) == 1;
            } else if compare_str(key, b"custom_types\0") {
                let custom_types_dict = match parse_loads_custom_types_dict(value) {
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
                    )
                    .to_py_error(PyExc_TypeError)
                } else {
                    PyErr_NoMemory()
                };
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
    let mut pointers = VecPointerHolder::new();
    let slice = match convert_py_buffer_into_bytes_slice(&buffer) {
        Ok(slice) => slice,
        Err(err) => {
            if should_enable_gc {
                gc_enabled();
            }
            return err;
        }
    };

    if !slice.starts_with(HEADER) {
        return format!(
            "{CORRUPTED_DATA}: missing starting marker `{}`",
            std::str::from_utf8(HEADER).unwrap()
        )
            .to_py_error(DESERIALIZATION_ERROR_TYPE);
    }
    let mut pointer = HEADER.len();
    let result = deserialize_object(
        slice,
        &mut pointer,
        &mut pointers,
        use_tuples,
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

pub unsafe extern "C" fn lazy_load_bytes(
    _self: *mut PyObject,
    args: *const *mut PyObject,
    nargs: Py_ssize_t,
    kwnames: *mut PyObject,
) -> *mut PyObject {
    call_lazy_load(_self, args, nargs, kwnames, false).unwrap_or_else(|err| err)
}

pub unsafe extern "C" fn bytes_check_if_contains(
    _self: *mut PyObject,
    args: *const *mut PyObject,
    nargs: Py_ssize_t,
    kwnames: *mut PyObject,
) -> *mut PyObject {
    call_lazy_load(_self, args, nargs, kwnames, true).unwrap_or_else(|err| {
        if PyErr_ExceptionMatches(DESERIALIZATION_ERROR_TYPE) == 1 {
            PyErr_Clear();
            decode_false()
        } else {
            err
        }
    })
}

unsafe fn call_lazy_load(
    _self: *mut PyObject,
    mut args: *const *mut PyObject,
    nargs: Py_ssize_t,
    kwnames: *mut PyObject,
    dont_load: bool,
) -> Result<*mut PyObject, *mut PyObject> {
    let mut buffer = None;
    let mut custom_types = None;
    let mut path_to_load = None;
    let mut include_falsy = true;
    // let mut use_tuples: bool = false;
    // let mut stop_gc: bool = false;
    if !kwnames.is_null() {
        let nkw = PyTuple_Size(kwnames);

        for i in 0..nkw {
            let key = tuple_get_item(kwnames, i);
            let value = *args.offset(nargs + i);
            if compare_str(key, b"buffer\0") {
                if PyBytes_Check(value) != 1 && PyByteArray_Check(value) != 1 {
                    return Err(format!(
                        "buffer must be of type `bytes` or `bytearray` but got `{}`",
                        pretty_type(value)
                    )
                    .to_py_error(PyExc_TypeError));
                }
                buffer = Some(value);
            } else if compare_str(key, b"include_falsy\0") {
                include_falsy = PyObject_IsTrue(value) == 1;
            }  else if compare_str(key, b"path_to_load\0") {
                path_to_load = Some(value);
            } else if compare_str(key, b"custom_types\0") {
                custom_types = Some(parse_loads_custom_types_dict(value)?);
            } else {
                let rust_str = py_str_to_rust_str(&key);
                return Err(if let Ok(rust_str) = rust_str {
                    format!(
                        "lazy_load_bytes() got an unexpected keyword argument '{}'",
                        rust_str
                    )
                    .to_py_error(PyExc_TypeError)
                } else {
                    PyErr_NoMemory()
                });
            }
        }
    }

    let mut num_args = PyVectorcall_NARGS(nargs as usize);
    let original_num_args = num_args;

    let buffer = if let Some(buffer) = buffer {
        if num_args != 0 {
            return Err(
                "lazy_load_bytes() got multiple values for argument 'buffer'"
                    .to_py_error(PyExc_TypeError),
            );
        }
        buffer
    } else {
        if num_args == 0 {
            return Err(
                "lazy_load_bytes() missing 1 required positional argument: 'buffer'"
                    .to_py_error(PyExc_TypeError),
            );
        }
        let buffer = *args;
        args = args.add(1);
        num_args -= 1;
        buffer
    };
    let path_to_load = if let Some(path_to_load) = path_to_load {
        if num_args != 0 {
            return Err(
                "lazy_load_bytes() got multiple values for argument 'path_to_load'"
                    .to_py_error(PyExc_TypeError),
            );
        }
        path_to_load
    } else {
        if num_args != 1 {
            return Err(format!(
                "lazy_load_bytes() expected exactly 2 positional argument, but {original_num_args} were provided"
            )
            .to_py_error(PyExc_TypeError));
        }
        *args
    };
    let path_to_load = parse_path_to_load(path_to_load)?;

    let slice = convert_py_buffer_into_bytes_slice(&buffer)?;
    let mut pointers = PositionPointerHolder::new(slice);

    if !slice.starts_with(HEADER) {
        return Err(format!(
            "{CORRUPTED_DATA}: missing starting marker `{}`",
            std::str::from_utf8(HEADER).unwrap()
        )
        .to_py_error(DESERIALIZATION_ERROR_TYPE));
    }
    let mut pointer = HEADER.len();
    lazy_deserialize(
        slice,
        &mut pointer,
        &mut pointers,
        false,
        &custom_types,
        dont_load,
        include_falsy,
        &path_to_load,
    )
}
