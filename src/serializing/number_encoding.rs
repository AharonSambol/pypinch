use std::ffi::c_long;
use pyo3_ffi::{_PyLong_AsByteArray, _PyLong_NumBits, Py_DECREF, PyErr_SetString, PyExc_RuntimeError, PyLong_AsLongLongAndOverflow, PyLong_FromLong, PyLongObject, PyNumber_Add, PyNumber_Subtract, PyObject, PyObject_RichCompareBool};
use crate::serializing::utils::encode_number;
use crate::utils::consts::{AMOUNT_OF_USED_FLAGS, ENDING_FLAG, NEGATIVE_INT_FLAG, NUMBER_BASE, POSITIVE_INT_FLAG};

pub unsafe fn encode_python_int<const BASE: u128>(obj: *mut PyObject, buffer: &mut Vec<u8>) {
    let mut overflow = 0;
    let longlong = PyLong_AsLongLongAndOverflow(obj, &mut overflow);

    if overflow == 0 {
        if longlong >= 0 {
            if longlong < ((NUMBER_BASE as u8) - AMOUNT_OF_USED_FLAGS) as i64 {
                buffer.push(AMOUNT_OF_USED_FLAGS + longlong as u8);
            } else {
                buffer.push(POSITIVE_INT_FLAG);
                encode_number::<BASE>(buffer, longlong as u128);
            }
        } else {
            buffer.push(NEGATIVE_INT_FLAG);
            encode_number::<BASE>(buffer, -longlong as u128);
        }
        return;
    }

    encode_pylong_big::<BASE>(buffer, obj);
}


#[inline(always)]
unsafe fn encode_pylong_big<const BASE: u128>(
    buf: &mut Vec<u8>,
    obj: *mut PyObject,
) {
    let is_negative = PyObject_RichCompareBool(obj, PyLong_FromLong(0), pyo3_ffi::Py_LT) == 1;

    let python_base_num = PyLong_FromLong(BASE as c_long);
    let obj = if is_negative {
        PyNumber_Add(obj, python_base_num)
    } else {
        PyNumber_Subtract(obj, python_base_num)
    };
    Py_DECREF(python_base_num);

    let nbits = _PyLong_NumBits(obj);
    let nbytes = (nbits + 7) / 8 + 1; // +1 to preserve sign bit

    let mut bytes = Vec::<u8>::with_capacity(nbytes);
    bytes.set_len(nbytes);

    // signed = 1 → two's complement
    let rc = _PyLong_AsByteArray(
        obj as *mut PyLongObject,
        bytes.as_mut_ptr(),
        nbytes,
        0, // big-endian
        1, // signed
    );
    if rc != 0 {
        PyErr_SetString(
            PyExc_RuntimeError,
            b"Failed to extract PyLong bytes\0".as_ptr() as _,
        );
        return;
    }

    // Determine sign from MSB
    // let is_negative = (bytes[0] & 0x80) != 0;

    buf.push(if is_negative {
        NEGATIVE_INT_FLAG
    } else {
        POSITIVE_INT_FLAG
    });

    if is_negative {
        twos_complement_inplace(&mut bytes);
    }

    encode_base_from_bytes::<BASE>(buf, &bytes);
}

#[inline(always)]
fn twos_complement_inplace(bytes: &mut [u8]) {
    // invert
    for b in bytes.iter_mut() {
        *b = !*b;
    }

    // add 1
    for b in bytes.iter_mut().rev() {
        let (v, carry) = b.overflowing_add(1);
        *b = v;
        if !carry {
            break;
        }
    }
}

#[inline(always)]
fn encode_base_from_bytes<const BASE: u128>(buf: &mut Vec<u8>, bytes: &[u8]) {
    // Working copy (big-endian base-256 number)
    let mut work = bytes.to_vec();

    buf.push(ENDING_FLAG);

    while !work.is_empty() {
        let mut carry: u32 = 0;

        for b in work.iter_mut() {
            let v = (carry << 8) | (*b as u32);
            *b = (v / (BASE as u32)) as u8;
            carry = v % (BASE as u32);
        }

        // carry is the remainder
        buf.push(carry as u8);

        // Trim leading zero bytes
        while !work.is_empty() && work[0] == 0 {
            work.remove(0);
        }
    }

    buf.push(ENDING_FLAG);
}
