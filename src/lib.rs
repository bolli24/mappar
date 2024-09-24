#![allow(dead_code)]

use log::LevelFilter;
use simplelog::{ColorChoice, Config, TermLogger, TerminalMode};
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
		TermLogger::init(
			LevelFilter::Info,
			Config::default(),
			TerminalMode::Mixed,
			ColorChoice::Auto,
		).expect("Failed to initialize logger");
	});
}
