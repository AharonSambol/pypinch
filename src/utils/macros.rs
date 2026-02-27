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