use crate::raise_mem_error_if_null;
use crate::serializing::py_bytes_buffer::PyBytesBuffer;
use crate::serializing::utils::encode_number;
use crate::utils::consts::{AMOUNT_OF_USED_FLAGS, ENDING_FLAG, NEGATIVE_INT_FLAG, NUMBER_BASE, POSITIVE_INT_FLAG};
use pyo3_ffi::{PyLongObject, PyLong_AsLongLongAndOverflow, PyLong_FromLong, PyNumber_Add, PyNumber_Subtract, PyObject, PyObject_RichCompareBool, Py_DECREF, _PyLong_AsByteArray, _PyLong_NumBits};
use std::ffi::c_long;

pub fn encode_python_int<const BASE: u128>(obj: *mut PyObject, buffer: &mut PyBytesBuffer) -> Result<(), *mut PyObject> {
    let mut overflow = 0;
    let longlong = unsafe { PyLong_AsLongLongAndOverflow(obj, &mut overflow) };

    if overflow == 0 {
        return if longlong >= 0 {
            if longlong < ((NUMBER_BASE as u8) - AMOUNT_OF_USED_FLAGS) as i64 {
                buffer.push(AMOUNT_OF_USED_FLAGS + longlong as u8)
            } else {
                buffer.push(POSITIVE_INT_FLAG)?;
                // TODO: could technically subtract (NUMBER_BASE - AMOUNT_OF_USED_FLAGS) or serialize the number differently
                encode_number::<BASE>(buffer, longlong as u128)
            }
        } else {
            buffer.push(NEGATIVE_INT_FLAG)?;
            encode_number::<BASE>(buffer, -longlong as u128)
        };
    }

    encode_pylong_big::<BASE>(buffer, obj)
}


#[inline(always)]
fn encode_pylong_big<const BASE: u128>(
    buf: &mut PyBytesBuffer,
    obj: *mut PyObject,
) -> Result<(), *mut PyObject> {
    unsafe {
        let is_negative = PyObject_RichCompareBool(
            obj,
            raise_mem_error_if_null!(PyLong_FromLong(0)),
            pyo3_ffi::Py_LT
        ) == 1;

        let python_base_num = raise_mem_error_if_null!(PyLong_FromLong(BASE as c_long));
        let obj = raise_mem_error_if_null!(if is_negative {
        PyNumber_Add(obj, python_base_num)
    } else {
        PyNumber_Subtract(obj, python_base_num)
    });
        Py_DECREF(python_base_num);

        let nbits = _PyLong_NumBits(obj);
        let nbytes = (nbits + 7) / 8 + 1; // +1 to preserve sign bit

        let mut bytes = Vec::<u8>::with_capacity(nbytes);
        bytes.set_len(nbytes);

        // signed = 1 → two's complement
        _PyLong_AsByteArray(
            obj as *mut PyLongObject,
            bytes.as_mut_ptr(),
            nbytes,
            0, // big-endian
            1, // signed
        );

        // Determine sign from MSB
        // let is_negative = (bytes[0] & 0x80) != 0;

        buf.push(if is_negative {
            NEGATIVE_INT_FLAG
        } else {
            POSITIVE_INT_FLAG
        })?;

        if is_negative {
            twos_complement_inplace(&mut bytes);
        }

        encode_base_from_bytes::<BASE>(buf, &bytes)
    }
}

#[inline(always)]
fn twos_complement_inplace(bytes: &mut [u8]) {
    // invert
    for byte in bytes.iter_mut() {
        *byte = !*byte;
    }

    // add 1
    for byte in bytes.iter_mut().rev() {
        let (value, carry) = byte.overflowing_add(1);
        *byte = value;
        if !carry {
            break;
        }
    }
}

#[inline(always)]
fn encode_base_from_bytes<const BASE: u128>(buf: &mut PyBytesBuffer, bytes: &[u8]) -> Result<(), *mut PyObject> {
    // Working copy (big-endian base-256 number)
    let mut work = bytes.to_vec();

    buf.push(ENDING_FLAG)?;

    while !work.is_empty() {
        let mut carry: u32 = 0;

        for byte in work.iter_mut() {
            let byte_with_carry = (carry << 8) | (*byte as u32);
            *byte = (byte_with_carry / (BASE as u32)) as u8;
            carry = byte_with_carry % (BASE as u32);
        }

        // carry is the remainder
        buf.push(carry as u8)?;

        // Trim leading zero bytes
        while !work.is_empty() && work[0] == 0 {
            work.remove(0);
        }
    }

    buf.push(ENDING_FLAG)
}
