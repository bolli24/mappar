use std::collections::VecDeque;

use egui::{FontId, Frame, RichText, ScrollArea, Widget};

use crate::imagery::{Imageries, ImageryId};

pub struct Layers(pub VecDeque<Layer>);

impl Layers {
	pub fn get_active_imagery(&self) -> Option<ImageryId> {
		self.0.iter().find(|layer| layer.1).map(|layer| layer.0.clone())
	}

	pub fn new() -> Self {
		Self(VecDeque::new())
	}
}

pub struct LayerList<'a> {
	layers: &'a mut Layers,
	imageries: &'a Imageries,
}
impl<'a> LayerList<'a> {
	pub fn new(layers: &'a mut Layers, imageries: &'a Imageries) -> Self {
		Self { layers, imageries }
	}
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
						ui.horizontal(|ui| {
							ui.checkbox(visible, "");
							ui.label(self.imageries.entries.get(layer).unwrap().name());
						});
					}
				})
			})
			.response
	}
}
