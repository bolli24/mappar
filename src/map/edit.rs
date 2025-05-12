use super::map_data::{ElementId, MapData, MapElement};
use crate::unreachable_log;
use glam::DVec2;
use indexmap::map::Entry;
use smallvec::SmallVec;
use std::collections::HashMap;

type Multiple = SmallVec<[ElementId; 4]>;

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
			let Some(element) = target.get_mut(element_id) else {
				unreachable_log!("Element {} must exist", element_id);
			};
			match &self.1 {
				Action::AddNewTag { key, value } | Action::AddTag { key, value } => {
					element.new_tag = None;
					match element.tags.entry(key.clone()) {
						// FIXME: handle error, especially for multiple elements
						Entry::Occupied(_) => return Err(EditError::KeyAlreadyExists(key.clone())),
						Entry::Vacant(entry) => {
							entry.insert(value.clone());
						}
					}
				}
				Action::OverrideTag { key, value, .. } => {
					element.tags.insert(key.clone(), value.clone());
				}
				Action::UpdateValue { key, value, .. } => {
					element.tags.insert(key.clone(), value.clone());
				}
				Action::RemoveTag { key, .. } => {
					element.tags.shift_remove(key);
				}
				Action::UpdateNewTag { current, .. } => {
					element.new_tag = Some(current.clone());
				}
				Action::Move(_, _) => {}
			}
		}

		if let Action::Move(delta, false) = &self.1 {
			MapData::raw_move_by(target, &self.0, *delta);
		}

		Ok(self.0.clone())
	}

	fn undo(&mut self, target: &mut Self::Target) -> Self::Output {
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
					element.new_tag = previous.clone();
				}
				Action::AddNewTag { key, value } => {
					element.tags.shift_remove(key);
					element.new_tag = Some((key.clone(), value.clone()))
				}
				Action::Move(_, _) => {}
			}
		}
		if let Action::Move(delta, _) = &self.1 {
			MapData::raw_move_by(target, &self.0, -*delta);
		}

		Ok(self.0.clone())
	}
}
