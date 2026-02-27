use super::{debug_text::DebugText, map_widget::map_memory::Selection, messages::Messages};
use crate::map::map_data::{EditHistory, MapData};
use crate::ui::key_value_table::table_field;
use crate::{Element, ElementId};
use derive_new::new;
use egui::ahash::AHashSet;
use egui::{Frame, RichText, Ui, Widget};
use egui_extras::{Column, TableBuilder};

use epaint::FontId;
use itertools::Itertools;

#[derive(new)]
pub struct RelationsTable<'a> {
	map_data: &'a mut MapData,
	history: &'a mut EditHistory,
	selection: &'a mut Selection,
	messages: &'a mut Messages,
	debug_text: &'a mut DebugText,
	highlighted_elements: &'a mut AHashSet<ElementId>,
}

impl Widget for RelationsTable<'_> {
	fn ui(self, ui: &mut Ui) -> egui::Response {
		ui.label(RichText::new("Relations").font(FontId::proportional(20.0)));

		let single_element = self.selection.elements.iter().exactly_one().ok();

		let edit = None;
		let mut selected = None;
		let mut edit_relation = None;

		if let Some(element_id) = single_element {
			let relations = self
				.map_data
				.reference_index
				.parent_relations(element_id)
				.flat_map(|relation_id| {
					let relation = self.map_data.get(relation_id).expect("Relation must exist.");
					let Element::Relation { members, .. } = &relation.element else {
						unreachable!("Element must be relation.");
					};
					members
						.iter()
						.enumerate()
						.filter(|(_, member)| member.ref_id == *element_id)
						.map(move |(index, member)| (relation, &member.role, index))
				})
				.collect::<Vec<_>>();

			const POS_COLUMN: f32 = 36.0;
			let available_width = ui.available_width();
			let spacing = ui.spacing().item_spacing.x * 2.0;
			let column_width = ((available_width - POS_COLUMN - spacing) * 0.5).clamp(40.0, 400.0);

			Frame::NONE.show(ui, |ui| {
				TableBuilder::new(ui)
					.id_salt("parent_relations_table")
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
									ui.horizontal(|ui| {
										let label = ui.link(parent.id.id().to_string());
										if label.clicked() {
											selected = Some(parent.id);
										}
										if label.hovered() {
											self.highlighted_elements.insert(parent.id);
										}
										if ui.small_button(RichText::new(egui_phosphor::bold::PENCIL)).clicked() {
											edit_relation = Some(parent.id);
										}
									});
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
		} else {
			let relations = self.map_data.iter().filter(|e| e.id.is_relation()).collect::<Vec<_>>();

			Frame::NONE.show(ui, |ui| {
				TableBuilder::new(ui)
					.id_salt("all_relations_table")
					.auto_shrink([false, true])
					.striped(true)
					.column(Column::remainder())
					.header(20.0, |mut header| {
						header.col(|ui| {
							ui.strong("Relation");
						});
					})
					.body(|mut body| {
						for relation in relations {
							body.row(16.0, |mut row| {
								row.col(|ui| {
									ui.horizontal(|ui| {
										let label = ui.link(relation.id.id().to_string());

										if label.clicked() {
											selected = Some(relation.id);
										}
										if label.hovered() {
											self.highlighted_elements.insert(relation.id);
										}
										if ui.small_button(RichText::new(egui_phosphor::bold::PENCIL)).clicked() {
											edit_relation = Some(relation.id);
										}
									});
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

		if let Some(relation) = edit_relation {
			self.selection.relation = Some(relation);
		}

		ui.response()
	}
}
