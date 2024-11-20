use super::super::memory::Memory;
use super::MapState;
use glam::{dvec2, DVec2};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Copy, Clone, mappar_macros::Memory)]
pub struct MapMemory {
	pub zoom_level: f64,
	pub camera_pos: DVec2,
	pub state: MapState,
}

impl Default for MapMemory {
	fn default() -> Self {
		Self {
			zoom_level: 1.0,
			camera_pos: dvec2(0.0, 0.0),
			state: MapState::Default,
		}
	}
}
