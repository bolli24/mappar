use super::edit::{Action, Edit};
use crate::map::element;
use crate::map::element::{Element, ElementId, ElementType, MapElement};
use crate::map::latlon::LatLon;
use glam::DVec2;
use smallvec::{SmallVec, smallvec};
use std::cell::Cell;
use std::collections::{HashMap, HashSet};
use undo::Record;

pub type Elements = HashMap<ElementId, MapElement>;

pub struct MapData {
	pub elements: Elements,
	history: Record<Edit>,
	last_new_ids: (i64, i64, i64),
}

impl Default for MapData {
	fn default() -> Self {
		Self::new()
	}
}

impl MapData {
	#[must_use]
	pub fn new() -> Self {
		Self {
			elements: HashMap::new(),
			history: Record::new(),
			last_new_ids: (0, 0, 0),
		}
	}

	pub fn add(&mut self, element: MapElement) {
		self.elements.insert(element.id, element);
	}
	#[must_use]
	pub fn get(&self, id: &ElementId) -> Option<&MapElement> {
		self.elements.get(id)
	}

	pub fn get_mut(&mut self, id: &ElementId) -> Option<&mut MapElement> {
		self.elements.get_mut(id)
	}
	#[must_use]
	pub fn get_node_pos(&self, id: ElementId) -> Option<LatLon> {
		let e = self.get(&id)?;
		if let Element::Node { pos } = &e.element {
			return Some(pos.get());
		}
		None
	}

	pub fn iter(&self) -> impl Iterator<Item = &MapElement> {
		self.elements.values()
	}
	#[must_use]
	pub fn len(&self) -> usize {
		self.elements.len()
	}

	#[must_use]
	pub fn is_empty(&self) -> bool {
		self.len() == 0
	}

	#[must_use]
	pub fn history_len(&self) -> usize {
		self.history.len()
	}

	pub fn edit(&mut self, edit: Edit) -> <Edit as undo::Edit>::Output {
		self.history.edit(&mut self.elements, edit)
	}

	pub fn undo(&mut self) -> Option<<Edit as undo::Edit>::Output> {
		self.history.undo(&mut self.elements)
	}

	pub fn redo(&mut self) -> Option<<Edit as undo::Edit>::Output> {
		self.history.undo(&mut self.elements)
	}

	#[must_use]
	pub fn add_tag(&self, id: ElementId, key: String, value: String) -> Edit {
		Edit(smallvec![id], Action::AddTag { key, value })
	}
	#[must_use]
	pub fn add_new_tag(&self, id: ElementId) -> Edit {
		let (key, value) = self
			.get(&id)
			.expect("Element must exist")
			.new_tag
			.clone()
			.expect("New tag must have value.");
		Edit(smallvec![id], Action::AddNewTag { key, value })
	}
	#[must_use]
	pub fn override_tag(&self, id: ElementId, key: String, value: String) -> Edit {
		Edit(
			smallvec![id],
			Action::OverrideTag {
				previous_value: self.get_value(&id, &key),
				key,
				value,
			},
		)
	}
	#[must_use]
	pub fn update_value(&self, id: ElementId, key: String, value: String) -> Edit {
		Edit(
			smallvec![id],
			Action::UpdateValue {
				previous_value: self.get_value(&id, &key),
				key,
				value,
			},
		)
	}
	#[must_use]
	pub fn remove_tag(&self, id: ElementId, key: String) -> Edit {
		let Some(element) = self.get(&id) else {
			unreachable!("Element {id} must exist.");
		};
		let Some(index) = element.tags.get_index_of(&key) else {
			unreachable!("Element {id} must have key '{key}' to be overwritten.");
		};
		let value = element
			.tags
			.get_index(index)
			.expect("Must have tag at index.")
			.1
			.clone();
		Edit(smallvec![id], Action::RemoveTag { key, value, index })
	}
	#[must_use]
	pub fn update_new_tag(&self, id: ElementId, key: String, value: String) -> Edit {
		let Some(element) = self.get(&id) else {
			unreachable!("Element {id} must exist.");
		};
		Edit(
			smallvec![id],
			Action::UpdateNewTag {
				current: (key, value),
				previous: element.new_tag.clone(),
			},
		)
	}

