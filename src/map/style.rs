use egui::Color32;

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Style {
	pub color: Color32,
	pub size: f32,
	pub highlight_color: Color32,
	pub highlight_size: f32,
}

impl Style {
	pub fn highlight(&self, highlight: bool) -> (Color32, f32) {
		if highlight {
			(self.highlight_color, self.highlight_size)
		} else {
			(self.color, self.size)
		}
	}
}

pub const DEFAULT_WAY_STYLE: Style = Style {
	color: Color32::BLACK,
	size: 1.5,
	highlight_color: Color32::DARK_GRAY,
	highlight_size: 2.0,
};

pub const DEFAULT_NODE_STYLE: Style = Style {
	color: Color32::BLACK,
	size: 4.0,
	highlight_color: Color32::DARK_GRAY,
	highlight_size: 5.0,
};
