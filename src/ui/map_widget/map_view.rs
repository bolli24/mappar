use super::css::Styles;
use super::css::context::StyleContext;
use super::draw_list::DrawList;
use super::map_memory::MapMemory;
use super::render::DetailedElement;
use super::view::View;
use super::{ZOOM_SPEED, draw_element, draw_tiles, pointer_world_pos, scale};
use crate::dbgtxt;
use crate::imagery::ImageryRef;
use crate::imagery::imagery_fetcher::ImageryFetcher;
use crate::map::element::Element;
use crate::map::map_data::MapData;
use crate::ui::debug_text::DebugText;
use crate::ui::messages::Messages;
use crate::util::convert::ToDVec2;
use egui::{Frame, Id, Modifiers, Painter, PointerButton, Response, ScrollArea, Sense, Widget};
use epaint::mutex::Mutex;
use glam::{DVec2, dvec2};
use std::sync::Arc;

pub struct MapView<'a> {
	pub map_data: Option<&'a MapData>,
	pub styles: &'a mut Option<Styles>,
	pub imagery_fetcher: Arc<Mutex<ImageryFetcher>>,
	pub id: Id,
	pub debug_text: Option<&'a mut DebugText>,
	pub mem: &'a mut MapMemory,
	pub active_imagery_ref: Option<ImageryRef>,
	pub messages: Messages,
}

pub struct MapViewOutput {
	pub response: Response,
	pub frame_response: Response,
	pub view: View,
	pub render_data: Vec<DetailedElement>,
	pub painter: Painter,
	pub world_delta: DVec2,
	pub panning: bool,
}

impl MapViewOutput {
	/// Draw all base elements without highlighting. For simple read-only views.
	pub fn draw_elements(&self, ui: &egui::Ui) {
		let mut shapes = DrawList::new();
		ui.fonts_mut(|fonts| {
			for element in &self.render_data {
				draw_element(element, &mut shapes, &self.view, fonts);
			}
		});
		shapes.draw(&self.painter);
	}
}

impl MapView<'_> {
	pub fn show(mut self, ui: &mut egui::Ui) -> MapViewOutput {
		let mut output: Option<MapViewOutput> = None;

		let frame_response = ScrollArea::neither()
			.show(ui, |ui| {
				Frame::NONE.show(ui, |ui| {
					puffin::profile_function!("Draw MapView");
					let (response, painter) = ui.allocate_painter(ui.available_size(), Sense::click_and_drag());
					ui.request_repaint();
					let clip_rect = ui.clip_rect();
					dbgtxt!(self.debug_text, "Clip Rect: {clip_rect:?}");
					let viewport = response.interact_rect;
					dbgtxt!(self.debug_text, "Viewport: {viewport:?}");

					let zoom = f64::from(
						response.ctx.input(|i| i.smooth_scroll_delta).x
							+ response.ctx.input(|i| i.smooth_scroll_delta).y,
					);
					if zoom != 0.0 && response.contains_pointer() {
						let zoom_diff = zoom / 1000.0 * ZOOM_SPEED;
						let old_zoom = self.mem.zoom_level;
						self.mem.zoom_level += zoom_diff;
						self.mem.zoom_level = self.mem.zoom_level.clamp(1.0, 25.0);

						if let Some(pointer_world_pos) =
							pointer_world_pos(self.mem.camera_pos, &response, viewport, scale(old_zoom))
						{
							let delta = pointer_world_pos - self.mem.camera_pos;
							let new_delta = delta * (scale(self.mem.zoom_level) / scale(old_zoom));
							self.mem.camera_pos = pointer_world_pos - new_delta;
						}
					}

					let view = View::new(&response, self.mem.camera_pos, self.mem.zoom_level);

					let world_delta = {
						let mut delta = ui.input(|i| i.pointer.delta()).to_dvec2();
						if let Some(from_global) = ui.layer_transform_from_global(ui.layer_id()) {
							delta *= f64::from(from_global.scaling);
						}
						delta = dvec2(delta.x, -delta.y);
						delta * view.scale
					};

					let panning = response.dragged_by(PointerButton::Middle)
						|| (response.ctx.input(|i| i.modifiers == Modifiers::ALT)
							&& response.dragged_by(PointerButton::Primary));
					if panning {
						self.mem.camera_pos -= world_delta;
					}

					dbgtxt!(self.debug_text, "Camera: {:?}", self.mem.camera_pos);
					dbgtxt!(self.debug_text, "Zoom: {}", self.mem.zoom_level);
					dbgtxt!(self.debug_text, "Scale: {}", view.scale);
					dbgtxt!(self.debug_text, "UI Scale: {}", view.ui_scale);
					dbgtxt!(self.debug_text, "World View Rect: {:?}", view.world_view_rect);
					dbgtxt!(
						self.debug_text,
						"Pointer Pos: {:?}",
						view.pointer_world_pos.map(|p| p.round().as_ivec2())
					);

					let active_imagery_name = self
						.active_imagery_ref
						.and_then(|reference| {
							self.imagery_fetcher
								.lock()
								.imageries()
								.write()
								.get(reference)
								.map(|imagery| imagery.name().to_string())
						})
						.unwrap_or_else(|| "None".to_owned());

					dbgtxt!(self.debug_text, "Active imagery: {active_imagery_name}");

					if let Some(active_imagery_ref) = self.active_imagery_ref {
						draw_tiles(
							&painter,
							active_imagery_ref,
							&self.imagery_fetcher,
							&view,
							&mut self.debug_text,
						);
					}

					// Element render_data preparation
					let render_data = if let Some(map_data) = self.map_data {
						let mut render_data = Vec::with_capacity(map_data.len());
						let mut style_context = StyleContext {
							zoom_level: view.zoom_level as f32,
							zoom_bucket: view.zoom_level.round_ties_even() as u8,
							updated_styles: 0,
						};

						{
							puffin::profile_scope!("Calculate Render data");
							for element in map_data
								.iter()
								.filter(|e| !matches!(e.element, Element::Relation { .. }))
							{
								DetailedElement::add_elements(
									&mut render_data,
									element,
									map_data,
									self.styles,
									&view,
									&mut style_context,
								);
							}
						}

						if style_context.updated_styles == 0 {
							dbgtxt!(self.debug_text, "Updated styles: 0 ({})", self.mem.last_updated_styles);
						} else {
							dbgtxt!(self.debug_text, "Updated styles: {}", style_context.updated_styles);
							self.mem.last_updated_styles = style_context.updated_styles;
						}

						render_data
					} else {
						Vec::new()
					};

					output = Some(MapViewOutput {
						frame_response: response.clone(), // placeholder, replaced below
						response,
						view,
						render_data,
						painter,
						world_delta,
						panning,
					});
				})
			})
			.inner
			.response;

		let mut output = output.expect("MapView output must be set");
		output.frame_response = frame_response;
		output
	}
}

impl Widget for MapView<'_> {
	fn ui(self, ui: &mut egui::Ui) -> Response {
		let output = self.show(ui);
		output.draw_elements(ui);
		output.frame_response
	}
}
