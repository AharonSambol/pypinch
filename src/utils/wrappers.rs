use pyo3_ffi::*;

#[inline(always)]
pub fn get_tuple_size(obj: *mut PyObject) -> isize {
    #[cfg(PyPy)]
    unsafe {
        PyTuple_Size(obj)
    }

    #[cfg(not(PyPy))]
    unsafe {
        PyTuple_GET_SIZE(obj)
    }
}

#[inline(always)]
pub fn get_list_size(obj: *mut PyObject) -> isize {
    #[cfg(PyPy)]
    unsafe {
        PyList_Size(obj)
    }

    #[cfg(not(PyPy))]
    unsafe {
        PyList_GET_SIZE(obj)
    }
}

#[inline(always)]
pub fn list_get_item(obj: *mut PyObject, i: Py_ssize_t) -> *mut PyObject {
    #[cfg(PyPy)]
    unsafe {
        PyList_GetItem(obj, i)
    }

    #[cfg(not(PyPy))]
    unsafe {
        PyList_GET_ITEM(obj, i)
    }
}

#[inline(always)]
pub fn tuple_get_item(obj: *mut PyObject, i: Py_ssize_t) -> *mut PyObject {
    #[cfg(PyPy)]
    unsafe {
        PyTuple_GetItem(obj, i)
    }

    #[cfg(not(PyPy))]
    unsafe {
        PyTuple_GET_ITEM(obj, i)
    }
}

#[inline(always)]
pub fn tuple_set_item(tuple: *mut PyObject, i: Py_ssize_t, obj: *mut PyObject) {
    #[cfg(PyPy)]
    unsafe {
        PyTuple_SetItem(tuple, i, obj);
    }

    #[cfg(not(PyPy))]
    unsafe {
        PyTuple_SET_ITEM(tuple, i, obj);
    }
}

#[inline(always)]
pub fn list_set_item(list: *mut PyObject, i: Py_ssize_t, obj: *mut PyObject) {
    #[cfg(PyPy)]
    unsafe {
        PyList_SetItem(list, i, obj);
    }

    #[cfg(not(PyPy))]
    unsafe {
        PyList_SET_ITEM(list, i, obj);
    }
}

#[inline(always)]
pub fn is_ascii(obj: *mut PyObject) -> bool {
    #[cfg(Py_3_14)]
    unsafe {
        false // no support for the macro anymore :(
    }

    #[cfg(not(Py_3_14))]
    unsafe {
        PyUnicode_IS_ASCII(obj) == 1
    }
}

#[inline(always)]
pub fn is_gc_enabled() -> bool {
    #[cfg(Py_3_10)]
    unsafe {
        PyGC_IsEnabled() == 1
    }

    #[cfg(not(Py_3_10))]
    unsafe {
        false // no support :(
    }
}

#[inline(always)]
pub fn gc_enabled() {
    #[cfg(Py_3_10)]
    unsafe {
        PyGC_Enable();
    }

    #[cfg(not(Py_3_10))]
    {} // no support :(
}

#[inline(always)]
pub fn gc_disable() {
    #[cfg(Py_3_10)]
    unsafe {
        PyGC_Disable();
    }

    #[cfg(not(Py_3_10))]
    {} // no support :(
}

#[inline(always)]
pub fn py_unicode_data(obj: *mut PyObject) -> *const u8 {
    #[cfg(PyPy)]
    unsafe {
        // TODO: maybe can use PyUnicode_READ & PyUnicode_WRITE
        PyUnicode_AsUTF8(obj) as *const u8
    }

    #[cfg(not(PyPy))]
    unsafe {
        PyUnicode_DATA(obj) as *const u8
    }
}
