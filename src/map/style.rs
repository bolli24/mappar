use egui::Color32;

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Style {
	pub color: Color32,
	pub size: f32,
	pub highlight_color: Option<Color32>,
}

impl Style {
	pub fn highlight(&self, highlight: bool) -> (Color32, f32) {
		if highlight {
			(self.highlight_color.unwrap_or(self.color), self.size)
		} else {
			(self.color, self.size)
		}
	}
}

impl Default for Style {
	fn default() -> Self {
		Self {
			color: Color32::LIGHT_RED,
			size: 2.0,
			highlight_color: None,
		}
	}
}
