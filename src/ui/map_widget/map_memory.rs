use std::collections::HashSet;

use super::MapState;
use crate::map::map_data::{ElementId, MapData};
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
}

impl Selection {
	pub fn clear(&mut self, map_data: &mut MapData) {
		for element in self.selected_elements.drain() {
			map_data.get_mut(&element).unwrap().new_tag = None;
		}
		self.selected_tags.clear();
	}
}
