use super::map_data::{ElementId, MapElement};
use crate::unreachable_log;
use indexmap::map::Entry;
use std::collections::HashMap;

// TODO: allow editing of multiple elements, use ArrayVec
#[derive(Debug)]
pub struct Edit(pub ElementId, pub Action);

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
}

#[derive(thiserror::Error, Debug)]
pub enum EditError {
	#[error("Key '{0}' already exists.")]
	KeyAlreadyExists(String),
}

// TODO: Merging
impl undo::Edit for Edit {
	type Target = HashMap<ElementId, MapElement>;
	type Output = Result<ElementId, EditError>;

	fn edit(&mut self, target: &mut Self::Target) -> Self::Output {
		let Some(element) = target.get_mut(&self.0) else {
			unreachable_log!("Element {} must exist", self.0);
		};
		match &self.1 {
			Action::AddTag { key, value } => match element.tags.entry(key.clone()) {
				Entry::Occupied(_) => return Err(EditError::KeyAlreadyExists(key.clone())),
				Entry::Vacant(entry) => {
					entry.insert(value.clone());
				}
			},
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
			Action::AddNewTag { key, value } => {
				element.new_tag = None;
				return Self(
					self.0,
					Action::AddTag {
						key: key.clone(),
						value: value.clone(),
					},
				)
				.edit(target);
			}
		}
		Ok(self.0)
	}

	fn undo(&mut self, target: &mut Self::Target) -> Self::Output {
		let Some(element) = target.get_mut(&self.0) else {
			unreachable_log!("Element must exist");
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
		}
		Ok(self.0)
	}
}
