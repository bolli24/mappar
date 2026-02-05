use super::{debug_text::DebugText, map_widget::map_memory::Selection, messages::Messages};
use crate::map::element::MapElement;
use crate::map::map_data::{EditHistory, MapData};
use crate::ui::key_value_table::table_field;
use crate::{Element, ElementId};
use chumsky::container::Container;
use derive_new::new;
use egui::ahash::AHashSet;
use egui::{Frame, RichText, Ui, Widget};
use egui_extras::{Column, TableBuilder};
use epaint::FontId;

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
		let Some(MapElement { id, .. }) = element else {
			return ui.label("Nothing is selected.");
		};

		let mut relations = self
			.map_data
			.reference_index
			.parent_relations(id)
			.flat_map(|relation_id| {
				let element = self.map_data.get(relation_id).expect("Relation must exist.");
				let Element::Relation { members } = &element.element else {
					unreachable!("Element must be relation.");
				};

				members
					.iter()
					.enumerate()
					.filter(|(_, member)| member.ref_id == *id)
					.map(move |(index, member)| (element, &member.role, index))
			})
			.peekable();

		if relations.peek().is_none() {
			return ui.label("Element is not member of any relations.");
		}

		let response = Frame::NONE
			.show(ui, |ui| {
				TableBuilder::new(ui)
					.id_salt("relations_table")
					.auto_shrink([false, true])
					.striped(true)
					// .sense(Sense::click_and_drag())
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
			})
			.response;

		if let Some(Err(err)) = edit.map(|edit| self.history.edit(self.map_data, edit)) {
			self.messages.add(&err);
		}

		if let Some(selected) = selected {
			self.selection.clear(self.map_data);
			self.selection.elements.push(selected);
		}

		response
	}
}
