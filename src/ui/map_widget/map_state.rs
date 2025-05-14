use egui::{Modifiers, PointerButton};
use glam::DVec2;
use statig::prelude::*;

use crate::{
	map::map_data::MapData,
	message_error,
	ui::{edit_bar::EditMode, messages::Messages},
};

use super::{map_memory::MapMemory, render::DetailedElement, view::View};

#[derive(Debug, Clone, Default)]
pub struct MapStateMachine {
	// TODO: edit_mode: EditMode
	pub selection_box: Option<(DVec2, DVec2)>,
}

pub struct Context<'a> {
	pub ui_context: &'a egui::Context,
	pub response: egui::Response,
	pub map_data: &'a mut MapData,
	pub view: &'a View,
	pub interactive: bool,
	pub mem: &'a mut MapMemory,
	pub world_delta: DVec2,
	pub edit_mode: EditMode,
	pub hover_element: Option<&'a DetailedElement>,
	pub messages: Messages,
}

#[state_machine(
	initial = "State::default(DVec2::ZERO, None)",
	state(derive(Debug)),
	superstate(derive(Debug))
)]
impl MapStateMachine {
	fn default_default() -> State {
		State::Default {
			pointer_down_delta: DVec2::ZERO,
			pointer_down_pos: None,
		}
	}

	#[state]
	fn default(
		context: &mut Context<'_>,
		pointer_down_delta: &mut DVec2,
		pointer_down_pos: &mut Option<DVec2>,
	) -> Response<State> {
		let Context {
			ui_context,
			response,
			map_data: _,
			view,
			interactive,
			mem,
			world_delta,
			edit_mode,
			hover_element,
			messages: _,
		} = context;

		if !*interactive {
			return Response::Handled;
		}

		if ui_context.input(|i| i.pointer.button_down(PointerButton::Primary)) {
			view.pointer_world_pos.map(|pos| pointer_down_pos.get_or_insert(pos));
			*pointer_down_delta += *world_delta;
		} else {
			*pointer_down_pos = None;
			*pointer_down_delta = DVec2::ZERO;
		}

		if response.drag_started_by(PointerButton::Middle)
			|| (response.ctx.input(|i| i.modifiers == Modifiers::ALT)
				&& response.drag_started_by(PointerButton::Primary))
		{
			return Response::Transition(State::MoveView {});
		}

		if response.drag_started_by(PointerButton::Primary) && *edit_mode == EditMode::Select {
			if mem.selection.elements.is_empty() {
				return Response::Transition(State::BoxSelect {
					pointer_down_pos: *pointer_down_pos,
				});
			} else {
				return Response::Transition(State::MoveElements {
					total_delta: DVec2::ZERO,
					initial_delta: *pointer_down_delta,
				});
			}
		}

		let shift = ui_context.input(|i| i.modifiers.shift);
		if response.contains_pointer()
			&& ui_context.input(|i| i.pointer.button_down(PointerButton::Primary))
			&& *edit_mode == EditMode::Select
		{
			if let Some(hover_element) = hover_element {
				if !shift && !mem.selection.elements.contains(&hover_element.id) {
					mem.selection.elements.clear();
				}
				mem.selection.elements.insert(hover_element.id);
			}
		}
		if *edit_mode == EditMode::Select && response.clicked_by(PointerButton::Primary) && hover_element.is_none() {
			mem.selection.elements.clear();
		}

		Response::Handled
	}

	#[state]
	fn move_view(context: &mut Context<'_>) -> Response<State> {
		context.mem.camera_pos -= context.world_delta;
		let response = &context.response;

		if response.dragged_by(PointerButton::Middle)
			|| (response.ctx.input(|i| i.modifiers == Modifiers::ALT) && response.dragged_by(PointerButton::Primary))
		{
			Response::Handled
		} else {
			Response::Transition(Self::default_default())
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
			Response::Transition(Self::default_default())
		} else if context.response.clicked_by(PointerButton::Secondary) {
			MapData::raw_move_by(
				&mut context.map_data.elements,
				&context.mem.selection.elements,
				-*total_delta,
			);
			Response::Transition(Self::default_default())
		} else {
			Response::Handled
		}
	}

	#[state(exit_action = "exit_box_select")]
	fn box_select(&mut self, context: &mut Context<'_>, pointer_down_pos: &mut Option<DVec2>) -> Response<State> {
		let _selection_box = if let Some(pointer) = context.view.pointer_world_pos {
			let selection_box = self
				.selection_box
				.get_or_insert((pointer_down_pos.unwrap_or(pointer), pointer));
			selection_box.1 = pointer;
			Some(selection_box)
		} else {
			None
		};

		if context.response.drag_stopped_by(PointerButton::Primary) {
			// TODO: update selection
			Response::Transition(Self::default_default())
		} else if context.response.clicked_by(PointerButton::Secondary) {
			Response::Transition(Self::default_default())
		} else {
			Response::Handled
		}
	}

	#[action]
	fn exit_box_select(&mut self) {
		self.selection_box = None;
	}
}
