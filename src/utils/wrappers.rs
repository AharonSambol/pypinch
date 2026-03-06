use pyo3_ffi::*;

#[inline(always)]
pub unsafe fn get_tuple_size(obj: *mut PyObject) -> isize {
    #[cfg(PyPy)]
    {
        PyTuple_Size(obj)
    }

    #[cfg(not(PyPy))]
    {
        PyTuple_GET_SIZE(obj)
    }
}

#[inline(always)]
pub unsafe fn get_list_size(obj: *mut PyObject) -> isize {
    #[cfg(PyPy)]
    {
        PyList_Size(obj)
    }

    #[cfg(not(PyPy))]
    {
        PyList_GET_SIZE(obj)
    }
}

#[inline(always)]
pub unsafe fn list_get_item(obj: *mut PyObject, i: Py_ssize_t) -> *mut PyObject {
    #[cfg(PyPy)]
    {
        PyList_GetItem(obj, i)
    }

    #[cfg(not(PyPy))]
    {
        PyList_GET_ITEM(obj, i)
    }
}

#[inline(always)]
pub unsafe fn tuple_get_item(obj: *mut PyObject, i: Py_ssize_t) -> *mut PyObject {
    #[cfg(PyPy)]
    {
        PyTuple_GetItem(obj, i)
    }

    #[cfg(not(PyPy))]
    {
        PyTuple_GET_ITEM(obj, i)
    }
}

#[inline(always)]
pub unsafe fn tuple_set_item(tuple: *mut PyObject, i: Py_ssize_t, obj: *mut PyObject) {
    #[cfg(PyPy)]
    {
        PyTuple_SetItem(tuple, i, obj);
    }

    #[cfg(not(PyPy))]
    {
        PyTuple_SET_ITEM(tuple, i, obj);
    }
}

#[inline(always)]
pub unsafe fn list_set_item(list: *mut PyObject, i: Py_ssize_t, obj: *mut PyObject) {
    #[cfg(PyPy)]
    {
        PyList_SetItem(list, i, obj);
    }

    #[cfg(not(PyPy))]
    {
        PyList_SET_ITEM(list, i, obj);
    }
}

#[inline(always)]
pub unsafe fn is_ascii(obj: *mut PyObject) -> bool {
    #[cfg(Py_3_14)]
    {
        false // no support for the macro anymore :(
    }

    #[cfg(not(Py_3_14))]
    {
        PyUnicode_IS_ASCII(obj) == 1
    }
}

#[inline(always)]
pub unsafe fn is_gc_enabled() -> bool {
    #[cfg(Py_3_10)]
    {
        PyGC_IsEnabled() == 1
    }

    #[cfg(not(Py_3_10))]
    {
        false // no support :(
    }
}

#[inline(always)]
pub unsafe fn gc_enabled() -> bool {
    #[cfg(Py_3_10)]
    {
        PyGC_Enable()
    }

    #[cfg(not(Py_3_10))]
    {} // no support :(
}

#[inline(always)]
pub unsafe fn gc_disable() -> bool {
    #[cfg(Py_3_10)]
    {
        PyGC_Disable()
    }

    #[cfg(not(Py_3_10))]
    {} // no support :(
}