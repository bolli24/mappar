use egui::Color32;

use crate::css::Icon;

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Style {
	pub color: Color32,
	pub size: f32,
	pub highlight_color: Option<Color32>,
	pub icon: Option<Icon>,
}

impl Style {
	pub fn hovered(&self) -> Self {
		Self {
			size: self.size + 2.0,
			color: Color32::RED.gamma_multiply(0.75),
			..*self
		}
	}
}

impl Default for Style {
	fn default() -> Self {
		Self {
			color: Color32::LIGHT_RED,
			size: 2.0,
			highlight_color: None,
			icon: None,
		}
	}
}
