use std::os::raw::c_char;
use std::ptr;
use pyo3_ffi::*;
use rustc_hash::FxHashMap;
use crate::serializing::serialize::serialize;
use crate::utils::consts::FALSE_FLAG;

mod utils;
mod serializing;

static mut MODULE_DEF: PyModuleDef = PyModuleDef {
    m_base: PyModuleDef_HEAD_INIT,
    m_name: "pinch\0".as_ptr().cast::<c_char>(),
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

static mut METHODS: [PyMethodDef; 2] = [
    PyMethodDef {
        ml_name: "dump_bytes\0".as_ptr().cast::<c_char>(),
        ml_meth: PyMethodDefPointer {
            _PyCFunctionFastWithKeywords: dumps,
        },
        ml_flags: METH_FASTCALL | METH_KEYWORDS,
        ml_doc: "serializes pinch\0"
            .as_ptr()
            .cast::<c_char>(),
    },
    // A zeroed PyMethodDef to mark the end of the array.
    PyMethodDef::zeroed()
];

// The module initialization function
#[allow(non_snake_case)]
#[no_mangle]
pub unsafe extern "C" fn PyInit_pinch() -> *mut PyObject {
    PyModule_Create(ptr::addr_of_mut!(MODULE_DEF))
}

pub unsafe extern "C" fn dumps(
    _self: *mut PyObject,
    args: *const *mut PyObject,
    nargs: Py_ssize_t,
    kwnames: *mut PyObject,
) -> *mut PyObject {
    let num_args = PyVectorcall_NARGS(nargs as usize);

    if num_args == 0 {
        PyErr_SetString(
            PyExc_TypeError,
            py_string!("dumps() expected 1 positional argument"),
        );
        return ptr::null_mut();
    }

    let arg1 = *args;

    let mut use_pointers = false;
    if !kwnames.is_null() {
        let nkw = PyTuple_Size(kwnames);

        for i in 0..nkw {
            let key = PyTuple_GetItem(kwnames, i);
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

    // match arg1.checked_add(arg2) {
    //     Some(sum) => {
    //         let string = sum.to_string();
    //         PyUnicode_FromStringAndSize(string.as_ptr().cast::<c_char>(), string.len() as isize)
    //     }
    //     None => {
    //         PyErr_SetString(
    //             PyExc_OverflowError,
    //             "arguments too large to add\0".as_ptr().cast::<c_char>(),
    //         );
    //         ptr::null_mut()
    //     }
    // }

    let mut buf = Vec::from(b"<o>");
    let mut map = FxHashMap::default();
    let mut pointers = if use_pointers { Some(&mut map) } else { None };
    serialize(arg1, &mut buf, &mut pointers);
    // buf.as_mut_ptr()
    let ptr = buf.as_ptr() as *const c_char;
    let len = buf.len() as Py_ssize_t;

    // Creates a new bytes object by copying data from the pointer
    PyBytes_FromStringAndSize(ptr, len)

}