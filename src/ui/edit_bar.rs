use derive_new::new;
use egui::{Button, Key, RichText, Widget};

use crate::map::map_data::MapData;

use super::map_widget::map_memory::MapMemory;

#[derive(new)]
pub struct EditBar<'a> {
	edit_mode: &'a mut EditMode,
	map_memory: &'a mut MapMemory,
	map_data: &'a mut MapData,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EditMode {
	Select,
	DrawPen,
}

struct EditButton<'a>(&'a str, &'a str, EditMode, Key);

const MODE_BUTTONS: [EditButton; 2] = [
	EditButton(egui_phosphor::bold::SELECTION, "Select stuff", EditMode::Select, Key::S),
	EditButton(
		egui_phosphor::bold::PEN_NIB,
		"Draw nodes and ways",
		EditMode::DrawPen,
		Key::A,
	),
];

impl Widget for EditBar<'_> {
	fn ui(self, ui: &mut egui::Ui) -> egui::Response {
		ui.horizontal(|ui| {
			for EditButton(icon, tooltip, mode, key) in MODE_BUTTONS {
				if Button::new(RichText::new(icon.to_string()).size(20.0))
					.selected(*self.edit_mode == mode)
					.ui(ui)
					.on_hover_text(tooltip)
					.clicked() || ui.ctx().input(|i| i.key_pressed(key))
				{
					*self.edit_mode = mode;

					if mode != EditMode::Select {
						self.map_memory.selection.clear(self.map_data);
					}
				}
			}
		})
		.response
	}
}
