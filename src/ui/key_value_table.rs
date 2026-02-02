use super::{debug_text::DebugText, map_widget::map_memory::Selection, messages::Messages};
use crate::map::element::MapElement;
use crate::map::map_data::{EditHistory, MapData};
use derive_new::new;
use egui::{Frame, Key, Margin, RichText, TextBuffer, TextEdit, Ui, Widget, text_edit::TextEditOutput};
use egui_extras::{Column, TableBuilder};
use either::Either;
use epaint::FontId;
use fxhash::FxHashMap;
use indexmap::IndexMap;
use log::info;

#[derive(new)]
pub struct KeyValueTable<'a> {
	map_data: &'a mut MapData,
	history: &'a mut EditHistory,
	selection: &'a mut Selection,
	messages: &'a mut Messages,
	debug_text: &'a mut DebugText,
}

impl Widget for KeyValueTable<'_> {
	fn ui(self, ui: &mut Ui) -> egui::Response {
		ui.label(RichText::new("Tags").font(FontId::proportional(20.0)));
		// const COLUMN_MARGIN: f32 = 20.0; // FIXME: Make it so we don't need this for the side panel to be able to shrink
		const ACTION_COLUMN: f32 = 36.0;
		let panel = ui.response().rect;
		let column_width = ((panel.width() - ACTION_COLUMN) * 0.5).clamp(40.0, 400.0);

		let id = self.selection.elements.iter().next();

		let element = id.and_then(|id| self.map_data.get(id));
		let mut edit = None;

		let response = Frame::NONE
			.show(ui, |ui| {
				TableBuilder::new(ui)
					.id_salt("key_value_table")
					.auto_shrink([false, true])
					.striped(true)
					// .sense(Sense::click_and_drag())
					.column(Column::exact(column_width))
					.column(Column::exact(column_width))
					.column(Column::remainder())
					.header(20.0, |mut header| {
						header.col(|ui| {
							ui.strong("Key");
						});
						header.col(|ui| {
							ui.strong("Value");
						});
					})
					.body(|mut body| {
						if self.selection.elements.len() > 1 {
							body.ui_mut().label("Select only one element to edit tags.");
							return;
						}
						let Some(MapElement { tags, new_tag, .. }) = element else {
							body.ui_mut().label("Nothing is selected.");
							return;
						};
						let id = *id.unwrap();
						let mut tags_iter = tags.iter();
						body.rows(16.0, tags.len() + 1, |mut row| {
							if let Some((key, value)) = tags_iter.next() {
								let mut key = key.clone();
								let mut update_value = value.clone();
								row.col(|ui| {
									table_field(&mut key, ui);
								});
								row.col(|ui| {
									table_field(&mut update_value, ui);
								});
								if update_value != *value {
									edit = Some(self.map_data.update_value(id, key.clone(), update_value));
								}
								row.col(|ui| {
									if ui.small_button(RichText::new(egui_phosphor::bold::TRASH)).clicked() {
										edit = Some(self.map_data.remove_tag(id, key));
									}
								});
							} else {
								// TODO: Add Action for editing new_tag
								let (mut key, mut value) = new_tag.clone().unwrap_or_default();
								let mut confirm = false;
								for field in [&mut key, &mut value] {
									row.col(|ui| {
										if table_field(field, ui).response.lost_focus()
											&& ui.ctx().input(|i| i.key_down(Key::Enter))
										{
											confirm = true;
										}
									});
								}
								match new_tag {
									Some((previous_key, previous_value)) => {
										if *previous_value != value || *previous_key != key {
											edit = Some(self.map_data.update_new_tag(id, key, value));
										}
									}
									None => {
										if !key.is_empty() || !value.is_empty() {
											edit = Some(self.map_data.update_new_tag(id, key, value));
										}
									}
								}
								if confirm {
									info!("Confirmed");
									edit = Some(self.map_data.add_new_tag(id));
								}
							}
						});
					});
			})
			.response;

		if let Some(Err(err)) = edit.map(|edit| self.history.edit(self.map_data, edit)) {
			self.messages.add(&err);
		}

		response
	}
}