	pub fn raw_move_by<'a>(elements: &mut Elements, ids: impl IntoIterator<Item = &'a ElementId>, delta: DVec2) {
		let mut moved = HashSet::with_capacity(100);

		for element_id in ids {
			Self::move_element(elements, &mut moved, element_id, &delta);
		}
	}
	#[must_use]
	pub fn move_by<'a>(&self, ids: impl IntoIterator<Item = &'a ElementId>, delta: DVec2, already_moved: bool) -> Edit {
		Edit(ids.into_iter().copied().collect(), Action::Move(delta, already_moved))
	}

	fn move_element(elements: &mut Elements, moved: &mut HashSet<ElementId>, element_id: &ElementId, delta: &DVec2) {
		if !moved.insert(*element_id) {
			return;
		}
		let element = elements.get(element_id).expect("Element must exist");
		match &element.element {
			Element::Node { pos } => {
				pos.set(pos.get() + *delta);
			}
			Element::Way { nodes } => {
				debug_assert!(
					nodes.len() >= 2 && !element::is_closed(nodes) || nodes.len() >= 3,
					"Way must be proper shape."
				);
				for node in nodes.clone() {
					Self::move_element(elements, moved, &node, delta);
				}
			}
			Element::Relation { .. } => {
				todo!("Moving relations.")
				// TODO: Do I even have to do this?
			}
		}
	}
	#[must_use]
	pub fn new_node(&mut self, pos: DVec2) -> Edit {
		let id = self.new_element_id(ElementType::Node);
		Edit(
			smallvec![id],
			Action::NewElement {
				element: Element::Node {
					pos: Cell::new(LatLon::from_web_mercator(pos)),
				},
			},
		)
	}
	#[must_use]
	pub fn new_way(&mut self, nodes: Vec<ElementId>) -> Edit {
		let id = self.new_element_id(ElementType::Way);
		Edit(
			smallvec![id],
			Action::NewElement {
				element: Element::Way { nodes },
			},
		)
	}
	#[must_use]
	pub fn add_node_to_way_end(&self, way: ElementId, node: ElementId) -> Edit {
		debug_assert_eq!(ElementType::Way, way.1);
		debug_assert_eq!(ElementType::Node, node.1);
		Edit(smallvec![way], Action::AddNodeToWayEnd { node })
	}
	#[must_use]
	pub fn add_node_on_way_segments(&mut self, pos: DVec2, ways: SmallVec<[(ElementId, usize); 1]>) -> Edit {
		let node_id = self.new_element_id(ElementType::Node);
		Edit(
			smallvec![node_id], // Should this include the id of the edited ways???
			Action::AddNodeToWaySegments {
				node: Element::Node {
					pos: Cell::new(LatLon::from_web_mercator(pos)),
				},
				ways,
			},
		)
	}
	#[must_use]
	const fn new_element_id(&mut self, e_type: ElementType) -> ElementId {
		let id = match e_type {
			ElementType::Node => {
				self.last_new_ids.0 -= 1;
				self.last_new_ids.0
			}
			ElementType::Way => {
				self.last_new_ids.1 -= 1;
				self.last_new_ids.1
			}
			ElementType::Relation => {
				self.last_new_ids.2 -= 1;
				self.last_new_ids.2
			}
		};
		ElementId(id, e_type)
	}

	/// # Safety:
	/// Panics if element or key does not exist.
	fn get_value(&self, id: &ElementId, key: &String) -> String {
		let Some(element) = self.get(id) else {
			unreachable!("Element {id} must exist.");
		};
		let Some(previous_value) = element.tags.get(key) else {
			unreachable!("Element {id} must have key '{key}' to be overwritten.");
		};
		previous_value.clone()
	}
}
