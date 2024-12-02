use undo::Edit;

use super::map_data::{ElementId, MapData};

pub struct UpdateData(pub ElementId, pub Action);

pub enum Action {
	AddTag {
		key: String,
		value: String,
	},
	UpdateValue {
		key: String,
		value: String,
		previous_value: String,
	},
	RemoveTag {
		key: String,
		value: String,
	},
}

// TODO:
pub enum EditError {}

// TODO: Merging
impl Edit for UpdateData {
	type Target = MapData;
	type Output = Result<(), EditError>;

	fn edit(&mut self, target: &mut Self::Target) -> Self::Output {
		let Some(element) = target.get_mut(&self.0) else {
			unreachable!("Element must exist");
		};
		match &self.1 {
			Action::AddTag { key, value } => {
				// TODO: Error when key already exists.
				element.tags.insert(key.clone(), value.clone());
			}
			Action::UpdateValue { key, value, .. } => {
				element.tags.insert(key.clone(), value.clone());
			}
			Action::RemoveTag { key, .. } => {
				element.tags.remove(key);
			}
		}
		Ok(())
	}

	fn undo(&mut self, target: &mut Self::Target) -> Self::Output {
		let Some(element) = target.get_mut(&self.0) else {
			unreachable!("Element must exist");
		};
		match &self.1 {
			Action::AddTag { key, .. } => {
				element.tags.remove(key);
			}
			Action::UpdateValue {
				key, previous_value, ..
			} => {
				element.tags.insert(key.clone(), previous_value.clone());
			}
			Action::RemoveTag { key, value } => {
				element.tags.insert(key.clone(), value.clone());
			}
		}
		Ok(())
	}
}
