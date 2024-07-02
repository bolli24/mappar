#![allow(dead_code)]

pub use app::MyApp;

mod api;
mod app;
mod debug_text;
mod imagery;
mod imagery_fetcher;
mod map_data;
mod map_widget;
mod render;
mod skia;

#[cfg(test)]
static INIT: std::sync::Once = std::sync::Once::new();
#[cfg(test)]
pub fn init_test_logger() {
	INIT.call_once(|| {
		simple_logger::init().expect("Failed to initialize logger");
	});
}
