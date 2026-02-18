use std::ffi::{c_char, c_long};

use pyo3_ffi::{Py_DECREF, Py_False, Py_INCREF, Py_None, Py_ssize_t, Py_True, PyBytes_FromStringAndSize, PyDict_New, PyDict_SetItem, PyErr_SetString, PyExc_TypeError, PyFloat_FromDouble, PyList_New, PyLong_FromLong, PyNumber_Negative, PyObject, PyTuple_New, PyUnicode_New};
use rustc_hash::FxHashMap;

use crate::deserializing::string_cache::StringCache;
use crate::deserializing::utils::{decode_large_number, decode_number__py_ssize_t, decode_number__usize};
use crate::py_string;
use crate::utils::consts::{NOT_A_STR_BUT_A_POINTER_FLAG, AMOUNT_OF_USED_FLAGS, BOOL_FLAG, BYTES_FLAG, CONSISTENT_TYPE_LIST_FLAG, DICT_FLAG, EMPTY_BYTES_FLAG, EMPTY_DICT_FLAG, EMPTY_LIST_FLAG, EMPTY_STR_FLAG, FALSE_FLAG, FLOAT_FLAG, INT_FLAG, INVALID_UTF_8_START_BYTE, LEFTMOST_BIT_MASK, LIST_FLAG, NEGATIVE_INT_FLAG, NEGATIVE_NUMBER_SIGN, NULL_FLAG, NUMBER_BASE, NUMBER_BASE_USIZE, POINTER_FLAG, POSITIVE_INT_FLAG, STR_FLAG, STR_KEY_DICT_FLAG, TRUE_FLAG};
use crate::utils::wrappers::{list_set_item, tuple_set_item};

