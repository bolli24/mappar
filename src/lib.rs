#![allow(dead_code)]

pub use ui::app::MyApp;

pub mod imagery;

mod css;
mod map;
mod ui;
pub mod util;

#[cfg(test)]
static INIT: std::sync::Once = std::sync::Once::new();
#[cfg(test)]
pub fn init_test_logger() {
	use log::LevelFilter;
	use simplelog::{ColorChoice, Config, TermLogger, TerminalMode};

	INIT.call_once(|| {
		TermLogger::init(
			LevelFilter::Info,
			Config::default(),
			TerminalMode::Mixed,
			ColorChoice::Auto,
		)
		.expect("Failed to initialize logger");
	});
}
