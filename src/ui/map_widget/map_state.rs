use egui::{Modifiers, PointerButton};
use glam::DVec2;
use statig::prelude::*;

use crate::{map::map_data::MapData, ui::edit_bar::EditMode};

use super::{map_memory::MapMemory, render::DetailedElement};

#[derive(Debug, Clone)]
pub struct MapState2 {
	// TODO: edit_mode: EditMode
}

pub struct Context<'a> {
	pub ui_context: &'a egui::Context,
	pub response: egui::Response,
	pub map_data: &'a mut MapData,
	pub interactive: bool,
	pub mem: &'a mut MapMemory,
	pub world_delta: DVec2,
	pub edit_mode: EditMode,
	pub hover_element: Option<&'a DetailedElement>,
}

#[state_machine(initial = "State::default()", state(derive(Debug)), superstate(derive(Debug)))]
impl MapState2 {
	#[state]
	fn default(context: &mut Context<'_>) -> Response<State> {
		if !context.interactive {
			return Response::Handled;
		}

		let response = &context.response;
		if response.drag_started_by(PointerButton::Middle)
			|| (response.ctx.input(|i| i.modifiers == Modifiers::ALT)
				&& response.drag_started_by(PointerButton::Primary))
		{
			return Response::Transition(State::MoveView {});
		}

		if context.response.contains_pointer()
			&& context
				.ui_context
				.input(|i| i.pointer.button_down(PointerButton::Primary))
			&& context.edit_mode == EditMode::Select
		{
			if let Some(hover_element) = context.hover_element {
				if !context.ui_context.input(|i| i.modifiers.shift)
					&& !context.mem.selection.elements.contains(&hover_element.id)
				{
					context.mem.selection.elements.clear();
				}
				context.mem.selection.elements.insert(hover_element.id);
			}
		}

		if response.drag_started_by(PointerButton::Primary)
			&& !context.mem.selection.elements.is_empty()
			&& context.edit_mode == EditMode::Select
		{
			return Response::Transition(State::MoveElements {});
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
			Response::Transition(State::Default {})
		}
	}

	#[state]
	fn move_elements(context: &mut Context<'_>) -> Response<State> {
		context
			.map_data
			.move_by(&context.mem.selection.elements, &context.world_delta);

		if context.response.drag_stopped_by(PointerButton::Primary) {
			Response::Transition(State::Default {})
		} else {
			Response::Handled
		}
	}
}
