use egui::{Modifiers, PointerButton};
use glam::DVec2;
use statig::prelude::*;

use crate::{
	map::map_data::{ElementId, MapData},
	message_error,
	ui::messages::Messages,
	util::{convert::ToRect, quick_set::QuickSet},
};

use super::{map_memory::MapMemory, render::DetailedElement, view::View};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum EditMode {
	#[default]
	Select,
	DrawPen,
}

#[derive(Debug, Clone, Default)]
pub struct MapStateMachine {
	pub selection_box: Option<(DVec2, DVec2)>,
	edit_mode: EditMode,
	pointer_down_delta: DVec2,
	pointer_down_pos: Option<DVec2>,
}

pub struct Context<'a> {
	pub ui_context: &'a egui::Context,
	pub response: egui::Response,
	pub map_data: &'a mut MapData,
	pub view: &'a View,
	pub interactive: bool,
	pub mem: &'a mut MapMemory,
	pub world_delta: DVec2,
	pub render_data: &'a [DetailedElement],
	pub hover_elements: &'a mut QuickSet<ElementId>,
	pub messages: Messages,
	pub update_edit_mode: Option<EditMode>,
}

#[state_machine(initial = "State::initial()", state(derive(Debug)), superstate(derive(Debug)))]
impl MapStateMachine {
	const fn select_default() -> State {
		State::Select {
			last_clicked_element: None,
		}
	}

	#[state]
	fn initial() -> Response<State> {
		Response::Transition(State::Select {
			last_clicked_element: None,
		})
	}

	pub fn edit_mode(&self) -> EditMode {
		self.edit_mode
	}

	#[superstate]
	fn default(&mut self, context: &mut Context<'_>) -> Response<State> {
		let Context {
			response,
			update_edit_mode,
			..
		} = context;

		if let Some(new_mode) = update_edit_mode.take() {
			self.edit_mode = new_mode;

			return match self.edit_mode {
				EditMode::Select => Response::Transition(State::Select {
					last_clicked_element: None,
				}),
				EditMode::DrawPen => Response::Handled,
			};
		}

		if response.drag_started_by(PointerButton::Middle)
			|| (response.ctx.input(|i| i.modifiers == Modifiers::ALT)
				&& response.drag_started_by(PointerButton::Primary))
		{
			return Response::Transition(State::MoveView {});
		}

		Response::Handled
	}

	#[state(superstate = "default")]
	fn select(&mut self, context: &mut Context<'_>, last_clicked_element: &mut Option<ElementId>) -> Response<State> {
		let Context {
			ui_context,
			response,
			view,
			interactive,
			mem,
			world_delta,
			hover_elements,
			..
		} = context;

		if !*interactive {
			return Response::Handled;
		}

		if ui_context.input(|i| i.pointer.button_down(PointerButton::Primary)) {
			view.pointer_world_pos
				.map(|pos| self.pointer_down_pos.get_or_insert(pos));
			self.pointer_down_delta += *world_delta;
		} else {
			self.pointer_down_pos = None;
			self.pointer_down_delta = DVec2::ZERO;
		}

		if response.drag_started_by(PointerButton::Primary) && !response.ctx.input(|i| i.modifiers == Modifiers::ALT) {
			if let Some(last) = last_clicked_element {
				if mem.selection.elements.contains(last) {
					return Response::Transition(State::MoveElements {
						total_delta: DVec2::ZERO,
						initial_delta: self.pointer_down_delta,
					});
				}
			} else {
				return Response::Transition(State::BoxSelect {
					pointer_down_pos: self.pointer_down_pos,
				});
			}
		}

		let shift = ui_context.input(|i| i.modifiers.shift);
		if response.contains_pointer() && ui_context.input(|i| i.pointer.button_down(PointerButton::Primary)) {
			if let Some(hover_element) = hover_elements.single() {
				*last_clicked_element = Some(*hover_element);
				if response.ctx.input(|i| i.modifiers.shift_only()) {
					mem.selection.elements.insert(*hover_element);
				} else if response.ctx.input(|i| i.modifiers.command_only()) {
					mem.selection.elements.remove(hover_element);
				} else {
					if !shift && !mem.selection.elements.contains(hover_element) {
						mem.selection.elements.clear();
					}
					mem.selection.elements.insert(*hover_element);
				}
			}
		}
		if response.clicked_by(PointerButton::Primary) && hover_elements.is_empty() {
			mem.selection.elements.clear();
		}

		Response::Super
	}

	#[state(superstate = "default")]
	fn move_view(context: &mut Context<'_>) -> Response<State> {
		context.mem.camera_pos -= context.world_delta;
		let response = &context.response;

		if response.dragged_by(PointerButton::Middle)
			|| (response.ctx.input(|i| i.modifiers == Modifiers::ALT) && response.dragged_by(PointerButton::Primary))
		{
			Response::Handled
		} else {
			Response::Transition(Self::select_default())
		}
	}

	#[state]
	fn move_elements(context: &mut Context<'_>, total_delta: &mut DVec2, initial_delta: &mut DVec2) -> Response<State> {
		*total_delta += context.world_delta + *initial_delta;
		MapData::raw_move_by(
			&mut context.map_data.elements,
			&context.mem.selection.elements,
			context.world_delta + *initial_delta,
		);

		*initial_delta = DVec2::ZERO;

		if context.response.drag_stopped_by(PointerButton::Primary) {
			let edit = context
				.map_data
				.move_by(&context.mem.selection.elements, *total_delta, true);
			let result = context.map_data.edit(edit);
			if let Err(err) = result {
				message_error!(context.messages, "Error moving elements: {err}");
			}
			Response::Transition(Self::select_default())
		} else if context.response.clicked_by(PointerButton::Secondary) {
			MapData::raw_move_by(
				&mut context.map_data.elements,
				&context.mem.selection.elements,
				-*total_delta,
			);
			Response::Transition(Self::select_default())
		} else {
			Response::Handled
		}
	}

	#[state(exit_action = "exit_box_select")]
	fn box_select(&mut self, context: &mut Context<'_>, pointer_down_pos: &mut Option<DVec2>) -> Response<State> {
		let selection_box = if let Some(pointer) = context.view.pointer_world_pos {
			let selection_box = self
				.selection_box
				.get_or_insert((pointer_down_pos.unwrap_or(pointer), pointer));
			selection_box.1 = pointer;
			Some(selection_box.to_rect())
		} else {
			None
		};

		*context.hover_elements = QuickSet::from_iter(
			context
				.render_data
				.iter()
				.filter_map(|e| e.element.in_box_selection(selection_box).then_some(e.id)),
		);

		if context.response.drag_stopped_by(PointerButton::Primary) {
			if context.response.ctx.input(|i| i.modifiers.shift_only()) {
				for element in context.hover_elements.iter() {
					context.mem.selection.elements.insert(*element);
				}
			} else if context.response.ctx.input(|i| i.modifiers.command_only()) {
				for element in context.hover_elements.iter() {
					context.mem.selection.elements.remove(element);
				}
			} else {
				context.mem.selection.elements.clear();
				for element in context.hover_elements.iter() {
					context.mem.selection.elements.insert(*element);
				}
			}

			Response::Transition(Self::select_default())
		} else if context.response.clicked_by(PointerButton::Secondary) {
			Response::Transition(Self::select_default())
		} else {
			Response::Handled
		}
	}

	#[action]
	fn exit_box_select(&mut self) {
		self.selection_box = None;
	}
}
