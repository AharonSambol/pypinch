#[macro_export] macro_rules! py_string {
    ($exp:expr) => {
        concat!($exp, "\0")
            .as_ptr()
            .cast::<std::os::raw::c_char>()
    };
}

#[macro_export] macro_rules! py_string_format {
    ($exp:expr) => {
        format!("{}\0", $exp)
            .as_ptr()
            .cast::<std::os::raw::c_char>()
    };
}

#[macro_export] macro_rules! safe_get {
    ($buf:expr, $idx:expr) => {
        {
            safe_get!($buf, $idx, crate::utils::consts::UNEXPECTED_END_OF_INPUT)
        }
    };
    ($buf:expr, $idx:expr, $reason:expr) => {
        {
            #[allow(unused_imports)]
            use crate::utils::py_helpers::ToPyErr;
            match $buf.get($idx) {
                Some(x) => x,
                None => {
                    return Err($reason.to_py_error(
                        crate::deserializing::utils::DESERIALIZATION_ERROR_TYPE
                    ))
                }
            }
        }
    }
}

#[macro_export] macro_rules! safe_new_py_list {
    ($length:expr, $use_tuples:expr) => {
        {
            let length = $length;
            let list = if $use_tuples { pyo3_ffi::PyTuple_New(length) } else { pyo3_ffi::PyList_New(length) };
            if list.is_null() {
                return Err(pyo3_ffi::PyErr_NoMemory());
            }
            list
        }
    }
}

#[macro_export] macro_rules! safe_new_py_dict {
    () => {
        {
            let dict = pyo3_ffi::PyDict_New();
            if dict.is_null() {
                return Err(pyo3_ffi::PyErr_NoMemory());
            }
            dict
        }
    }
}

#[macro_export] macro_rules! raise_mem_error_if_null {
    ($item:expr) => {
        {
            let item = $item;
            if item.is_null() {
                return Err(pyo3_ffi::PyErr_NoMemory());
            }
            item
        }
    }
}