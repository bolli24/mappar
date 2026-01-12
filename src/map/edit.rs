use super::map_data::MapData;
use crate::map::element::{Element, ElementId, ElementInfo, MapElement, Member, is_closed};
use crate::unreachable_log;
use glam::DVec2;
use indexmap::IndexMap;
use indexmap::map::Entry;
use smallvec::SmallVec;
use std::collections::HashMap;

pub type Multiple = SmallVec<[ElementId; 4]>;

// TODO: profile for ideal length of buffer
#[derive(Debug)]
pub struct Edit(pub Multiple, pub Action);

#[derive(Debug)]
pub enum Action {
	AddTag {
		key: String,
		value: String,
	},
	OverrideTag {
		key: String,
		value: String,
		previous_value: String,
	},
	UpdateValue {
		key: String,
		value: String,
		previous_value: String,
	},
	RemoveTag {
		key: String,
		value: String,
		index: usize,
	},
	UpdateNewTag {
		current: (String, String),
		previous: Option<(String, String)>,
	},
	AddNewTag {
		key: String,
		value: String,
	},
	Move(DVec2, bool),
	NewElement {
		element: Element,
	},
	AddNodeToWayEnd {
		node: ElementId,
	},
	AddNodeToWaySegments {
		node: Element,
		ways: SmallVec<[(ElementId, usize); 1]>,
	},
	Delete {
		elements: SmallVec<[MapElement; 1]>,
		references: HashMap<ElementId, Vec<(ElementId, Reference)>>,
	},
}

pub type References = HashMap<ElementId, Vec<(ElementId, Reference)>>;

#[derive(Debug)]
pub enum Reference {
	Path(usize, bool),
	Member(usize, String),
}

impl Reference {
	pub const fn get_index(&self) -> usize {
		match self {
			Reference::Path(index, _) | Reference::Member(index, _) => *index,
		}
	}
}

#[derive(thiserror::Error, Debug)]
pub enum EditError {
	#[error("Key '{0}' already exists.")]
	KeyAlreadyExists(String),
}

// TODO: Merging
impl undo::Edit for Edit {
	type Target = HashMap<ElementId, MapElement>;
	type Output = Result<Multiple, EditError>;

	fn edit(&mut self, target: &mut Self::Target) -> Self::Output {
		for element_id in &self.0 {
			match &self.1 {
				Action::AddNewTag { key, value } | Action::AddTag { key, value } => {
					let element = target.get_mut(element_id).expect("Element must exist");
					element.new_tag = None;
					match element.tags.entry(key.clone()) {
						// FIXME: handle error, especially for multiple elements
						Entry::Occupied(_) => return Err(EditError::KeyAlreadyExists(key.clone())),
						Entry::Vacant(entry) => {
							entry.insert(value.clone());
						}
					}
				}
				Action::OverrideTag { key, value, .. } | Action::UpdateValue { key, value, .. } => {
					let element = target.get_mut(element_id).expect("Element must exist");
					element.tags.insert(key.clone(), value.clone());
				}
				Action::RemoveTag { key, .. } => {
					let element = target.get_mut(element_id).expect("Element must exist");
					element.tags.shift_remove(key);
				}
				Action::UpdateNewTag { current, .. } => {
					let element = target.get_mut(element_id).expect("Element must exist");
					element.new_tag = Some(current.clone());
				}
				Action::Move(_, _) => {}
				Action::NewElement { element } => {
					target.insert(
						*element_id,
						MapElement {
							id: *element_id,
							element: element.clone(),
							tags: IndexMap::default(),
							info: ElementInfo::default(),
							new_tag: None,
						},
					);
				}
				Action::AddNodeToWayEnd { node } => {
					let element = target.get_mut(element_id).expect("Element must exist");
					let Element::Way { nodes } = &mut element.element else {
						unreachable_log!("Element must be way");
					};
					nodes.push(*node);
				}
				Action::AddNodeToWaySegments { node, ways } => {
					target.insert(
						*element_id,
						MapElement {
							id: *element_id,
							element: node.clone(),
							tags: IndexMap::default(),
							info: ElementInfo::default(),
							new_tag: None,
						},
					);

					for (id, index) in ways {
						let Element::Way { nodes } = &mut target.get_mut(id).expect("Way must exist.").element else {
							unreachable_log!("Element must be way");
						};

						nodes.insert(*index, *element_id);
					}
				}
				Action::Delete { .. } => { // Handled below
				}
			}
		}

		if let Action::Move(delta, false) = &self.1 {
			MapData::raw_move_by(target, &self.0, *delta);
		} else if let Action::Delete { elements, references } = &self.1 {
			for element in elements.iter().rev() {
				let removed = target.remove(&element.id);
				debug_assert!(removed.is_some());
			}

			for (parent, parent_refs) in references {
				// We remove nodes/members from the back
				// because removing elements moves everything after forward
				for (_, reference) in parent_refs.iter().rev() {
					match reference {
						Reference::Path(index, closed) => {
							let Element::Way { nodes } =
								&mut target.get_mut(parent).expect("Element must exist.").element
							else {
								unreachable!("Element must be way.");
							};
							nodes.remove(*index);
							if *index == 0 && *closed {
								let len = nodes.len();
								nodes[len - 1] = nodes[0];
							}
						}
						Reference::Member(index, _) => {
							let Element::Relation { members } =
								&mut target.get_mut(parent).expect("Element must exist.").element
							else {
								unreachable!("Element must be relation.");
							};
							members.remove(*index);
						}
					}
				}

				// Make 2-ways not closed
				if let Element::Way { nodes } = &mut target.get_mut(parent).expect("Element must exist.").element
					&& nodes.len() == 3
					&& is_closed(nodes)
				{
					nodes.remove(2);
				}
			}
		}

		Ok(self.0.clone())
	}

