use crate::deserializing::pointer_holders::pointer_holder::PointerHolder;
use crate::deserializing::pointer_holders::position_pointer_holder::Pointer::{Position, Str};
use crate::deserializing::primitives::decode_string_without_inserting_pointer;
use crate::safe_get;
use crate::utils::consts::{MIGHT_BE_ASCII, NUMBER_BASE};
use pyo3_ffi::{PyObject, Py_DECREF, Py_INCREF};

enum Pointer {
    Str(*mut PyObject),
    Position {
        pos: usize,
        is_base_254: bool,
    },
}

pub struct PositionPointerHolder<'a> {
    buf: &'a [u8],
    str_posses: Vec<Pointer>,
}

impl<'a> PositionPointerHolder<'a> {
    pub fn new(buf: &'a [u8]) -> Self {
        PositionPointerHolder {
            buf, str_posses: Vec::new(),
        }
    }
}

impl PointerHolder for PositionPointerHolder<'_> {
    fn safe_get(&self, position: usize) -> Result<*mut PyObject, *mut PyObject> {
        match *safe_get!(self.str_posses, position) {
            Str(str) => {
                unsafe { Py_INCREF(str); }
                Ok(str)
            },
            Position { pos, is_base_254 }  => {
                let mut position = pos;
                return if is_base_254 {
                    decode_string_without_inserting_pointer::<MIGHT_BE_ASCII, { NUMBER_BASE - 1 }>(self.buf, &mut position)
                } else {
                    decode_string_without_inserting_pointer::<MIGHT_BE_ASCII, NUMBER_BASE>(self.buf, &mut position)
                }
            },
        }
    }

    fn insert(&mut self, object: *mut PyObject) {
        unsafe { Py_INCREF(object) };
        self.str_posses.push(Str(object));
    }
}

impl PositionPointerHolder<'_> {
    pub fn insert_position(&mut self, position: usize, is_base_254: bool) {
        self.str_posses.push(Position { pos: position, is_base_254 });
    }
}

impl Drop for PositionPointerHolder<'_> {
    fn drop(&mut self) {
        for item in self.str_posses.iter() {
            match item {
                Position { .. } => {},
                Str(str) => {
                    unsafe { Py_DECREF(*str); }
                }
            }
        }
    }
}