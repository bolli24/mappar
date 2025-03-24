use std::collections::VecDeque;

use derive_new::new;
use egui::{Color32, FontId, Frame, RichText, ScrollArea, Widget};

use crate::imagery::{Imageries, ImageryId};

pub struct Layers(pub VecDeque<Layer>);

impl Layers {
	pub fn get_active_imagery(&self) -> Option<&ImageryId> {
		self.0.iter().find(|layer| layer.1).map(|layer| &layer.0)
	}

	pub fn new() -> Self {
		Self(VecDeque::new())
	}
}

#[derive(new)]
pub struct LayerList<'a> {
	layers: &'a mut Layers,
	imageries: &'a Imageries,
}

pub struct Layer(pub ImageryId, pub bool);

impl Widget for LayerList<'_> {
	fn ui(self, ui: &mut egui::Ui) -> egui::Response {
		Frame::NONE
			.show(ui, |ui| {
				if ui.input(|i| i.key_pressed(egui::Key::Backtick)) {
					self.layers.0.rotate_left(1);
				}

				ui.label(RichText::new("Layers").font(FontId::proportional(20.0)));
				ScrollArea::vertical().show(ui, |ui| {
					if self.imageries.entries.is_empty() {
						ui.label("Loading Imageries");
						ui.spinner();
						return;
					}
					for Layer(layer, visible) in &mut self.layers.0 {
						let Some(imagery) = self.imageries.entries.get(layer) else {
							ui.label(
								RichText::new(format!("Invalid layer: '{}'", layer.0))
									.color(Color32::from_rgb(211, 47, 47)),
							);
							continue;
						};

						ui.horizontal(|ui| {
							ui.checkbox(visible, "");
							ui.label(imagery.name());
						});
					}
				})
			})
			.response
	}
}
