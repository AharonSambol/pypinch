#[macro_export] macro_rules! py_string {
    ($exp:expr) => {
        concat!($exp, "\0")
            .as_ptr()
            .cast::<c_char>()
    };
}