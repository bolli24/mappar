use std::collections::VecDeque;

use derive_new::new;
use egui::{Color32, FontId, Frame, RichText, Widget};
use egui_extras::{Column, TableBuilder};

use crate::imagery::{Imageries, ImageryId};

pub struct Layer(pub ImageryId, pub bool);

pub struct Layers(pub VecDeque<Layer>);

impl Layers {
	pub fn get_active_imagery(&self) -> Option<&ImageryId> {
		self.0.iter().find(|layer| layer.1).map(|layer| &layer.0)
	}

	pub fn new() -> Self {
		Self(VecDeque::new())
	}

	fn rotate(&mut self) {
		if self.0.len() > 1 {
			self.0.rotate_left(1);
		}
	}
}

#[derive(new)]
pub struct LayerList<'a> {
	layers: &'a mut Layers,
	imageries: &'a Imageries,
	imagery_settings: &'a mut bool,
}

impl Widget for LayerList<'_> {
	fn ui(self, ui: &mut egui::Ui) -> egui::Response {
		Frame::NONE
			.show(ui, |ui| {
				ui.label(RichText::new("Layers").font(FontId::proportional(20.0)));

				let mut delete = None;

				TableBuilder::new(ui)
					.striped(true)
					.column(Column::auto())
					.column(Column::remainder())
					.column(Column::auto())
					.body(|mut body| {
						if self.imageries.entries.is_empty() {
							body.ui_mut().label("Loading Imageries");
							body.ui_mut().spinner();
							return;
						}

						for (index, Layer(layer, visible)) in &mut self.layers.0.iter_mut().enumerate() {
							body.row(20.0, |mut row| {
								let imagery = self.imageries.entries.get(layer);
								row.col(|ui| {
									ui.checkbox(visible, "");
								});
								row.col(|ui| {
									ui.horizontal_centered(|ui| match imagery {
										Some(imagery) => {
											ui.label(imagery.name());
										}
										None => {
											ui.label(
												RichText::new(format!("Invalid layer: '{}'", layer.0))
													.color(Color32::from_rgb(211, 47, 47)),
											);
										}
									});
								});
								row.col(|ui| {
									if ui
										.button(RichText::new(egui_phosphor::bold::TRASH).size(12.0))
										.clicked()
									{
										delete = Some(index);
									}
								});
							});
						}
					});

				if let Some(delete) = delete {
					self.layers.0.remove(delete);
				}

				ui.horizontal(|ui| {
					if ui
						.button(RichText::new(egui_phosphor::bold::ARROWS_COUNTER_CLOCKWISE).size(20.0))
						.clicked() || ui.input(|i| i.key_pressed(egui::Key::Backtick))
					{
						self.layers.rotate();
					}

					if ui.button(RichText::new(egui_phosphor::bold::GEAR).size(20.0)).clicked() {
						*self.imagery_settings = true;
					}
				})
				.response
			})
			.response
	}
}
