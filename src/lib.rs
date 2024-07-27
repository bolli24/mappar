#![allow(dead_code)]
pub use ui::app::MyApp;

pub mod imagery;

mod map;
mod ui;
pub mod util;

#[cfg(test)]
static INIT: std::sync::Once = std::sync::Once::new();
#[cfg(test)]
pub fn init_test_logger() {
	INIT.call_once(|| {
		simple_logger::init().expect("Failed to initialize logger");
	});
}
