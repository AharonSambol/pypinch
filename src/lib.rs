use std::os::raw::c_char;
use std::{ptr, slice};
use pyo3_ffi::*;
use rustc_hash::FxHashMap;
use crate::deserializing::deserialize::deserialize_object;
use crate::deserializing::string_cache::StringCache;
use crate::serializing::serialize::serialize;
use crate::utils::consts::{HEADER};
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
            _PyCFunctionFastWithKeywords: dump_bytes,
        },
        ml_flags: METH_FASTCALL | METH_KEYWORDS,
        ml_doc: "serializes pinch\0"
            .as_ptr()
            .cast::<c_char>(),
    },
    PyMethodDef {
        ml_name: "load_bytes\0".as_ptr().cast::<c_char>(),
        ml_meth: PyMethodDefPointer {
            _PyCFunctionFastWithKeywords: load_bytes,
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
    PyModule_Create(ptr::addr_of_mut!(MODULE_DEF))
}

pub unsafe extern "C" fn dump_bytes(
    _self: *mut PyObject,
    args: *const *mut PyObject,
    nargs: Py_ssize_t,
    kwnames: *mut PyObject,
) -> *mut PyObject {
    let num_args = PyVectorcall_NARGS(nargs as usize);

    if num_args == 0 {
        PyErr_SetString(
            PyExc_TypeError,
            py_string!("dump_bytes() expected 1 positional argument"),
        );
        return ptr::null_mut();
    }

    let arg1 = *args;

    let mut use_pointers = true;
    if !kwnames.is_null() {
        let nkw = PyTuple_Size(kwnames);

        for i in 0..nkw {
            let key = tuple_get_item(kwnames, i);
            // key is guaranteed to be str
            if PyUnicode_CompareWithASCIIString(
                key,
                b"use_pointers\0".as_ptr() as *const _,
            ) == 0
            {
                let value = *args.offset(nargs + i);
                use_pointers = PyObject_IsTrue(value) == 1;
            }
        }
    }

    let mut buf = Vec::from(b"<o>");
    let mut map = FxHashMap::default();
    let mut pointers = if use_pointers { Some(&mut map) } else { None };
    serialize(arg1, &mut buf, &mut pointers, &mut 0);
    let ptr = buf.as_ptr() as *const c_char;
    let len = buf.len() as Py_ssize_t;
    PyBytes_FromStringAndSize(ptr, len)
}

pub unsafe extern "C" fn load_bytes(
    _self: *mut PyObject,
    args: *const *mut PyObject,
    nargs: Py_ssize_t,
    kwnames: *mut PyObject,
) -> *mut PyObject {
    let num_args = PyVectorcall_NARGS(nargs as usize);

    if num_args == 0 {
        PyErr_SetString(
            PyExc_TypeError,
            py_string!("load_bytes() expected 1 positional argument"),
        );
        return ptr::null_mut();
    }

    let arg1 = *args;

    let mut use_pointers = true;
    let mut use_tuples = false;
    if !kwnames.is_null() {
        let nkw = PyTuple_Size(kwnames);

        for i in 0..nkw {
            let key = tuple_get_item(kwnames, i);
            // key is guaranteed to be str
            if PyUnicode_CompareWithASCIIString(
                key,
                b"use_pointers\0".as_ptr() as *const _,
            ) == 0
            {
                let value = *args.offset(nargs + i);
                use_pointers = PyObject_IsTrue(value) == 1;
            }
            if PyUnicode_CompareWithASCIIString(
                key,
                b"use_tuples\0".as_ptr() as *const _,
            ) == 0
            {
                let value = *args.offset(nargs + i);
                use_tuples = PyObject_IsTrue(value) == 1;
            }
        }
    }

    let mut map = FxHashMap::default();
    let mut pointers = if use_pointers { Some(&mut map) } else { None };
    let slice = if (*arg1).ob_type == &mut PyByteArray_Type {
        let len = PyByteArray_Size(arg1) as usize;
        let data_ptr = PyByteArray_AsString(arg1) as *const u8;
        slice::from_raw_parts(data_ptr, len)
    } else {
        let len = PyBytes_Size(arg1) as usize;
        let data_ptr = PyBytes_AsString(arg1) as *const u8;
        slice::from_raw_parts(data_ptr, len)
    };

    let mut string_cache = StringCache::new();
    deserialize_object(slice, &mut (HEADER.len()), &mut pointers, use_tuples, &mut string_cache, &mut 0)
    
}