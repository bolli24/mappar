use super::{debug_text::DebugText, map_widget::map_memory::Selection, messages::Messages};
use crate::api::client::OsmClient;
use crate::map::element::Member;
use crate::map::map_data::{EditHistory, MapData};
use crate::ui::app::{DownloadTask, MapDataResult};
use crate::ui::key_value_table::table_field;
use crate::{Element, ElementId, message_info, spawn_task};
use derive_new::new;
use egui::ahash::AHashSet;
use egui::{Button, Frame, RichText, Sides, Ui, Widget};
use egui_extras::{Column, TableBuilder};
use epaint::FontId;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;

#[derive(new)]
pub struct RelationsTable<'a> {
	map_data: &'a mut MapData,
	history: &'a mut EditHistory,
	selection: &'a mut Selection,
	messages: &'a mut Messages,
	debug_text: &'a mut DebugText,
	highlighted_elements: &'a mut AHashSet<ElementId>,
	map_data_sender: &'a UnboundedSender<MapDataResult>,
	download_tasks: &'a mut AHashSet<DownloadTask>,
	osm_client: Arc<tokio::sync::Mutex<OsmClient>>,
}

impl Widget for RelationsTable<'_> {
	fn ui(self, ui: &mut Ui) -> egui::Response {
		ui.label(RichText::new("Relations").font(FontId::proportional(20.0)));
		const ACTION_COLUMN: f32 = 36.0;
		let available_width = ui.available_width();
		let spacing = ui.spacing().item_spacing.x * 2.0; // 2 gaps between 3 columns
		let column_width = ((available_width - ACTION_COLUMN - spacing) * 0.5).clamp(40.0, 400.0);

		let id = self.selection.elements.iter().next();

		let element = id.and_then(|id| self.map_data.get(id));
		let edit = None;
		let mut selected = None;

		if self.selection.elements.len() > 1 {
			return ui.label("Select only one element to edit relations.");
		}
		let Some(element) = element else {
			return ui.label("Nothing is selected.");
		};

		{
			let mut relations = self
				.map_data
				.reference_index
				.parent_relations(&element.id)
				.flat_map(|relation_id| {
					let relation = self.map_data.get(relation_id).expect("Relation must exist.");
					let Element::Relation { members, .. } = &relation.element else {
						unreachable!("Element must be relation.");
					};

					members
						.iter()
						.enumerate()
						.filter(|(_, member)| member.ref_id == element.id)
						.map(move |(index, member)| (relation, &member.role, index))
				})
				.peekable();

			if relations.peek().is_none() {
				ui.label("Element is not member of any relations.");
			} else {
				Frame::NONE.show(ui, |ui| {
					TableBuilder::new(ui)
						.id_salt("relations_table")
						.auto_shrink([false, true])
						.striped(true)
						.column(Column::exact(column_width))
						.column(Column::exact(column_width))
						.column(Column::remainder())
						.header(20.0, |mut header| {
							header.col(|ui| {
								ui.strong("Parent");
							});
							header.col(|ui| {
								ui.strong("Role");
							});
							header.col(|ui| {
								ui.strong("Pos");
							});
						})
						.body(|mut body| {
							for (parent, role, index) in relations {
								body.row(16.0, |mut row| {
									let mut role = role.clone();
									row.col(|ui| {
										// TODO: Generate element name.
										let label = ui.link(parent.id.0.to_string());
										if label.clicked() {
											selected = Some(parent.id);
										}
										if label.hovered() {
											self.highlighted_elements.insert(parent.id);
										}
									});
									row.col(|ui| {
										table_field(&mut role, ui);
									});
									row.col(|ui| {
										ui.label(index.to_string());
									});
								});
							}
						});
				});
			}
		}

		if let Element::Relation {
			members,
			fully_downloaded,
		} = &element.element
		{
			Sides::new().show(
				ui,
				|ui| {
					ui.label(RichText::new("Members").font(FontId::proportional(20.0)));
				},
				|ui| {
					let task = DownloadTask::Relation(element.id);
					let already_downloading = self.download_tasks.contains(&task);
					let can_download = !fully_downloaded && !already_downloading;
					if ui
						.add_enabled(
							can_download,
							Button::new(RichText::new(egui_phosphor::bold::CLOUD_ARROW_DOWN)),
						)
						.on_hover_text(if *fully_downloaded {
							"No members to download."
						} else if already_downloading {
							"Already downloading members."
						} else {
							"Download missing members."
						})
						.clicked()
					{
						message_info!(self.messages, "Downloading relation members.");

						spawn_task!({
							let map_data_sender = self.map_data_sender.clone();
							let id = element.id;
							self.download_tasks.insert(task);
							async move {
								map_data_sender
									.send(
										self.osm_client
											.lock()
											.await
											.get_full(id)
											.await
											.map(|result| (result, task)),
									)
									.expect("Channel must not be closed.");
							}
						});
					}
				},
			);

			let available_width = ui.available_width();
			let spacing = ui.spacing().item_spacing.x * 2.0;
			let column_width = ((available_width - spacing) * 0.5).clamp(40.0, 400.0);

			Frame::NONE.show(ui, |ui| {
				TableBuilder::new(ui)
					.id_salt("members_table")
					.auto_shrink([false, true])
					.striped(true)
					.column(Column::exact(column_width))
					.column(Column::exact(column_width))
					.header(20.0, |mut header| {
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
								let mut role = role.clone();
								row.col(|ui| {
									// TODO: Generate element name.
									let name = ref_id.0.to_string();
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
		}

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
