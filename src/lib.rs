#![allow(dead_code)]
extern crate alloc;

pub use map::element::Element;
pub use map::element::ElementId;
pub use map::element::ElementInfo;
pub use map::element::ElementType;
pub use map::element::MapElement;
pub use map::latlon::LatLon;
pub use ui::app::MyApp;

pub mod api;
pub mod imagery;
pub mod map;
pub mod ui;
pub mod util;

pub use ui::key_value_table::get_tags;

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
