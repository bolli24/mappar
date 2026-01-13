use super::edit::{Action, Edit, Multiple, Reference, References};
use crate::map::element::{self, is_closed};
use crate::map::element::{Element, ElementId, ElementType, MapElement, Member};
use crate::map::latlon::LatLon;
use glam::DVec2;
use smallvec::{SmallVec, smallvec};
use std::cell::Cell;
use std::collections::{HashMap, HashSet};
use undo::Record;

pub type Elements = HashMap<ElementId, MapElement>;

pub type EditHistory = Record<Edit>;

pub struct MapData {
	pub elements: Elements,
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
			last_new_ids: (0, 0, 0),
		}
	}

	pub fn add(&mut self, element: MapElement) {
		self.elements.insert(element.id, element);
	}

	pub fn remove(&mut self, id: &ElementId) -> Option<MapElement> {
		self.elements.remove(id)
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

	pub fn raw_move_by<'a>(&mut self, ids: impl IntoIterator<Item = &'a ElementId>, delta: DVec2) {
		let mut moved = HashSet::with_capacity(100);

		for element_id in ids {
			Self::move_element(&mut self.elements, &mut moved, element_id, &delta);
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
	pub fn delete(&mut self, delete_ids: &HashSet<ElementId>) -> Edit {
		let mut ids: Multiple = delete_ids.iter().copied().collect();
		let mut references = HashMap::new();
		let mut elements: SmallVec<[MapElement; 1]> = ids
			.iter()
			.map(|id| {
				self.get_references(&mut references, *id);
				self.elements[id].clone()
			})
			.collect();

		// We also want to delete all the ways that become length 0 or 1 after deleting some of its nodes
		let invalid_ways: Vec<_> = references
			.iter()
			.filter_map(|(&parent_id, ref_set)| {
				let element = self.get(&parent_id).expect("Element must exist");
				match &element.element {
					Element::Way { nodes } if nodes.len() <= ref_set.len() + 1 => Some(parent_id),
					_ => None,
				}
			})
			.collect();

		for id in &invalid_ways {
			// Remove the reference set for this way since it's being deleted
			references.remove(id);
			if !delete_ids.contains(id) {
				ids.push(*id);
				self.get_references(&mut references, *id);
				elements.push(self.elements[id].clone());
			}
		}

		for ref_set in references.values_mut() {
			ref_set.sort_by_key(|r| r.1.get_index());
		}

		Edit(ids, Action::Delete { elements, references })
	}

	fn get_references(&self, references: &mut References, id: ElementId) {
		puffin::profile_function!();

		for other in self.elements.values() {
			match &other.element {
				Element::Way { nodes } if id.1 == ElementType::Node => {
					let closed = is_closed(nodes);
					for (index, node) in
						nodes
							.iter()
							.enumerate()
							.take(if closed { nodes.len() - 1 } else { nodes.len() })
					{
						if *node == id {
							references
								.entry(other.id)
								.or_default()
								.push((id, Reference::Path(index, closed)));
						}
					}
				}
				Element::Relation { members } => {
					for (index, Member { ref_id, role }) in members.iter().enumerate() {
						if *ref_id == id {
							references
								.entry(other.id)
								.or_default()
								.push((id, Reference::Member(index, role.clone())));
						}
					}
				}
				_ => {}
			}
		}
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
