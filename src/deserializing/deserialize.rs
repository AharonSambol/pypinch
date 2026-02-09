use std::ffi::{c_char, c_long, c_ulonglong};
use pyo3_ffi::{Py_False, Py_INCREF, Py_None, Py_ssize_t, Py_True, PyBytes_FromStringAndSize, PyDict_New, PyDict_SetItem, PyErr_SetString, PyExc_TypeError, PyFloat_FromDouble, PyList_New, PyList_SET_ITEM, PyLong_FromLong, PyLong_FromLongLong, PyLong_FromUnsignedLongLong, PyObject, PyTuple_New, PyTuple_SET_ITEM, PyUnicode_FromStringAndSize, PyUnicode_New};
use rustc_hash::FxHashMap;
use crate::deserializing::utils::decode_number;
use crate::py_string;
use crate::utils::consts::{AMOUNT_OF_USED_FLAGS, BOOL_FLAG, BYTES_FLAG, CONSISTENT_TYPE_LIST_FLAG, DICT_FLAG, EMPTY_BYTES_FLAG, EMPTY_DICT_FLAG, EMPTY_LIST_FLAG, EMPTY_STR_FLAG, ENDING_FLAG, FALSE_FLAG, FLOAT_FLAG, INT_FLAG, LEFTMOST_BIT_MASK, LIST_FLAG, NEGATIVE_INT_FLAG, NEGATIVE_NUMBER_SIGN, NULL_FLAG, NUMBER_BASE, POINTER_FLAG, POSITIVE_INT_FLAG, STR_FLAG, STR_KEY_DICT_FLAG, TRUE_FLAG};
use crate::utils::wrappers::{list_set_item, tuple_set_item};

pub unsafe fn deserialize_object(
    buf: &[u8],
    ptr: &mut usize,
    pointers: &mut Option<&mut FxHashMap<usize, *mut PyObject>>,
    use_tuples: bool,
) -> *mut PyObject {
    let flag = *buf.get_unchecked(*ptr);
    *ptr += 1;
    match flag {
        POSITIVE_INT_FLAG => {
            // todo support larger numbers
            let n= decode_number::<NUMBER_BASE>(buf, ptr);
            PyLong_FromUnsignedLongLong(n as c_ulonglong)
        }
        NEGATIVE_INT_FLAG => {
            // todo support larger numbers
            let n= decode_number::<NUMBER_BASE>(buf, ptr);
            PyLong_FromLongLong(-(n as i64))
        }
        FLOAT_FLAG => {
            PyFloat_FromDouble(decode_f64(buf, ptr))
        }
        STR_FLAG => decode_string(
            buf,
            ptr,
            pointers,
        ),
        TRUE_FLAG => {
            let t = Py_True();
            Py_INCREF(t);
            t
        },
        FALSE_FLAG => {
            let f = Py_False();
            Py_INCREF(f);
            f
        },
        NULL_FLAG => {
            let none = Py_None();
            Py_INCREF(none);
            none
        },
        POINTER_FLAG => {
            let pos = decode_number::<NUMBER_BASE>(buf, ptr);
            if let Some(pointers) = pointers {
                pointers[&(pos as usize)]
            } else {
                // todo not type error, encoding\decoding error
                PyErr_SetString(PyExc_TypeError, py_string!("unexpected flag pointer, when use_pointers is disabled"));
                return std::ptr::null_mut();
            }
        }
        BYTES_FLAG => {
            let len = decode_number::<NUMBER_BASE>(buf, ptr);
            let bytes = PyBytes_FromStringAndSize(
                buf.as_ptr().add(*ptr) as *const c_char,
                len as Py_ssize_t,
            );
            *ptr += len as usize;
            bytes
        },
        CONSISTENT_TYPE_LIST_FLAG => {
            
            let typ = *buf.get_unchecked(*ptr);
            *ptr += 1;
            let len= decode_number::<NUMBER_BASE>(buf, ptr) as Py_ssize_t;

            match typ {
                NULL_FLAG => {
                    let none = Py_None();

                    if use_tuples {
                        let tuple = PyTuple_New(len);
                        for i in 0..len {
                            Py_INCREF(none);
                            tuple_set_item(tuple, i, none);
                        }
                        tuple
                    } else {
                        let list = PyList_New(len);
                        for i in 0..len {
                            Py_INCREF(none);
                            list_set_item(list, i, none);
                        }
                        list
                    }
                }
                // todo all these can create tuples instead of lists
                BOOL_FLAG => decode_bool_list(buf, ptr, len),
                INT_FLAG => {
                    let list = PyList_New(len);
                    for i in 0..len {
                        let is_negative_number = *buf.get_unchecked(*ptr) == NEGATIVE_NUMBER_SIGN as u8;
                        *ptr += 1;
                        if is_negative_number {
                            let num= decode_number::<{ NUMBER_BASE - 1 }>(buf, ptr);
                            list_set_item(list, i, PyLong_FromLongLong(-(num as i64)));   // todo support larger numbers
                        } else {
                            let num= decode_number::<{ NUMBER_BASE - 1 }>(buf, ptr);
                            list_set_item(list, i, PyLong_FromUnsignedLongLong(num as c_ulonglong));
                        }
                    }
                    list
                }
                BYTES_FLAG => {
                    let list = PyList_New(len);
                    for i in 0..len {
                        let bytes_len = decode_number::<NUMBER_BASE>(buf, ptr);
                        let bytes = PyBytes_FromStringAndSize(
                            buf.as_ptr().add(*ptr) as *const c_char,
                            bytes_len as Py_ssize_t,
                        );
                        list_set_item(list, i, bytes);
                        *ptr += bytes_len as usize;
                    }
                    list
                }
                STR_FLAG => {
                    let list = PyList_New(len);
                    for i in 0..len {
                        let s = decode_string(
                            buf,
                            ptr,
                            pointers,
                        );
                        list_set_item(list, i, s);
                    }
                    list
                }
                FLOAT_FLAG => {
                    let list = PyList_New(len);
                    for i in 0..len {
                        let float = decode_f64(buf, ptr);
                        list_set_item(list, i, PyFloat_FromDouble(float));
                    }
                    list
                }
                _ => {
                    PyErr_SetString(PyExc_TypeError, py_string!("unexpected consistent list type"));
                    return std::ptr::null_mut();
                }
            }
        },
        DICT_FLAG =>  {
            let len = decode_number::<NUMBER_BASE>(buf, ptr);
            let dict = PyDict_New();
            for _ in 0..len {
                let k = deserialize_object(buf, ptr, pointers, use_tuples);
                let v = deserialize_object(buf, ptr, pointers, use_tuples);
                PyDict_SetItem(dict, k, v);
            }
            dict
        }
        STR_KEY_DICT_FLAG => {
            let len = decode_number::<NUMBER_BASE>(buf, ptr);
            let dict = PyDict_New();
            for _ in 0..len {
                let k = decode_string(
                    buf,
                    ptr,
                    pointers,
                );
                let v = deserialize_object(buf, ptr, pointers, use_tuples);
                PyDict_SetItem(dict, k, v);
            }
            dict
        }
        EMPTY_BYTES_FLAG => PyBytes_FromStringAndSize(std::ptr::null(), 0),
        EMPTY_DICT_FLAG => PyDict_New(),
        EMPTY_LIST_FLAG => if use_tuples { PyTuple_New(0) /*todo cache this?*/} else { PyList_New(0) },
        EMPTY_STR_FLAG => PyUnicode_New(0, 127), // todo cache this?
        LIST_FLAG => {
            let len = decode_number::<NUMBER_BASE>(buf, ptr) as Py_ssize_t;

            if use_tuples {
                let tup = PyTuple_New(len);
                for i in 0..len {
                    let obj = deserialize_object(buf, ptr, pointers, use_tuples);
                    tuple_set_item(tup, i, obj);
                }
                tup
            } else {
                let list = PyList_New(len);
                for i in 0..len {
                    let obj = deserialize_object(buf, ptr, pointers, use_tuples);
                    list_set_item(list, i, obj);
                }
                list
            }
        },
        _ => {
            // small int
            PyLong_FromLong((flag - AMOUNT_OF_USED_FLAGS) as c_long)
        }
    }
}

