use egui::Color32;

use crate::css::Icon;

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Style {
	pub color: Color32,
	pub size: f32,
	pub icon: Option<Icon>,
}

impl Default for Style {
	fn default() -> Self {
		Self {
			color: Color32::LIGHT_RED,
			size: 2.0,
			icon: None,
		}
	}
}
