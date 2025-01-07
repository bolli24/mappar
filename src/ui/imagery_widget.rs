use crate::imagery::{Imageries, ImageryId};
use crate::ui::memory::Memory;
use crate::util::convert::CountryFlag;
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

// TODO: Maybe use string interning for ids.
impl Widget for ImageriesWidget<'_> {
	fn ui(self, ui: &mut Ui) -> Response {
		let mut mem = ImageriesMemory::load_or_default(ui.ctx(), Id::from("imagery_settings"));

		TopBottomPanel::top("top_panel").show_inside(ui, |ui| {
			menu::bar(ui, |ui| {
				if ui.button("Fetch JOSM imagery 🇺🇸").clicked() {
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
				ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);

				for (id, imagery) in &self.imageries.entries {
					ui.label(imagery.country.and_then(|c| c.flag()).unwrap_or("".to_string()));
					ui.selectable_value(&mut mem.selected, Some(id.clone()), imagery.name());
				}
			})
		});

		let response = CentralPanel::default()
			.show_inside(ui, |ui| match &mem.selected {
				Some(selected) => ui.heading(self.imageries.entries.get(selected).unwrap().name()),
				None => ui.label("Nothing is selected"),
			})
			.response;

		mem.store(ui.ctx(), Id::from("imagery_settings"));

		response
	}
}
