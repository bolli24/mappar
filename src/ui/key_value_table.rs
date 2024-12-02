use super::map_widget::map_memory::Selection;
use crate::map::{
	edit::{Action, UpdateData},
	map_data::MapData,
};
use egui::{Frame, Margin, TextBuffer, TextEdit, Ui, Widget};
use egui_extras::{Column, TableBuilder};
use std::sync::{Arc, Mutex};
use undo::Record;

pub struct KeyValueTable<'a> {
	map_data: Arc<Mutex<MapData>>,
	selection: &'a mut Selection,
	history: &'a mut Record<UpdateData>,
}

impl<'a> KeyValueTable<'a> {
	pub fn new(
		map_data: Arc<Mutex<MapData>>,
		selection: &'a mut Selection,
		history: &'a mut Record<UpdateData>,
	) -> Self {
		Self {
			map_data,
			selection,
			history,
		}
	}
}

impl Widget for KeyValueTable<'_> {
	fn ui(self, ui: &mut egui::Ui) -> egui::Response {
		// const COLUMN_MARGIN: f32 = 20.0; // FIXME: Make it so we don't need this for the side panel to be able to shrink
		const ACTION_COLUMN: f32 = 36.0;
		let panel = ui.response().rect;
		let column_width = ((panel.width() - ACTION_COLUMN) * 0.5).clamp(40.0, 400.0);
		let mut map_data = self.map_data.lock().unwrap();

		let id = self.selection.selected_elements.iter().next();

		let tags = id.and_then(|id| map_data.get(id)).map(|e| &e.tags);
		let mut edit = None;

		let response = Frame::none()
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
					.body(|body| {
						let Some(tags) = tags else {
							return;
						};
						let id = *id.unwrap();
						// FIXME: Keep order of tags.
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
									edit = Some(UpdateData(
										id,
										Action::UpdateValue {
											key: key.clone(),
											value: update_value,
											previous_value: value.clone(),
										},
									));
								}
								row.col(|ui| {
									if ui.small_button("🗑").clicked() {
										edit = Some(UpdateData(
											id,
											Action::RemoveTag {
												key,
												value: value.clone(),
											},
										));
									}
								});
							} else {
								// TODO: Add Action for editing new_tag
								let (key, value) = self.selection.new_tag.get_or_insert(Default::default());
								row.col(|ui| {
									table_field(key, ui);
								});
								row.col(|ui| {
									table_field(value, ui);
								});
								if !key.is_empty() && !value.is_empty() {
									let (key, value) = self.selection.new_tag.take().unwrap();
									edit = Some(UpdateData(id, Action::AddTag { key, value }));
								}
							}
						});
					});
			})
			.response;

		if let Some(edit) = edit {
			self.history.edit(&mut map_data, edit);
		}

		response
	}
}

fn table_field(value: &mut dyn TextBuffer, ui: &mut Ui) {
	TextEdit::singleline(value).margin(Margin::same(1.0)).show(ui);
}
