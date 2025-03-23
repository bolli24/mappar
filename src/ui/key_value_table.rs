use super::{debug_text::DebugText, map_widget::map_memory::Selection, messages::Messages};
use crate::map::map_data::{MapData, MapElement};
use derive_new::new;
use egui::{Frame, Key, Margin, TextBuffer, TextEdit, Ui, Widget, text_edit::TextEditOutput};
use egui_extras::{Column, TableBuilder};
use log::info;
use std::sync::{Arc, Mutex};

#[derive(new)]
pub struct KeyValueTable<'a> {
	map_data: Arc<Mutex<MapData>>,
	selection: &'a mut Selection,
	messages: &'a mut Messages,
	debug_text: &'a mut DebugText,
}

impl Widget for KeyValueTable<'_> {
	fn ui(self, ui: &mut egui::Ui) -> egui::Response {
		// const COLUMN_MARGIN: f32 = 20.0; // FIXME: Make it so we don't need this for the side panel to be able to shrink
		const ACTION_COLUMN: f32 = 36.0;
		let panel = ui.response().rect;
		let column_width = ((panel.width() - ACTION_COLUMN) * 0.5).clamp(40.0, 400.0);
		let mut map_data = self.map_data.lock().unwrap();

		let id = self.selection.selected_elements.iter().next();

		let element = id.and_then(|id| map_data.get(id));
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
					.body(|body| {
						let Some(MapElement { tags, new_tag, .. }) = element else {
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
									edit = Some(map_data.update_value(id, key.clone(), update_value))
								}
								row.col(|ui| {
									if ui.small_button("🗑").clicked() {
										edit = Some(map_data.remove_tag(id, key));
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
											edit = Some(map_data.update_new_tag(id, key, value));
										}
									}
									None => {
										if !key.is_empty() || !value.is_empty() {
											edit = Some(map_data.update_new_tag(id, key, value));
										}
									}
								}
								if confirm {
									info!("Confirmed");
									edit = Some(map_data.add_new_tag(id));
								}
							}
						});
					});
			})
			.response;

		if let Some(Err(err)) = edit.map(|edit| map_data.edit(edit)) {
			self.messages.add(err);
		}

		response
	}
}

fn table_field(value: &mut dyn TextBuffer, ui: &mut Ui) -> TextEditOutput {
	TextEdit::singleline(value).margin(Margin::same(1)).show(ui)
}
