use crate::map::element::{ElementId, ElementType};
use crate::map::map_data::MapData;
use egui::ahash::AHashSet;
use glam::DVec2;
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct MapMemory {
	pub zoom_level: f64,
	pub camera_pos: DVec2,
	pub selection: Selection,
	pub last_updated_styles: usize,
	pub highlighted_elements: AHashSet<ElementId>,
}

impl Default for MapMemory {
	fn default() -> Self {
		Self {
			zoom_level: 1.0,
			camera_pos: DVec2::ZERO,
			selection: Selection::default(),
			last_updated_styles: 0,
			highlighted_elements: AHashSet::new(),
		}
	}
}

#[derive(Debug, Clone, Default)]
pub struct Selection {
	pub elements: HashSet<ElementId>,
	pub tags: HashSet<usize>,
	pub relation: Option<ElementId>,
}

impl Selection {
	pub fn clear(&mut self, map_data: &mut MapData) {
		for element in self.elements.drain() {
			if let Some(element) = map_data.get_mut(&element) {
				element.new_tag = None;
			}
		}
		self.tags.clear();
	}
	#[must_use]
	pub fn node_count(&self) -> usize {
		self.elements
			.iter()
			.filter(|element_id| element_id.1 == ElementType::Node)
			.count()
	}
	#[must_use]
	pub fn way_count(&self) -> usize {
		self.elements
			.iter()
			.filter(|element_id| element_id.1 == ElementType::Way)
			.count()
	}
	#[must_use]
	pub fn relation_count(&self) -> usize {
		self.elements
			.iter()
			.filter(|element_id| element_id.1 == ElementType::Relation)
			.count()
	}
}
