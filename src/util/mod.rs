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

#[macro_export]
macro_rules! spawn_task {
	($closure:expr) => {{
		#[cfg(not(target_arch = "wasm32"))]
		tokio::spawn($closure);

		#[cfg(target_arch = "wasm32")]
		wasm_bindgen_futures::spawn_local($closure);
	}};
}

#[macro_export]
macro_rules! message_error {
    ($messages:expr, $($arg:tt)*) => {
        {
            let message = format!($($arg)*);
            $messages.add(&message);
            log::error!("{}", message);
        }
    };
}

#[macro_export]
macro_rules! message_warn {
    ($messages:expr, $($arg:tt)*) => {
        {
            let message = format!($($arg)*);
            $messages.add(&message);
            log::warn!("{}", message);
        }
    };
}

#[macro_export]
macro_rules! message_info {
    ($messages:expr, $($arg:tt)*) => {
        {
            let message = format!($($arg)*);
            $messages.add(&message);
            log::info!("{}", message);
        }
    };
}

#[macro_export]
macro_rules! message_debug {
    ($messages:expr, $($arg:tt)*) => {
        {
            let message = format!($($arg)*);
            $messages.add(&message);
            log::debug!("{}", message);
        }
    };
}

#[macro_export]
macro_rules! message_trace {
    ($messages:expr, $($arg:tt)*) => {
        {
            let message = format!($($arg)*);
            $messages.add(&message);
            log::trace!("{}", message);
        }
    };
}
