use pyo3_ffi::{PyASCIIObject, PyObject, PyObject_Hash, PyUnicode_Compare};
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hash, Hasher};
use std::ops::Index;

#[derive(Copy, Clone)]
pub struct PyStringKey(*mut PyObject);

impl PyStringKey {
    #[inline(always)]
    pub fn new(obj: *mut PyObject) -> PyStringKey {
        // compute the hash once, so that it will be saved in python's cache
        unsafe { PyObject_Hash(obj) };
        PyStringKey(obj)
    }
}
impl Hash for PyStringKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        unsafe {
            // the hash was already computed in the constructor so it will be saved here
            let hash = (*(self.0 as *mut PyASCIIObject)).hash;
            state.write_isize(hash);
        }
    }
}

impl PartialEq for PyStringKey {
    fn eq(&self, other: &Self) -> bool {
        if self.0 == other.0 { return true; }

        unsafe {
            let a = self.0 as *mut PyASCIIObject;
            let b = other.0 as *mut PyASCIIObject;

            if (*a).hash != (*b).hash || (*a).length != (*b).length {
                return false;
            }

            // If both are 1-byte strings, use raw memory comparison (SIMD optimized)
            if (*a).kind() == 1 && (*b).kind() == 1 {
                let len = (*a).length as usize;
                let a_data = a.add(1) as *const u8;
                let b_data = b.add(1) as *const u8;

                return std::slice::from_raw_parts(a_data, len) ==
                    std::slice::from_raw_parts(b_data, len);
            }

            PyUnicode_Compare(self.0, other.0) == 0
        }
    }
}

impl Eq for PyStringKey {}


#[derive(Default)]
pub struct PassThroughHasher(u64);

impl Hasher for PassThroughHasher {
    fn finish(&self) -> u64 { self.0 }
    fn write(&mut self, _bytes: &[u8]) { unreachable!() }
    fn write_isize(&mut self, i: isize) { self.0 = i as u64; }
}

pub struct Pointers {
    map: HashMap<PyStringKey, usize, BuildHasherDefault<PassThroughHasher>>,
}

impl Pointers {
    pub fn new() -> Self {
        Pointers {
            map: HashMap::default(),
        }
    }

    #[inline(always)]
    pub fn entry(&mut self, obj: *mut PyObject) -> Entry<'_, PyStringKey, usize> {
        self.map.entry(PyStringKey::new(obj))
    }

    #[inline(always)]
    pub fn insert(&mut self, obj: *mut PyObject, new_id: usize) -> Option<usize> {
        self.map.insert(PyStringKey::new(obj), new_id)
    }

    #[inline(always)]
    pub fn contains_key(&self, obj: *mut PyObject) -> bool {
        self.map.contains_key(&PyStringKey(obj))
    }

    #[inline(always)]
    pub fn index(&self, obj: *mut PyObject) -> &usize {
        self.map.index(&PyStringKey::new(obj))
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        self.map.len()
    }
}