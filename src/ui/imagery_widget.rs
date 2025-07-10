use crate::imagery::{Imageries, ImageryId};
use crate::ui::memory::Memory;
use crate::util::convert::CountryFlag;
use derive_new::new;
#[cfg(not(target_arch = "wasm32"))]
use directories::ProjectDirs;
use egui::{CentralPanel, Label, MenuBar, Response, Sense, SidePanel, TopBottomPanel, Ui, Widget};
use egui_extras::{Column, TableBuilder};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use super::messages::Messages;

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

#[derive(new)]
pub struct ImageriesWidget<'a> {
	imageries: Arc<Mutex<Imageries>>,
	messages: &'a Messages,
	#[cfg(not(target_arch = "wasm32"))]
	project_dirs: &'a ProjectDirs,
}

impl Widget for ImageriesWidget<'_> {
	fn ui(self, ui: &mut Ui) -> Response {
		let mut mem = ImageriesMemory::load_or_default(ui.ctx(), Id::from("imagery_settings"));
		let imageries = self.imageries.lock().unwrap();

		TopBottomPanel::top("top_panel").show_inside(ui, |ui| {
			MenuBar::new().ui(ui, |ui| {
				if ui.button("Update JOSM imagery").clicked() {
					crate::imagery::update_josm_imagery(
						#[cfg(not(target_arch = "wasm32"))]
						self.project_dirs.data_dir().join(crate::imagery::JOSM_FILE),
						self.messages.clone(),
						self.imageries.clone(),
					);
				}
			});
		});

		let selected_index = mem
			.selected
			.clone()
			.and_then(|selected| imageries.entries.get_index_of(&selected));

		SidePanel::left("imagery_list").show_inside(ui, |ui| {
			TableBuilder::new(ui)
				.striped(true)
				.column(Column::auto())
				.column(Column::remainder())
				.sense(Sense::CLICK)
				.body(|body| {
					body.rows(16.0, imageries.entries.len(), |mut row| {
						let (id, imagery) = imageries
							.entries
							.get_index(row.index())
							.expect("Index should be valid.");

						row.set_selected(Some(row.index()) == selected_index);

						row.col(|ui| {
							ui.label(imagery.country.and_then(|c| c.flag()).unwrap_or(String::new()));
						});
						row.col(|ui| {
							Label::new(imagery.name()).selectable(false).ui(ui);
						});

						if row.response().clicked() {
							mem.selected = Some(id.clone());
						}
					});
				});
		});

		let response = CentralPanel::default()
			.show_inside(ui, |ui| match &mem.selected {
				Some(selected) => ui.heading(imageries.entries.get(selected).unwrap().name()),
				None => ui.label("Nothing is selected"),
			})
			.response;

		mem.store(ui.ctx(), Id::from("imagery_settings"));

		response
	}
}