// todo add necessary checks so it never crashes completely
pub unsafe fn deserialize_object<'a>(
    buf: &'a [u8],
    ptr: &mut usize,
    pointers: &mut FxHashMap<usize, *mut PyObject>,
    use_tuples: bool,
    string_cache: &mut StringCache<'a>,
    str_count: &mut usize,
) -> *mut PyObject {
    let flag = *buf.get_unchecked(*ptr);
    *ptr += 1;
    match flag {
        POSITIVE_INT_FLAG => {
            decode_large_number::<NUMBER_BASE>(buf, ptr)
        }
        NEGATIVE_INT_FLAG => {
            let num = decode_large_number::<NUMBER_BASE>(buf, ptr);
            let res = PyNumber_Negative(num);
            Py_DECREF(num);
            res
        }
        FLOAT_FLAG => {
            PyFloat_FromDouble(decode_f64(buf, ptr))
        }
        STR_FLAG => decode_string(
            buf,
            ptr,
            pointers,
            string_cache,
            str_count
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
            let pos = decode_number__usize::<NUMBER_BASE>(buf, ptr);
            let res = pointers[&pos];
            Py_INCREF(res);
            res
        }
        BYTES_FLAG => {
            let len = decode_number__py_ssize_t::<NUMBER_BASE>(buf, ptr);
            let bytes = PyBytes_FromStringAndSize(
                buf.as_ptr().add(*ptr) as *const c_char,
                len,
            );
            *ptr += len as usize;
            bytes
        },
        CONSISTENT_TYPE_LIST_FLAG => {
            let typ = *buf.get_unchecked(*ptr);
            *ptr += 1;
            let len= decode_number__py_ssize_t::<NUMBER_BASE>(buf, ptr);

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
                        if is_negative_number {
                            *ptr += 1;
                            let num= decode_large_number::<{ NUMBER_BASE - 1 }>(buf, ptr);
                            list_set_item(list, i, PyNumber_Negative(num));   // todo support larger numbers
                            Py_DECREF(num);
                        } else {
                            let num= decode_large_number::<{ NUMBER_BASE - 1 }>(buf, ptr);
                            list_set_item(list, i, num);
                        }
                    }
                    list
                }
                BYTES_FLAG => {
                    let list = PyList_New(len);
                    for i in 0..len {
                        let bytes_len = decode_number__py_ssize_t::<NUMBER_BASE>(buf, ptr);
                        let bytes = PyBytes_FromStringAndSize(
                            buf.as_ptr().add(*ptr) as *const c_char,
                            bytes_len,
                        );
                        list_set_item(list, i, bytes);
                        *ptr += bytes_len as usize;
                    }
                    list
                }
                STR_FLAG => {
                    let list = PyList_New(len);
                    for i in 0..len {
                        let str = decode_string(
                            buf,
                            ptr,
                            pointers,
                            string_cache,
                            str_count,
                        );
                        list_set_item(list, i, str);
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
            let len = decode_number__usize::<NUMBER_BASE>(buf, ptr);
            let dict = PyDict_New();
            for _ in 0..len {
                let k = deserialize_object(buf, ptr, pointers, use_tuples, string_cache, str_count);
                let v = deserialize_object(buf, ptr, pointers, use_tuples, string_cache, str_count);
                PyDict_SetItem(dict, k, v);
            }
            dict
        }
        STR_KEY_DICT_FLAG => {
            let len = decode_number__usize::<NUMBER_BASE>(buf, ptr);
            let dict = PyDict_New();
            for _ in 0..len {
                let key = if buf[*ptr..*ptr + NOT_A_STR_BUT_A_POINTER_FLAG.len()] == NOT_A_STR_BUT_A_POINTER_FLAG {
                    *ptr += NOT_A_STR_BUT_A_POINTER_FLAG.len();
                    let position = decode_number__usize::<NUMBER_BASE>(buf, ptr) as usize;
                    pointers[&position]
                } else {
                    decode_string(
                        buf,
                        ptr,
                        pointers,
                        string_cache,
                        str_count,
                    )
                };
                let value = deserialize_object(buf, ptr, pointers, use_tuples, string_cache, str_count);
                PyDict_SetItem(dict, key, value);
            }
            dict
        }
        EMPTY_BYTES_FLAG => PyBytes_FromStringAndSize(std::ptr::null(), 0),
        EMPTY_DICT_FLAG => PyDict_New(),
        EMPTY_LIST_FLAG => if use_tuples { PyTuple_New(0) /*todo cache this?*/} else { PyList_New(0) },
        EMPTY_STR_FLAG => PyUnicode_New(0, 127), // todo cache this?
        LIST_FLAG => {
            let len = decode_number__py_ssize_t::<NUMBER_BASE>(buf, ptr);

            if use_tuples {
                let tup = PyTuple_New(len);
                for i in 0..len {
                    let obj = deserialize_object(buf, ptr, pointers, use_tuples, string_cache, str_count);
                    tuple_set_item(tup, i, obj);
                }
                tup
            } else {
                let list = PyList_New(len);
                for i in 0..len {
                    let obj = deserialize_object(buf, ptr, pointers, use_tuples, string_cache, str_count);
                    list_set_item(list, i, obj);
                }
                list
            }
        },
        _ => {
            PyLong_FromLong((flag - AMOUNT_OF_USED_FLAGS) as c_long)
        }
    }
}

#[inline(always)]
unsafe fn decode_string<'a>(
    buf: &'a [u8],
    ptr: &mut usize,
    pointers: &mut FxHashMap<usize, *mut PyObject>,
    string_cache: &mut StringCache<'a>,
    str_count: &mut usize,
) -> *mut PyObject {
    let len = decode_number__usize::<NUMBER_BASE>(buf, ptr);

    let string = string_cache.get_or_create(&buf[*ptr..*ptr + len]);
    *ptr += len;

    pointers.insert(*str_count, string);
    *str_count += 1;
    string
}

#[inline(always)]
unsafe fn decode_f64(buf: &[u8], ptr: &mut usize) -> f64 {
    let p = buf.as_ptr().add(*ptr) as *const u64;
    *ptr += 8;
    f64::from_bits(u64::from_be(std::ptr::read_unaligned(p)))
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
    let table = [Py_True(), Py_False()];
    for i in 0..amount_of_bytes {
        let mut byte = buf[*ptr + i];
        for _ in 0..8 {
            let obj = table[((byte & LEFTMOST_BIT_MASK) == 0) as usize];
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