	fn undo(&mut self, target: &mut Self::Target) -> Self::Output {
		match &self.1 {
			Action::Move(delta, _) => {
				MapData::raw_move_by(target, &self.0, -*delta);
				return Ok(self.0.clone());
			}
			Action::Delete { elements, references } => {
				for element in elements {
					target.insert(element.id, (*element).clone());
				}

				for (parent, parent_refs) in references {
					// Make 2-ways that were closed before, closed again before adding the third node
					if let Element::Way { nodes } = &mut target.get_mut(parent).expect("Element must exist.").element
						&& nodes.len() == 2
						&& parent_refs
							.iter()
							.any(|(_, reference)| matches!(reference, Reference::Path(_, true)))
					{
						nodes.push(nodes[0]);
					}

					for (id, reference) in parent_refs {
						match reference {
							Reference::Path(index, closed) => {
								let Element::Way { nodes } =
									&mut target.get_mut(parent).expect("Element must exist.").element
								else {
									unreachable!("Element must be way.");
								};
								nodes.insert(*index, *id);
								if *index == 0 && *closed {
									let len = nodes.len();
									nodes[len - 1] = *id;
								}
							}
							Reference::Member(index, role) => {
								let Element::Relation { members } =
									&mut target.get_mut(parent).expect("Element must exist.").element
								else {
									unreachable!("Element must be relation.");
								};
								members.insert(
									*index,
									Member {
										ref_id: *id,
										role: role.clone(),
									},
								);
							}
						}
					}
				}
				return Ok(self.0.clone());
			}
			_ => {}
		}

		for element_id in &self.0 {
			let Some(element) = target.get_mut(element_id) else {
				unreachable_log!("Element {} must exist", element_id);
			};
			match &self.1 {
				Action::AddTag { key, .. } => {
					element.tags.shift_remove(key);
				}
				Action::OverrideTag {
					key, previous_value, ..
				}
				| Action::UpdateValue {
					key, previous_value, ..
				} => {
					element.tags.insert(key.clone(), previous_value.clone());
				}
				Action::RemoveTag { key, value, index } => {
					element.tags.insert_before(*index, key.clone(), value.clone());
				}
				Action::UpdateNewTag { previous, .. } => {
					element.new_tag.clone_from(previous);
				}
				Action::AddNewTag { key, value } => {
					element.tags.shift_remove(key);
					element.new_tag = Some((key.clone(), value.clone()));
				}
				Action::Move(_, _) => { // Handled above
				}
				Action::NewElement { .. } => {
					target.remove(element_id);
				}
				Action::AddNodeToWayEnd { .. } => {
					let Element::Way { nodes } = &mut element.element else {
						unreachable_log!("Element must be way");
					};
					nodes.pop();
				}
				Action::AddNodeToWaySegments { ways, .. } => {
					target.remove(element_id);

					for (id, index) in ways {
						let Element::Way { nodes } = &mut target.get_mut(id).expect("Way must exist.").element else {
							unreachable_log!("Element must be way");
						};

						nodes.remove(*index);
					}
				}
				Action::Delete { .. } => { // Handled above
				}
			}
		}

		Ok(self.0.clone())
	}
}