/// Aggregates tags from multiple elements, returning Left for common values or Right with counts for differing values.
pub fn get_tags(elements: &[MapElement]) -> IndexMap<String, Either<String, FxHashMap<Option<String>, usize>>> {
	match elements.len() {
		0 => return IndexMap::new(),
		1 => {
			let mut result = IndexMap::with_capacity(elements[0].tags.len());
			for (key, value) in &elements[0].tags {
				result.insert(key.clone(), Either::Left(value.clone()));
			}
			return result;
		}
		_ => {}
	}

	let mut value_counts: FxHashMap<String, FxHashMap<Option<String>, usize>> = FxHashMap::default();
	let estimated_keys = elements.iter().map(|e| e.tags.len()).sum::<usize>() / 2;
	value_counts.reserve(estimated_keys);

	for element in elements {
		for (key, value) in &element.tags {
			let count_map = value_counts.entry(key.clone()).or_default();
			*count_map.entry(Some(value.clone())).or_insert(0) += 1;
		}
	}

	let total_elements = elements.len();
	for count_map in value_counts.values_mut() {
		let total_count: usize = count_map.values().sum();
		let missing = total_elements - total_count;
		if missing > 0 {
			count_map.entry(None).or_insert(missing);
		}
	}

	let mut result = IndexMap::with_capacity(value_counts.len());
	for (key, counts) in value_counts {
		if counts.len() == 1 {
			if let Some((Some(value), _count)) = counts.into_iter().next() {
				result.insert(key, Either::Left(value));
			}
		} else {
			result.insert(key, Either::Right(counts));
		}
	}

	result
}

