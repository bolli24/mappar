use derive_new::new;
use egui::{RichText, Widget};

use super::map_widget::{map_memory::MapMemory, map_state::EditMode};
use crate::map::map_data::MapData;
use crate::ui::input::{Command, KeyBinds};

#[derive(new)]
pub struct EditBar<'a> {
	edit_mode: EditMode,
	update_edit_mode: &'a mut Option<EditMode>,
	map_memory: &'a mut MapMemory,
	map_data: &'a mut MapData,
	key_binds: &'a KeyBinds,
}

struct EditButton<'a>(&'a str, &'a str, EditMode, Command);

const MODE_BUTTONS: [EditButton; 2] = [
	EditButton(
		egui_phosphor::bold::CURSOR,
		"Select stuff",
		EditMode::Select,
		Command::ModeSelect,
	),
	EditButton(
		egui_phosphor::bold::PEN_NIB,
		"Draw nodes and ways",
		EditMode::DrawPen,
		Command::ModeDraw,
	),
];

impl Widget for EditBar<'_> {
	fn ui(self, ui: &mut egui::Ui) -> egui::Response {
		ui.horizontal(|ui| {
			for EditButton(icon, tooltip, mode, command) in MODE_BUTTONS {
				if ui
					.selectable_label(self.edit_mode == mode, RichText::new(icon.to_string()).size(20.0))
					.on_hover_text(tooltip)
					.clicked() || self.key_binds.pressed(command)
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
