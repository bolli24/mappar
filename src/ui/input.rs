use egui::{Context, InputState, Key, Modifiers};
use serde::{Deserialize, Serialize};
use smallvec::{SmallVec, smallvec};
use std::collections::HashMap;
use strum::VariantArray;

#[derive(Debug, Default, Clone)]
pub struct KeyBinds {
	bindings: HashMap<Command, SmallVec<[KeyBind; 1]>>,
	pressed_commands: HashMap<Command, bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, strum::VariantArray)]
#[serde(rename_all = "snake_case")]
pub enum Command {
	Delete,
	ReloadStyles,
	ToggleDebugPanel,
	Undo,
	Redo,
	ModeSelect,
	ModeDraw,
	RotateImageries,
}

#[derive(Debug, Clone, Copy)]
pub struct KeyBind(pub Key, pub Modifiers);

impl From<KeyBind> for SmallVec<[KeyBind; 1]> {
	fn from(value: KeyBind) -> Self {
		smallvec![value]
	}
}

impl KeyBind {
	pub fn matches(self, input: &InputState) -> bool {
		input.key_pressed(self.0) && input.modifiers == self.1
	}
}

impl KeyBinds {
	fn default_bindings() -> HashMap<Command, SmallVec<[KeyBind; 1]>> {
		HashMap::from([
			(Command::ReloadStyles, smallvec![KeyBind(Key::F5, Modifiers::NONE)]),
			(Command::ToggleDebugPanel, smallvec![KeyBind(Key::F3, Modifiers::NONE)]),
			(
				Command::Undo,
				KeyBind(Key::Z, Modifiers::CTRL | Modifiers::COMMAND).into(),
			),
			(
				Command::Redo,
				KeyBind(Key::Y, Modifiers::CTRL | Modifiers::COMMAND).into(),
			),
			(
				Command::Delete,
				smallvec![KeyBind(Key::X, Modifiers::NONE), KeyBind(Key::Delete, Modifiers::NONE)],
			),
			(Command::ModeSelect, KeyBind(Key::S, Modifiers::NONE).into()),
			(Command::ModeDraw, KeyBind(Key::D, Modifiers::NONE).into()),
			(Command::RotateImageries, KeyBind(Key::Backtick, Modifiers::NONE).into()),
		])
	}

	pub fn new(messages: &mut super::messages::Messages) -> Self {
		let bindings = Self::default_bindings();
		Self::warn_unbound(&bindings, messages);
		Self {
			bindings,
			pressed_commands: HashMap::new(),
		}
	}

	fn warn_unbound(bindings: &HashMap<Command, SmallVec<[KeyBind; 1]>>, messages: &mut super::messages::Messages) {
		for cmd in Command::VARIANTS {
			if !bindings.contains_key(cmd) {
				crate::message_warn!(messages, "No keybindings defined for command: {cmd:?}");
			}
		}
	}

	pub fn update_pressed_commands(&mut self, ctx: &Context) {
		let has_focused_element = ctx.memory(|mem| mem.focused().is_some());
		self.pressed_commands.clear();

		ctx.input(|input| {
			for e in &input.events {
				if let egui::Event::Key {
					pressed: true,
					key,
					modifiers,
					repeat,
					..
				} = e
				{
					for (cmd, binds) in &self.bindings {
						if binds
							.iter()
							.any(|bind| bind.0 == *key && bind.1.matches_logically(*modifiers))
						{
							let should_block =
								Self::key_requires_no_focused_element(*key, *modifiers) && has_focused_element;

							if !should_block {
								self.pressed_commands.insert(*cmd, *repeat);
								break;
							}
						}
					}
				}
			}
		});
	}

	fn key_requires_no_focused_element(key: Key, modifiers: Modifiers) -> bool {
		if modifiers != Modifiers::NONE {
			return false;
		}

		matches!(
			key,
			Key::A
				| Key::B | Key::C
				| Key::D | Key::E
				| Key::F | Key::G
				| Key::H | Key::I
				| Key::J | Key::K
				| Key::L | Key::M
				| Key::N | Key::O
				| Key::P | Key::Q
				| Key::R | Key::S
				| Key::T | Key::U
				| Key::V | Key::W
				| Key::X | Key::Y
				| Key::Z | Key::Num0
				| Key::Num1 | Key::Num2
				| Key::Num3 | Key::Num4
				| Key::Num5 | Key::Num6
				| Key::Num7 | Key::Num8
				| Key::Num9 | Key::Backspace
				| Key::Space | Key::Tab
				| Key::Home | Key::End
				| Key::Enter | Key::Delete
				| Key::Slash | Key::Backslash
				| Key::Period
				| Key::Comma | Key::Semicolon
				| Key::Quote | Key::Backtick
				| Key::Minus | Key::PageUp
				| Key::PageDown
				| Key::Insert
				| Key::ArrowDown
				| Key::ArrowLeft
				| Key::ArrowRight
				| Key::ArrowUp
		)
	}

	pub fn pressed_once(&self, cmd: Command) -> bool {
		self.pressed_commands.get(&cmd).is_some_and(|&is_repeat| !is_repeat)
	}

	pub fn pressed(&self, cmd: Command) -> bool {
		self.pressed_commands.contains_key(&cmd)
	}
}
