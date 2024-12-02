use egui::{Frame, Margin, TextBuffer, TextEdit, Ui, Widget};
use egui_extras::{Column, TableBuilder};

use super::map_widget::map_memory::Selection;
use crate::map::map_data::{MapData, Tag};
use std::sync::{Arc, Mutex};

pub struct KeyValueTable<'a> {
	map_data: Arc<Mutex<MapData>>,
	selection: &'a mut Selection,
}

impl<'a> KeyValueTable<'a> {
	pub fn new(map_data: Arc<Mutex<MapData>>, selection: &'a mut Selection) -> Self {
		Self { map_data, selection }
	}
}

impl Widget for KeyValueTable<'_> {
	fn ui(self, ui: &mut egui::Ui) -> egui::Response {
		// const COLUMN_MARGIN: f32 = 20.0; // FIXME: Make it so we don't need this for the side panel to be able to shrink
		const ACTION_COLUMN: f32 = 36.0;
		let panel = ui.response().rect;
		let column_width = ((panel.width() - ACTION_COLUMN) * 0.5).clamp(40.0, 400.0);
		let mut map_data = self.map_data.lock().unwrap();

		let tags = self
			.selection
			.selected_elements
			.iter()
			.next()
			.and_then(|id| map_data.get_mut(id))
			.map(|e| &mut e.tags);

		let mut delete_index = None;

		Frame::none()
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
						body.rows(16.0, tags.len() + 1, |mut row| {
							let index = row.index();
							if index < tags.len() {
								row.col(|ui| {
									table_field(&mut tags[index].key, ui);
								});
								row.col(|ui| {
									table_field(&mut tags[index].value, ui);
								});
								row.col(|ui| {
									if ui.small_button("🗑").clicked() {
										delete_index = Some(index);
									}
								});
							} else {
								let Tag { key, value } = self.selection.new_tag.get_or_insert(Default::default());
								row.col(|ui| {
									table_field(key, ui);
								});
								row.col(|ui| {
									table_field(value, ui);
								});
								if !key.is_empty() && !value.is_empty() {
									let new_tag = self.selection.new_tag.take().unwrap();
									tags.push(new_tag);
								}
							}
						});

						if let Some(delete_index) = delete_index {
							tags.remove(delete_index);
						}
					});
			})
			.response
	}
}

fn table_field(value: &mut dyn TextBuffer, ui: &mut Ui) {
	TextEdit::singleline(value).margin(Margin::same(1.0)).show(ui);
}
