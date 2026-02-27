use super::{debug_text::DebugText, map_widget::map_memory::Selection, messages::Messages};
use crate::map::element::Member;
use crate::map::map_data::{EditHistory, MapData};
use crate::ui::downloader::{DownloadTask, Downloader};
use crate::ui::key_value_table::table_field;
use crate::{Element, ElementId, MapElement, message_info};
use derive_new::new;
use egui::ahash::AHashSet;
use egui::{Button, Frame, RichText, Sides, Ui, Widget};
use egui_extras::{Column, TableBuilder};
use epaint::FontId;

#[derive(new)]
pub struct RelationEditor<'a> {
	map_data: &'a mut MapData,
	history: &'a mut EditHistory,
	selection: &'a mut Selection,
	messages: &'a mut Messages,
	debug_text: &'a mut DebugText,
	highlighted_elements: &'a mut AHashSet<ElementId>,
	downloader: &'a mut Downloader,
}

impl Widget for RelationEditor<'_> {
	fn ui(self, ui: &mut Ui) -> egui::Response {
		let relation = self.selection.relation.and_then(|id| self.map_data.get(&id));

		let Some(MapElement {
			id,
			element: Element::Relation {
				members,
				fully_downloaded,
			},
			..
		}) = relation
		else {
			return ui.response();
		};

		let edit = None;
		let mut selected = None;

		Sides::new().show(
			ui,
			|ui| {
				ui.label(RichText::new("Members").font(FontId::proportional(20.0)));
			},
			|ui| {
				let task = DownloadTask::Relation(*id);
				let already_downloading = self.downloader.is_downloading(&task);
				let can_download = !fully_downloaded && !already_downloading;
				if already_downloading {
					ui.spinner().on_hover_text("Already downloading members.");
				} else if ui
					.add_enabled(
						can_download,
						Button::new(RichText::new(egui_phosphor::bold::CLOUD_ARROW_DOWN)),
					)
					.on_hover_text(if *fully_downloaded {
						"No members to download."
					} else {
						"Download missing members."
					})
					.clicked()
				{
					message_info!(self.messages, "Downloading relation members.");
					self.downloader.download(task);
				}
			},
		);

		const ACTION_COLUMN: f32 = 36.0;
		let available_width = ui.available_width();
		let spacing = ui.spacing().item_spacing.x * 2.0;
		let column_width = ((available_width - ACTION_COLUMN - spacing) * 0.5).clamp(40.0, 400.0);

		Frame::NONE.show(ui, |ui| {
			TableBuilder::new(ui)
				.id_salt("members_table")
				.auto_shrink([false, true])
				.striped(true)
				.column(Column::exact(ACTION_COLUMN))
				.column(Column::exact(column_width))
				.column(Column::exact(column_width))
				.header(20.0, |mut header| {
					header.col(|ui| {
						ui.strong("Pos");
					});
					header.col(|ui| {
						ui.strong("Element");
					});
					header.col(|ui| {
						ui.strong("Role");
					});
				})
				.body(|mut body| {
					for Member { ref_id, role } in members {
						body.row(16.0, |mut row| {
							let index = row.index();
							let mut role = role.clone();
							row.col(|ui| {
								ui.label(index.to_string());
							});
							row.col(|ui| {
								// TODO: Generate element name.
								let name = ref_id.id().to_string();
								if self.map_data.get(ref_id).is_some() {
									let label = ui.link(name);
									if label.clicked() {
										selected = Some(*ref_id);
									}
									if label.hovered() {
										self.highlighted_elements.insert(*ref_id);
									}
								} else {
									ui.label(name).on_hover_text("Member not downloaded.");
								}
							});
							row.col(|ui| {
								table_field(&mut role, ui);
							});
						});
					}
				});
		});

		if let Some(Err(err)) = edit.map(|edit| self.history.edit(self.map_data, edit)) {
			self.messages.add(&err);
		}

		if let Some(selected) = selected {
			self.selection.clear(self.map_data);
			self.selection.elements.insert(selected);
		}

		ui.response()
	}
}
