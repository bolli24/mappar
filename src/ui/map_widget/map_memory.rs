use std::collections::HashSet;

use super::MapState;
use crate::map::map_data::ElementId;
use glam::DVec2;

#[derive(Debug, Clone)]
pub struct MapMemory {
	pub zoom_level: f64,
	pub camera_pos: DVec2,
	pub state: MapState,
	pub selection: Selection,
}

impl Default for MapMemory {
	fn default() -> Self {
		Self {
			zoom_level: 1.0,
			camera_pos: DVec2::ZERO,
			state: MapState::Default,
			selection: Selection::default(),
		}
	}
}

#[derive(Debug, Clone, Default)]
pub struct Selection {
	pub selected_elements: HashSet<ElementId>,
	pub selected_tags: HashSet<usize>,
	pub new_tag: Option<(String, String)>,
}

impl Selection {
	fn clear(&mut self) {
		self.selected_elements.clear();
		self.selected_tags.clear();
		self.new_tag = None;
	}
}
