use crate::imagery::{Imageries, ImageryId};
use crate::ui::memory::Memory;
use egui::{menu, CentralPanel, Response, ScrollArea, SidePanel, TopBottomPanel, Ui, Widget};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, Deserialize, mappar_macros::Memory)]
struct ImageriesMemory {
	favourites: HashSet<ImageryId>,
	#[serde(skip)]
	selected: Option<ImageryId>,
}

impl Default for ImageriesMemory {
	fn default() -> Self {
		let favourites = HashSet::new();
		Self {
			favourites,
			selected: None,
		}
	}
}

pub struct ImageriesWidget<'a> {
	imageries: &'a Imageries,
}

impl<'a> ImageriesWidget<'a> {
	pub fn new(imageries: &'a Imageries) -> Self {
		Self { imageries }
	}
}

impl Widget for ImageriesWidget<'_> {
	fn ui(self, ui: &mut Ui) -> Response {
		let mut mem = ImageriesMemory::load_or_default(ui.ctx(), Id::from("imagery_settings"));

		TopBottomPanel::top("top_panel").show_inside(ui, |ui| {
			menu::bar(ui, |ui| {
				if ui.button("Fetch JOSM imagery").clicked() {
					/*	crate::imagery::update_josm_imagery(
						Path::new(&self.storage_dir).join(JOSM_FILE).to_path_buf(),
						self.messages.clone(),
						self.imageries.clone(),
					);*/
				}
			});
		});

		SidePanel::left("imagery_list").show_inside(ui, |ui| {
			ScrollArea::vertical().show(ui, |ui| {
				for imagery in &self.imageries.entries {
					if ui.label(imagery.name()).clicked() {
						mem.selected = Some(imagery.id.clone())
					};
				}
			})
		});

		let response = CentralPanel::default().show_inside(ui, |ui| ui.label("hello")).response;

		mem.store(ui.ctx(), Id::from("imagery_settings"));

		response
	}
}
