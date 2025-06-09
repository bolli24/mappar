use derive_new::new;
use egui::{Key, RichText, Widget};

use crate::map::map_data::MapData;

use super::map_widget::{map_memory::MapMemory, map_state::EditMode};

#[derive(new)]
pub struct EditBar<'a> {
	edit_mode: EditMode,
	update_edit_mode: &'a mut Option<EditMode>,
	map_memory: &'a mut MapMemory,
	map_data: &'a mut MapData,
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
				if ui
					.selectable_label(self.edit_mode == mode, RichText::new(icon.to_string()).size(20.0))
					.on_hover_text(tooltip)
					.clicked() || (ui.ctx().memory(|m| m.focused().is_none()) && ui.ctx().input(|i| i.key_pressed(key)))
				{
					*self.update_edit_mode = Some(mode);

					if mode != EditMode::Select {
						self.map_memory.selection.clear(self.map_data);
					}
				}
			}
		})
		.response
	}
}
