use pyo3_ffi::{_PyLong_AsByteArray, _PyLong_NumBits, PyErr_SetString, PyExc_RuntimeError, PyLong_AsLongLongAndOverflow, PyLongObject, PyObject};
use crate::serializing::utils::encode_number;
use crate::utils::consts::{ENDING_FLAG, NEGATIVE_INT_FLAG, POSITIVE_INT_FLAG, small_int};

pub unsafe fn encode_python_int(obj: *mut PyObject, buffer: &mut Vec<u8>, base: u8) {
    let mut overflow = 0;
    let longlong = PyLong_AsLongLongAndOverflow(obj, &mut overflow);

    if overflow == 0 {
        if let Some(byte) = small_int(longlong) {
            buffer.push(byte);
        } else if longlong >= 0 {
            buffer.push(POSITIVE_INT_FLAG);
            encode_number(buffer, longlong as u128, base);
        } else {
            buffer.push(NEGATIVE_INT_FLAG);
            encode_number(buffer, (-longlong) as u128, base);
        }
        return;
    }

    // Huge integer path
    encode_pylong_big(buffer, obj, base as u32);
}


#[inline(always)]
unsafe fn encode_pylong_big(
    buf: &mut Vec<u8>,
    obj: *mut PyObject,
    base: u32
) {
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
    let is_negative = (bytes[0] & 0x80) != 0;

    buf.push(if is_negative {
        NEGATIVE_INT_FLAG
    } else {
        POSITIVE_INT_FLAG
    });

    // If negative, convert from two's complement to magnitude
    if is_negative {
        twos_complement_inplace(&mut bytes);
    }

    encode_base_from_bytes(buf, &bytes, base);
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
fn encode_base_from_bytes(buf: &mut Vec<u8>, bytes: &[u8], base: u32) {
    // Working copy (big-endian base-256 number)
    let mut work = bytes.to_vec();

    buf.push(ENDING_FLAG);

    while !work.is_empty() {
        let mut carry: u32 = 0;

        for b in work.iter_mut() {
            let v = (carry << 8) | (*b as u32);
            *b = (v / base) as u8;
            carry = v % base;
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
