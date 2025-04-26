use super::css::icon::Icon;
use egui::{Align2, Color32, FontFamily, FontId};
use strum::{EnumString, VariantNames};

#[derive(Debug, Clone, PartialEq)]
pub struct Style {
	pub color: Color32,
	pub size: f32,
	pub icon: Option<Icon>,
	pub icon_rotation: f32,
	pub text: Text,
}

impl Default for Style {
	fn default() -> Self {
		Self {
			color: Color32::LIGHT_RED,
			size: 2.0,
			icon: None,
			icon_rotation: 0.0,
			text: Text::default(),
		}
	}
}

#[derive(Debug, Clone, PartialEq)]
pub struct Text {
	pub text: String,
	pub color: Color32,
	pub offset: egui::Vec2,
	pub halo_alpha: f32,
	halo_color: Option<Color32>,
	pub anchor: Align2,
	// TODO: pub rotation: f32,
	pub position: TextPosition,
	pub font_family: FontFamily,
	pub font_size: f32,
	// TODO: pub font_weight        Can be added once egui supports font weight and style
	// TODO: pub font_style         (https://github.com/emilk/egui/pull/5784)
}

impl Text {
	pub fn new(text: impl Into<String>) -> Self {
		Self {
			text: text.into(),
			..Self::default()
		}
	}

	pub fn halo_color(&self) -> Option<Color32> {
		todo!()
	}

	pub fn font_id(&self) -> FontId {
		FontId::new(self.font_size, self.font_family.clone())
	}
}

impl Default for Text {
	fn default() -> Self {
		Self {
			text: String::new(),
			color: Color32::WHITE,
			offset: egui::vec2(0.0, -2.0),
			halo_alpha: 0.0,
			halo_color: None,
			anchor: Align2::CENTER_TOP,
			// rotation: 0.0,
			position: TextPosition::default(),
			font_family: FontFamily::Proportional,
			font_size: 8.0,
		}
	}
}

#[derive(Default, Debug, Copy, Clone, PartialEq, Eq, EnumString, VariantNames)]
#[strum(serialize_all = "snake_case")]
pub enum TextPosition {
	#[default]
	Center,
	Line,
}
