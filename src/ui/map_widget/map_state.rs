use egui::Response;
use glam::DVec2;

use crate::ui::edit_bar::EditMode;

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum MapState {
	Default,
	// BoxSelect(DVec2),
	MoveView,
	MoveElements(DVec2),
}

impl Default for MapState {
	fn default() -> Self {
		Self::Default
	}
}

impl MapState {
	pub fn update(&mut self, _response: &Response, _edit_mode: &EditMode) {}
}
