use egui::{Modifiers, PointerButton};
use glam::DVec2;
use itertools::Itertools;
use statig::prelude::*;

use super::{map_memory::MapMemory, render::DetailedElement, view::View};
use crate::map::element::{Element, ElementId, ElementType, is_closed};
use crate::{
	dbgtxt,
	map::map_data::MapData,
	message_error,
	ui::{
		debug_text::DebugText,
		map_widget::{
			preview::Preview,
			render::{RenderElement, closest_point_on_polyline, point_in_polygon},
		},
		messages::Messages,
	},
	util::{
		convert::{ToPos2, ToRect},
		quick_set::QuickSet,
	},
};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum EditMode {
	#[default]
	Select,
	DrawPen,
}

#[derive(Debug, Clone, Default)]
pub struct MapStateMachine {
	pub preview: Preview,
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
	pub render_data: &'a mut [DetailedElement],
	pub hover_elements: &'a mut QuickSet<ElementId>,
	pub messages: Messages,
	pub update_edit_mode: Option<EditMode>,
	pub debug_text: Option<&'a mut DebugText>,
}

#[state_machine(initial = "State::initial()", state(derive(Debug)), superstate(derive(Debug)))]
impl MapStateMachine {
	const fn select_default() -> State {
		State::Select {
			last_clicked_element: None,
		}
	}

	#[state]
	const fn initial() -> Outcome<State> {
		Transition(State::Select {
			last_clicked_element: None,
		})
	}
	#[must_use]
	pub const fn edit_mode(&self) -> EditMode {
		self.edit_mode
	}

	#[superstate]
	fn default(&mut self, context: &mut Context<'_>) -> Outcome<State> {
		let Context {
			response,
			update_edit_mode,
			..
		} = context;

		if let Some(new_mode) = update_edit_mode.take() {
			self.edit_mode = new_mode;

			return match self.edit_mode {
				EditMode::Select => Transition(State::Select {
					last_clicked_element: None,
				}),
				EditMode::DrawPen => Transition(State::Draw {
					first_node_or_way: None,
				}),
			};
		}

		if response.drag_started_by(PointerButton::Middle)
			|| (response.ctx.input(|i| i.modifiers == Modifiers::ALT)
				&& response.drag_started_by(PointerButton::Primary))
		{
			return Transition(State::MoveView {});
		}

		Handled
	}

