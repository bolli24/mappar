use std::collections::HashSet;

use super::MapState;
use crate::map::map_data::ElementId;
use glam::{dvec2, DVec2};

#[derive(Debug, Clone)]
pub struct MapMemory {
	pub zoom_level: f64,
	pub camera_pos: DVec2,
	pub state: MapState,
	pub selected: HashSet<ElementId>,
}

impl Default for MapMemory {
	fn default() -> Self {
		Self {
			zoom_level: 1.0,
			camera_pos: dvec2(0.0, 0.0),
			state: MapState::Default,
			selected: HashSet::new(),
		}
	}
}