#[inline(always)]
unsafe fn decode_string(
    buf: &[u8],
    ptr: &mut usize,
    pointers: &mut Option<&mut FxHashMap<usize, *mut PyObject>>,
) -> *mut PyObject {
    let start = *ptr;
    let len = decode_number::<NUMBER_BASE>(buf, ptr);

    let string = PyUnicode_FromStringAndSize(
        buf.as_ptr().add(*ptr) as *const c_char,
        len as Py_ssize_t,
    );
    *ptr += len as usize;

    if let Some(map) = pointers {
        map.insert(start, string);
    }

    string
}

#[inline(always)]
unsafe fn decode_f64(buf: &[u8], ptr: &mut usize) -> f64 {
    let mut bytes = [0u8; 8];
    std::ptr::copy_nonoverlapping(
        buf.as_ptr().add(*ptr),
        bytes.as_mut_ptr(),
        8,
    );
    *ptr += 8;
    f64::from_be_bytes(bytes)
}

#[inline(always)]
unsafe fn decode_bool_list(
    buf: &[u8],
    ptr: &mut usize,
    length: Py_ssize_t,
) -> *mut PyObject {
    /*
    same as: math.ceil(length / NUMBER_OF_BITS_IN_BYTE)
    the `>> 3` is like dividing by 8 (8 is `1000` in binary)
    the + 7 is like rounding up
     */
    let amount_of_bytes = ((length as usize) + 7) >> 3;
    let list = PyList_New(length);

    let mut pos = 0;
    for i in 0..amount_of_bytes {
        let mut byte = buf[*ptr + i];
        for _ in 0..8 {
            let obj = if (byte & LEFTMOST_BIT_MASK) == 0 { Py_False() } else { Py_True() };
            Py_INCREF(obj);
            list_set_item(list, pos, obj);
            pos += 1;
            if pos == length {
                break;
            }
            byte <<= 1;
        }
    }
    *ptr += amount_of_bytes;
    list
}