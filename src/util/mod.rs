pub mod convert;
pub mod history;
#[cfg(target_arch = "wasm32")]
pub mod sleep;

#[macro_export]
macro_rules! unreachable_log {
    ($($arg:tt)*) => {{
        log::error!(
            "UNREACHABLE  {}:{}:{} - {}",
            file!(),
            line!(),
            column!(),
            format_args!($($arg)*)
        );
        unreachable!($($arg)*);
    }};
}