	#[state(superstate = "default")]
	fn select(&mut self, context: &mut Context<'_>, last_clicked_element: &mut Option<ElementId>) -> Outcome<State> {
		let Context {
			ui_context,
			response,
			view,
			interactive,
			mem,
			world_delta,
			hover_elements,
			debug_text,
			map_data,
			..
		} = context;

		if let Some(element_id) = last_clicked_element
			&& element_id.1 == ElementType::Way
		{
			let element = &map_data.elements[element_id];
			if let Element::Way { nodes } = &element.element {
				let closed = is_closed(nodes);
				dbgtxt!(debug_text, "Closed: {closed}");
			}
		}

		if !*interactive {
			return Handled;
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
					return Transition(State::MoveElements {
						total_delta: DVec2::ZERO,
						initial_delta: self.pointer_down_delta,
					});
				}
			} else {
				return Transition(State::BoxSelect {
					pointer_down_pos: self.pointer_down_pos,
				});
			}
		}

		if !response.contains_pointer() {
			return Super;
		}

		let mut add_element = |element: ElementId| {
			if response.ctx.input(|i| i.modifiers.shift_only()) {
				mem.selection.elements.insert(element);
			} else if response.ctx.input(|i| i.modifiers.command_only()) {
				mem.selection.elements.remove(&element);
			} else {
				if !ui_context.input(|i| i.modifiers.shift) && !mem.selection.elements.contains(&element) {
					mem.selection.elements.clear();
				}
				mem.selection.elements.insert(element);
			}
		};

		if let Some(pointer_pos) = view.pointer_world_pos
			&& response.clicked_by(PointerButton::Secondary)
			&& hover_elements.is_empty()
		{
			let pointer_pos2 = pointer_pos.to_pos2();
			let mut elements = context
				.render_data
				.iter_mut()
				.filter(|element| match &element.element {
					RenderElement::Area(verts) | RenderElement::Path(verts, true) => {
						element.bounding_box.contains(pointer_pos2) && point_in_polygon(pointer_pos, verts)
					}
					_ => false,
				})
				.map(|element| {
					let area = element.bounding_box.area();
					debug_assert_ne!(area, 0.0);
					(element, area)
				})
				.collect::<Vec<_>>();
			elements.sort_by(|a, b| f32::total_cmp(&a.1, &b.1));

			if let Some((first, _)) = elements.iter_mut().next() {
				add_element(first.id);
			}

			return Super;
		}

		if let Some(hover_element) = hover_elements.single()
			&& ui_context.input(|i| i.pointer.button_down(PointerButton::Primary))
		{
			add_element(*hover_element);
			*last_clicked_element = Some(*hover_element);
		}
		if response.clicked_by(PointerButton::Primary) && hover_elements.is_empty() {
			mem.selection.elements.clear();
		}

		Super
	}

	#[state(superstate = "default")]
	fn move_view(context: &mut Context<'_>) -> Outcome<State> {
		context.mem.camera_pos -= context.world_delta;
		let response = &context.response;

		if response.dragged_by(PointerButton::Middle)
			|| (response.ctx.input(|i| i.modifiers == Modifiers::ALT) && response.dragged_by(PointerButton::Primary))
		{
			Handled
		} else {
			Transition(Self::select_default())
		}
	}

	#[state]
	fn move_elements(context: &mut Context<'_>, total_delta: &mut DVec2, initial_delta: &mut DVec2) -> Outcome<State> {
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
			Transition(Self::select_default())
		} else if context.response.clicked_by(PointerButton::Secondary) {
			MapData::raw_move_by(
				&mut context.map_data.elements,
				&context.mem.selection.elements,
				-*total_delta,
			);
			Transition(Self::select_default())
		} else {
			Handled
		}
	}

	#[state(exit_action = "exit_box_select")]
	fn box_select(&mut self, context: &mut Context<'_>, pointer_down_pos: &Option<DVec2>) -> Outcome<State> {
		let selection_box = if let Some(pointer) = context.view.pointer_world_pos {
			let selection_box = self
				.selection_box
				.get_or_insert_with(|| (pointer_down_pos.unwrap_or(pointer), pointer));
			selection_box.1 = pointer;
			Some(selection_box.to_rect())
		} else {
			None
		};

		*context.hover_elements = context
			.render_data
			.iter()
			.filter_map(|e| e.element.in_box_selection(selection_box).then_some(e.id))
			.collect::<QuickSet<_>>();

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

			Transition(Self::select_default())
		} else if context.response.clicked_by(PointerButton::Secondary) {
			Transition(Self::select_default())
		} else {
			Handled
		}
	}

	#[action]
	const fn exit_box_select(&mut self) {
		self.selection_box = None;
	}

	#[state(superstate = "default", exit_action = "exit_draw")]
	fn draw(&mut self, context: &mut Context<'_>, first_node_or_way: &mut Option<ElementId>) -> Outcome<State> {
		let Context {
			response,
			view,
			mem,
			map_data,
			hover_elements,
			render_data,
			..
		} = context;

		if response.clicked_by(PointerButton::Secondary) || response.ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
			return Transition(State::Draw {
				first_node_or_way: None,
			});
		}

		let mut way_hover_segment = None;

		if let Some(mut pos) = view.pointer_world_pos {
			if let Some(hover_element) = hover_elements.single() {
				// Snap to hovered element
				pos = match &hover_element {
					id if id.1 == ElementType::Node => map_data
						.get_node_pos(**id)
						.expect("Hovered node must exist.")
						.to_web_mercator(),
					id if id.1 == ElementType::Way => {
						let way = map_data.get(id).expect("New element must exist");
						let Element::Way { nodes } = &way.element else {
							unreachable!("New element must be way")
						};
						let _closed = is_closed(nodes);

						let points: Vec<DVec2> = nodes
							.iter()
							.map(|n| map_data.get_node_pos(*n).expect("Node must exist").to_web_mercator())
							.collect();

						way_hover_segment = closest_point_on_polyline(&points, nodes, pos);
						if let Some((new_pos, _, _)) = way_hover_segment {
							new_pos
						} else {
							pos
						}
					}
					_ => unreachable!("Must never hover over relation."),
				}
			}

			match *first_node_or_way {
				Some(id) if id.1 == ElementType::Node => self.preview = Preview::NewNodeToPath(id, pos),
				Some(id) if id.1 == ElementType::Way => {
					let way = map_data.get(&id).expect("New element must exist");
					let Element::Way { nodes } = &way.element else {
						unreachable!("New element must be way")
					};
					self.preview = Preview::NewNodeToPath(*nodes.last().expect("Way must have a last element."), pos);
				}
				None => {
					self.preview = Preview::Node(pos);
				}
				_ => unreachable!("Invalid element for first_node_or_way"),
			}
		} else {
			self.preview = Preview::None;
		}

		if let Some(pos) = view.pointer_world_pos
			&& (response.ctx.input(|i| i.pointer.button_pressed(PointerButton::Primary)))
		{
			let node_id = if let Some(hover_element) = hover_elements.single() {
				match &hover_element {
					ElementId(_, ElementType::Node) => {
						if Some(*hover_element) == *first_node_or_way {
							return Transition(State::Draw {
								first_node_or_way: None,
							});
						}

						*hover_element
					}
					id if id.1 == ElementType::Way => {
						// We could actually be hovering over multiple ways. So we find all that
						// contain the same two nodes of the hoverd segment.
						let way = map_data.get(id).expect("New element must exist");
						let Element::Way { nodes: _ } = &way.element else {
							unreachable!("New element must be way")
						};

						let way_hover_segment = way_hover_segment.expect("Hovered segment must be defined.");

						let ways = render_data
							.iter()
							.filter(|e| e.id.1 == ElementType::Way)
							.filter_map(|e| {
								let Element::Way { nodes } = &map_data.elements[&e.id].element else {
									unreachable!("Element must be way.");
								};
								nodes
									.iter()
									.tuple_windows()
									.position(|(a, b)| {
										(*a == way_hover_segment.1 && *b == way_hover_segment.2)
											|| (*b == way_hover_segment.1 && *a == way_hover_segment.2)
									})
									.map(|i| (e.id, i + 1))
							})
							.collect();

						let edit = map_data.add_node_on_way_segments(pos, ways);
						map_data
							.edit(edit)
							.expect("Adding node to way segments must not error.");

						// TODO: transition to moving node
						return Transition(State::Draw {
							first_node_or_way: None,
						});
					}
					_ => unreachable!("Must never hover over relation."),
				}
			} else {
				// Add new node
				let edit = map_data.new_node(pos);
				let new_node_id = edit.0[0];
				map_data.edit(edit).expect("Creating node must not error.");
				new_node_id
			};

			match *first_node_or_way {
				Some(first_id) if first_id.1 == ElementType::Node => {
					let edit = map_data.new_way(vec![first_id, node_id]);
					let new_way_id = edit.0[0];
					*first_node_or_way = Some(new_way_id);
					map_data.edit(edit).expect("Creating way must not error.");
					mem.selection.elements.clear();
					mem.selection.elements.insert(new_way_id);
				}
				Some(first_id) if first_id.1 == ElementType::Way => {
					let edit = map_data.add_node_to_way_end(first_id, node_id);
					let way_id = edit.0[0];
					debug_assert_eq!(first_id.0, way_id.0);
					*first_node_or_way = Some(way_id);

					let mut child_node = false;
					if let Element::Way { nodes } = &map_data.get(&way_id).expect("Last new element must exist").element
						&& nodes.contains(&node_id)
					{
						child_node = true;
					}

					map_data.edit(edit).expect("Adding node to way end must not error.");

					if child_node {
						return Transition(State::Draw {
							first_node_or_way: None,
						});
					}
				}
				None => {
					mem.selection.elements.clear();
					*first_node_or_way = Some(node_id);
					mem.selection.elements.insert(node_id);
				}
				_ => unreachable!(),
			}
		}

		Super
	}

	#[action]
	const fn exit_draw(&mut self) {
		self.preview = Preview::None;
	}
}
