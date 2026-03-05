use pyo3_ffi::*;
use rustc_hash::FxHashMap;
use crate::serializing::number_encoding::encode_python_int;
use crate::utils::consts::{FALSE_FLAG, NULL_FLAG, NUMBER_BASE, TRUE_FLAG};
use crate::utils::py_helpers::ToPyErr;
use pyo3_ffi::{PyBool_Type, PyBytes_Type, PyDict_Type, PyFloat_Type, PyList_Type, PyLong_Type, PyObject, PyTuple_Type, PyUnicode_Type};
use crate::serializing::{compound_types, primitives};
use crate::serializing::py_bytes_buffer::PyBytesBuffer;
use crate::serializing::utils::SERIALIZATION_ERROR_TYPE;

pub type Pointers = FxHashMap<*mut PyObject, usize>;



// todo: all_str_keys=False - if true store at the start a flag and then store all dicts without key types
#[inline(always)]
pub unsafe fn serialize(
    obj: *mut PyObject,
    buffer: &mut PyBytesBuffer,
    pointers: &mut Pointers,
    str_count: &mut usize,
) -> Result<(), *mut PyObject>{
    let typ = (*obj).ob_type;

    if typ == &mut PyUnicode_Type {
        primitives::serialize_str(obj, buffer, pointers, str_count);
    } else if typ == &mut PyBool_Type {
        buffer.push(if obj == Py_True() { TRUE_FLAG } else { FALSE_FLAG });
    } else if typ == &mut PyLong_Type {
        encode_python_int::<NUMBER_BASE>(obj, buffer);
    } else if typ == &mut PyList_Type || typ == &mut PyTuple_Type {
        compound_types::encode_list(obj, buffer, pointers, str_count, typ)?;
    } else if typ == &mut PyDict_Type {
        compound_types::serialize_dict(obj, buffer, pointers, str_count)?;
    } else if typ == &mut PyFloat_Type {
        primitives::serialize_float(obj, buffer);
    } else if typ == &mut PyBytes_Type {
        primitives::serialize_bytes(obj, buffer);
    } else if obj == Py_None() {
        buffer.push(NULL_FLAG)
    } else {
        return Err(format!("Unexpected type: {:?}", (*typ).tp_name).to_py_error(SERIALIZATION_ERROR_TYPE));
    }
    return Ok(());
}

