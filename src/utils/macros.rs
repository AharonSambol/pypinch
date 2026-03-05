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
            #[allow(unused_imports)]
            use crate::utils::py_helpers::ToPyErr;
            match $buf.get($idx) {
                Some(x) => x,
                None => {
                    return Err(crate::utils::consts::UNEXPECTED_END_OF_INPUT.to_py_error(
                        crate::deserializing::utils::DESERIALIZATION_ERROR_TYPE
                    ))
                }
            }
        }
    }
}