fn table_field(value: &mut dyn TextBuffer, ui: &mut Ui) -> TextEditOutput {
	TextEdit::singleline(value).margin(Margin::same(1)).show(ui)
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::map::element::{Element, ElementId, ElementInfo, ElementType};
	use crate::map::latlon::LatLon;
	use either::Either::{Left, Right};
	use indexmap::indexmap;
	use std::cell::Cell;
	use std::time::Instant;

	fn create_element(id: i64, tags: IndexMap<String, String>) -> MapElement {
		MapElement {
			id: ElementId(id, ElementType::Node),
			info: ElementInfo::default(),
			element: Element::Node {
				pos: Cell::new(LatLon { lat: 0.0, lon: 0.0 }),
			},
			tags,
			new_tag: None,
			generation: 0,
		}
	}

	#[test]
	fn test_get_tags_empty() {
		let elements: &[MapElement] = &[];
		let result = get_tags(elements);
		assert!(result.is_empty());
	}

	#[test]
	fn test_get_tags_single_element() {
		let tags =
			indexmap! { "name".to_string() => "test".to_string(), "highway".to_string() => "primary".to_string() };
		let elements = vec![create_element(1, tags)];
		let result = get_tags(&elements);
		assert_eq!(result.len(), 2);
		assert_eq!(result.get("name"), Some(&Left("test".to_string())));
		assert_eq!(result.get("highway"), Some(&Left("primary".to_string())));
	}

	#[test]
	fn test_get_tags_multiple_same_tags() {
		let tags1 = indexmap! { "name".to_string() => "test".to_string() };
		let tags2 = indexmap! { "name".to_string() => "test".to_string() };
		let elements = vec![create_element(1, tags1), create_element(2, tags2)];
		let result = get_tags(&elements);
		assert_eq!(result.len(), 1);
		assert_eq!(result.get("name"), Some(&Left("test".to_string())));
	}

	#[test]
	fn test_get_tags_multiple_different_tags() {
		let tags1 = indexmap! { "name".to_string() => "test1".to_string() };
		let tags2 = indexmap! { "name".to_string() => "test2".to_string() };
		let elements = vec![create_element(1, tags1), create_element(2, tags2)];
		let result = get_tags(&elements);
		assert_eq!(result.len(), 1);
		match result.get("name") {
			Some(Right(map)) => {
				assert_eq!(map.len(), 2);
				assert_eq!(map.get(&Some("test1".to_string())), Some(&1));
				assert_eq!(map.get(&Some("test2".to_string())), Some(&1));
			}
			_ => panic!("Expected Right with HashMap"),
		}
	}

	#[test]
	fn test_get_tags_mixed_same_and_different() {
		let tags1 =
			indexmap! { "name".to_string() => "common".to_string(), "highway".to_string() => "primary".to_string() };
		let tags2 =
			indexmap! { "name".to_string() => "common".to_string(), "highway".to_string() => "secondary".to_string() };
		let elements = vec![create_element(1, tags1), create_element(2, tags2)];
		let result = get_tags(&elements);
		assert_eq!(result.len(), 2);
		assert_eq!(result.get("name"), Some(&Left("common".to_string())));
		match result.get("highway") {
			Some(Right(map)) => {
				assert_eq!(map.len(), 2);
				assert_eq!(map.get(&Some("primary".to_string())), Some(&1));
				assert_eq!(map.get(&Some("secondary".to_string())), Some(&1));
			}
			_ => panic!("Expected Right with HashMap"),
		}
	}

	#[test]
	fn test_get_tags_counts_occurrences() {
		let tags1 = indexmap! { "highway".to_string() => "primary".to_string() };
		let tags2 = indexmap! { "highway".to_string() => "primary".to_string() };
		let tags3 = indexmap! { "highway".to_string() => "secondary".to_string() };
		let elements = vec![
			create_element(1, tags1),
			create_element(2, tags2),
			create_element(3, tags3),
		];
		let result = get_tags(&elements);
		assert_eq!(result.len(), 1);
		match result.get("highway") {
			Some(Right(map)) => {
				assert_eq!(map.len(), 2);
				assert_eq!(map.get(&Some("primary".to_string())), Some(&2));
				assert_eq!(map.get(&Some("secondary".to_string())), Some(&1));
			}
			_ => panic!("Expected Right with HashMap"),
		}
	}

	#[test]
	fn test_get_tags_missing_tags() {
		let tags1 = indexmap! { "name".to_string() => "test".to_string() };
		let tags2 = indexmap! {};
		let tags3 = indexmap! { "name".to_string() => "test".to_string() };
		let elements = vec![
			create_element(1, tags1),
			create_element(2, tags2),
			create_element(3, tags3),
		];
		let result = get_tags(&elements);
		assert_eq!(result.len(), 1);
		match result.get("name") {
			Some(Right(map)) => {
				assert_eq!(map.len(), 2);
				assert_eq!(map.get(&Some("test".to_string())), Some(&2));
				assert_eq!(map.get(&None), Some(&1));
			}
			_ => panic!("Expected Right with HashMap"),
		}
	}

	#[test]
	fn test_get_tags_mixed_present_and_missing() {
		let tags1 =
			indexmap! { "name".to_string() => "test".to_string(), "highway".to_string() => "primary".to_string() };
		let tags2 = indexmap! { "name".to_string() => "test".to_string() };
		let elements = vec![create_element(1, tags1), create_element(2, tags2)];
		let result = get_tags(&elements);
		assert_eq!(result.len(), 2);
		assert_eq!(result.get("name"), Some(&Left("test".to_string())));
		match result.get("highway") {
			Some(Right(map)) => {
				assert_eq!(map.len(), 2);
				assert_eq!(map.get(&Some("primary".to_string())), Some(&1));
				assert_eq!(map.get(&None), Some(&1));
			}
			_ => panic!("Expected Right with HashMap"),
		}
	}

	#[test]
	fn benchmark_get_tags() {
		let mut elements = Vec::new();
		let all_keys: Vec<String> = (0..100).map(|i| format!("key_{}", i)).collect();
		let all_values: Vec<String> = (0..200).map(|i| format!("value_{}", i)).collect();
		let mut seed = 42u64;

		for i in 0..1000 {
			let mut tags = IndexMap::new();
			for _ in 0..20 {
				seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
				let key_idx = seed as usize % all_keys.len();
				seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
				let value_idx = seed as usize % all_values.len();
				tags.insert(all_keys[key_idx].clone(), all_values[value_idx].clone());
			}
			elements.push(create_element(i as i64, tags));
		}

		let start = Instant::now();
		for _ in 0..100 {
			let _ = get_tags(&elements);
		}
		let duration = start.elapsed();
		println!("get_tags benchmark: 100 iterations in {:?}", duration);
		println!("Average: {:?}", duration / 100);
	}
}
