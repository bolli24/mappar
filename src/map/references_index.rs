use crate::{Element, ElementId, MapElement};
use egui::ahash::{AHashMap, AHashSet};

/// Bidirectional reference index for tracking element relationships
pub struct ReferenceIndex {
	/// Back references: child_id -> set of parent_ids that reference this element
	back_references: AHashMap<ElementId, AHashSet<ElementId>>,

	/// Forward references: parent_id -> set of child_ids that this element references
	forward_references: AHashMap<ElementId, AHashSet<ElementId>>,
}

impl ReferenceIndex {
	pub fn new() -> Self {
		Self {
			back_references: AHashMap::new(),
			forward_references: AHashMap::new(),
		}
	}

	pub fn index_element(&mut self, element: &MapElement) {
		match &element.element {
			Element::Node { .. } => {}
			Element::Way { nodes } => {
				self.add_references(element.id, nodes);
			}
			Element::Relation { members } => self.add_references(element.id, members.iter().map(|m| &m.ref_id)),
		};
	}

	pub fn add_references<'a>(&mut self, parent: ElementId, children: impl IntoIterator<Item = &'a ElementId>) {
		let forward_references = self.forward_references.entry(parent).or_default();
		for id in children {
			self.back_references.entry(*id).or_default().insert(parent);
			forward_references.insert(*id);
		}
	}

	#[inline]
	pub fn add_reference(&mut self, parent: ElementId, child: ElementId) {
		self.forward_references.entry(parent).or_default().insert(child);
		self.back_references.entry(child).or_default().insert(parent);
	}

	pub fn deindex_element(&mut self, element_id: ElementId) {
		// Remove all forward references from this element
		if let Some(forward_refs) = self.forward_references.remove(&element_id) {
			for child in forward_refs {
				if let Some(parents) = self.back_references.get_mut(&child) {
					parents.remove(&element_id);
					if parents.is_empty() {
						self.back_references.remove(&child);
					}
				}
			}
		}

		// Remove all back references to this element
		if let Some(back_refs) = self.back_references.remove(&element_id) {
			for parent in back_refs {
				if let Some(children) = self.forward_references.get_mut(&parent) {
					children.remove(&element_id);
					if children.is_empty() {
						self.forward_references.remove(&parent);
					}
				}
			}
		}
	}

	#[inline]
	pub fn remove_reference(&mut self, parent: ElementId, child: ElementId) {
		self.forward_references
			.get_mut(&parent)
			.map(|ref_set| ref_set.remove(&child));
		self.back_references
			.get_mut(&child)
			.map(|ref_set| ref_set.remove(&parent));
	}

	pub fn parents(&self, element: &ElementId) -> impl Iterator<Item = &ElementId> {
		self.back_references.get(element).into_iter().flatten()
	}

	pub fn parent_count(&self, element: &ElementId) -> usize {
		self.back_references.get(element).map_or(0, |set| set.len())
	}

	pub fn children(&self, element: &ElementId) -> impl Iterator<Item = &ElementId> {
		self.forward_references.get(element).into_iter().flatten()
	}

	pub fn children_count(&self, element: &ElementId) -> usize {
		self.forward_references.get(element).map_or(0, |set| set.len())
	}
}
