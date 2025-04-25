pub mod css;
mod draw_list;
pub mod map_memory;
mod map_state;
mod polygon;
pub mod render;
mod style;
mod view;

use crate::dbgtxt;
use crate::imagery::imagery_fetcher::{ImageryFetcher, Tile};
use crate::imagery::{Imagery, ImageryRef};
use crate::map::map_data::{CIRCUMFERENCE_EARTH, Element, LatLon, MapData, latlon};
use crate::ui::debug_text::DebugText;
use crate::ui::edit_bar::EditMode;
use crate::ui::map_widget::map_state::MapState;
use crate::util::convert::{ToDVec2, ToPos2};
use css::Styles;
use css::context::StyleContext;
use derive_new::new;
use draw_list::DrawList;
use egui::{
	Color32, Frame, Id, Modifiers, Painter, PointerButton, Pos2, Rect, Response, ScrollArea, Sense, Vec2, Widget, pos2,
	vec2,
};
use epaint::{CornerRadius, Fonts, PathShape, PathStroke, Shape, TextShape};
use glam::{DVec2, UVec2, dvec2, uvec2};
use itertools::Itertools;
use map_memory::MapMemory;
use render::{DetailedElement, RenderElement};
use std::cmp::Ordering;
use std::collections::HashSet;
use std::f64::consts::PI;
use std::ops::RangeInclusive;
use std::sync::{Arc, Mutex};
use view::View;

const ZOOM_SPEED: f64 = 5.0;

#[derive(new)]
pub struct MapWidget<'a> {
	map_data: &'a mut MapData,
	styles: &'a mut Option<Styles>,
	imagery_fetcher: Arc<Mutex<ImageryFetcher>>,
	id: Id,
	debug_text: Option<&'a mut DebugText>,
	mem: &'a mut MapMemory,
	interactive: bool,
	active_imagery_ref: Option<ImageryRef>,
	edit_mode: EditMode,
}

const UV: Rect = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));

impl Widget for MapWidget<'_> {
	#[allow(clippy::too_many_lines)]
	fn ui(mut self, ui: &mut egui::Ui) -> egui::Response {
		ScrollArea::neither()
			.show(ui, |ui| {
				Frame::canvas(ui.style()).show(ui, |ui| {
					puffin::profile_function!("Draw Map");
					let (response, painter) = ui.allocate_painter(ui.available_size(), Sense::click_and_drag());
					ui.ctx().request_repaint();
					let clip_rect = ui.clip_rect();
					dbgtxt!(self.debug_text, "Clip Rect: {clip_rect:?}");
					let viewport = response.interact_rect;
					dbgtxt!(self.debug_text, "Viewport: {viewport:?}");

					if self.interactive {
						let zoom = response.ctx.input(|i| i.smooth_scroll_delta).y as f64;
						if zoom != 0.0 && response.contains_pointer() {
							// TODO: fix zoom speed on chrome
							let zoom_diff = zoom / 1000.0 * ZOOM_SPEED;
							self.mem.zoom_level += zoom_diff;
							self.mem.zoom_level = self.mem.zoom_level.clamp(1.0, 25.0);
							if let Some(pointer_world_pos) =
								pointer_world_pos(self.mem.camera_pos, &response, viewport, scale(self.mem.zoom_level))
							{
								let camera_diff = pointer_world_pos - self.mem.camera_pos;
								self.mem.camera_pos += camera_diff * zoom_diff * 0.69; // No idea why the funny number is right here
							}
						}
					}

					let view = View::new(&response, self.mem.camera_pos, self.mem.zoom_level);

					let world_delta = {
						// How much has the pointer moved on the map
						let mut delta = ui.ctx().input(|i| i.pointer.delta());
						if let Some(from_global) = ui.ctx().layer_transform_from_global(ui.layer_id()) {
							delta *= from_global.scaling;
						}
						let delta = delta * view.scale as f32;
						dvec2(f64::from(delta.x), -f64::from(delta.y))
					};

					dbgtxt!(self.debug_text, "World delta: {world_delta}");

					if self.interactive
						&& (response.dragged_by(PointerButton::Middle)
							|| (response.ctx.input(|i| i.modifiers == Modifiers::ALT)
								&& response.dragged_by(PointerButton::Primary)))
					{
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

					let active_imagery_name = self.active_imagery_name();

					dbgtxt!(self.debug_text, "Active imagery: {active_imagery_name:?}");

					if let Some(active_imagery_ref) = self.active_imagery_ref
					/* && self.imagery_fetcher.lock().unwrap().imageries().lock().unwrap().get(reference)*/
					{
						// log::info!("{:?}", self.imagery_fetcher.lock().unwrap().imageries().lock().unwrap());
						draw_tiles(
							&painter,
							active_imagery_ref,
							&self.imagery_fetcher,
							&view,
							&mut self.debug_text,
						);
					}

					let mut render_data = Vec::with_capacity(self.map_data.len());
					let select_threshold = 5.0 * view.scale;
					let style_context = StyleContext {
						zoom_level: view.zoom_level as f32,
					};

					{
						puffin::profile_scope!("Calculate Render data");
						// TODO: Relations
						for element in self
							.map_data
							.iter()
							.filter(|e| !matches!(e.element, Element::Relation { .. }))
						{
							DetailedElement::add_elements(
								&mut render_data,
								element,
								self.map_data,
								self.styles,
								&view,
								&style_context,
							);
						}
					}

					let hover_element = {
						puffin::profile_scope!("Get hover element");
						match (self.interactive, view.pointer_world_pos) {
							(true, Some(_)) => closest_element(&render_data, &view.scale),
							_ => None,
						}
					};

					if self.interactive && self.edit_mode == EditMode::Select {
						if response.contains_pointer()
							&& ui.ctx().input(|i| i.pointer.button_down(PointerButton::Primary))
							&& self.mem.map_state == MapState::Default
						{
							if let Some(hover_element) = hover_element {
								if !ui.ctx().input(|i| i.modifiers.shift)
									&& !self.mem.selection.selected_elements.contains(&hover_element.id)
								{
									self.mem.selection.selected_elements.clear();
								}
								self.mem.selection.selected_elements.insert(hover_element.id);
							}
						}

						if response.clicked_by(PointerButton::Primary) && self.mem.map_state == MapState::Default {
							if let Some(hover_element) = hover_element {
								if !ui.ctx().input(|i| i.modifiers.shift) {
									self.mem.selection.selected_elements.clear();
								}
								self.mem.selection.selected_elements.insert(hover_element.id);
							}
						}

						if response.clicked_by(PointerButton::Primary)
							&& hover_element.is_none()
							&& !ui.ctx().input(|i| i.modifiers.shift)
						{
							self.mem.selection.selected_elements.clear();
						}

						if response.drag_started_by(PointerButton::Primary) {
							self.mem.map_state = MapState::MoveElements(view.pointer_world_pos.unwrap());
						} else if response.drag_stopped_by(PointerButton::Primary) {
							self.mem.map_state = MapState::Default;
						}
						if
						/* response.contains_pointer()
						&& ui.ctx().input(|i| i.pointer.button_down(PointerButton::Primary)) */
						matches!(self.mem.map_state, MapState::MoveElements(_))
							&& world_delta.length_squared() > 1e-5
						// && !self.mem.selection.selected_elements.is_empty()
						{
							self.map_data
								.move_by(&self.mem.selection.selected_elements, &world_delta);
						}
					}

					dbgtxt!(self.debug_text, "MapState: {:?}", self.mem.map_state);
					dbgtxt!(self.debug_text, "Closest element: {:?}", hover_element.map(|e| e.id));
					dbgtxt!(self.debug_text, "Select threshold: {select_threshold:.3}");

					let mut shapes = DrawList::new();
					let fonts = ui.ctx().fonts(|f| f.clone());

					for element in &render_data {
						let hovered = self.interactive && hover_element.is_some_and(|e| e.id == element.id);
						if hovered {
							draw_element(&element.hovered(), &mut shapes, &view, &fonts);
						}
						let selected = self.mem.selection.selected_elements.contains(&element.id);
						if selected {
							draw_element(&element.selected(), &mut shapes, &view, &fonts)
						}
						draw_element(element, &mut shapes, &view, &fonts);
					}

					dbgtxt!(self.debug_text, "Drawn Elements: {}", shapes.count());

					shapes.draw(&painter);

					response
				})
			})
			.inner
			.response
	}
}

impl MapWidget<'_> {
	fn active_imagery_name(&self) -> Option<String> {
		self.active_imagery_ref.and_then(|reference| {
			self.imagery_fetcher
				.lock()
				.unwrap()
				.imageries()
				.lock()
				.unwrap()
				.get(reference)
				.map(|imagery| imagery.name().to_string())
		})
	}
}

#[inline]
fn draw_element(element: &DetailedElement, shapes: &mut DrawList, view: &View, fonts: &Fonts) {
	let color = element.style.color;
	let size = element.style.size;

	match &element.element {
		RenderElement::Node(pos) => {
			let render_pos = view.render_pos(pos);
			let rect = Rect::from_center_size(render_pos, Vec2::splat(size * view.ui_scale));
			if let Some(icon) = element.style.icon {
				shapes.images.push(Shape::image(icon.0, rect, UV, Color32::WHITE))
			} else {
				shapes.nodes.push(Shape::rect_filled(rect, CornerRadius::ZERO, color));
			}

			if !element.style.text.text.is_empty() {
				let text = &element.style.text;
				shapes.nodes.push(Shape::text(
					fonts,
					render_pos + text.offset,
					text.anchor,
					&text.text,
					text.font_id(),
					text.color,
				))
			}
		}
		RenderElement::Path(nodes, is_closed) => {
			let nodes: Vec<Pos2> = nodes.iter().map(|pos| view.render_pos(pos)).collect();
			let stroke = PathStroke::new(size * view.ui_scale, color);

			if !element.style.text.text.is_empty() {
				// FIXME: double text rendering
				let text = &element.style.text;
				let galley = fonts.layout_no_wrap(text.text.to_string(), text.font_id(), text.color);
				let len_sq = galley.size().x.powi(2);

				use std::f32::consts::{FRAC_PI_2, PI};
				for (a, b) in nodes.iter().tuple_windows() {
					if a.distance_sq(*b) > len_sq {
						let vec = b.to_vec2() - a.to_vec2();
						let midpoint = *a + (vec * 0.5);

						if !view.viewport.contains(midpoint) {
							return;
						}

						let mut angle = vec.angle();
						if angle > FRAC_PI_2 {
							angle -= PI
						}

						shapes.paths.push(Shape::Text(TextShape {
							pos: midpoint,
							galley: galley.clone(),
							underline: egui::Stroke::NONE,
							fallback_color: text.color,
							override_text_color: None,
							opacity_factor: 1.0,
							angle,
						}))
					}
				}
			}

			shapes.paths.push(Shape::Path(PathShape {
				points: nodes,
				stroke,
				fill: Color32::TRANSPARENT,
				closed: *is_closed,
			}));
		}
		RenderElement::Area(_, polygons) => {
			for polygon in polygons {
				let nodes: Vec<Pos2> = polygon.iter().map(|pos| view.render_pos(pos)).collect();
				shapes.areas.push(Shape::convex_polygon(
					nodes,
					color,
					PathStroke::NONE,
					//	PathStroke::new(1.0, Color32::LIGHT_BLUE),
				));
			}
		}
		RenderElement::Collection(elements) => {
			for e in elements {
				draw_element(e, shapes, view, fonts);
			}
		}
	};
}

fn draw_tiles(
	painter: &Painter,
	imagery_ref: ImageryRef,
	imagery_fetcher: &Arc<Mutex<ImageryFetcher>>,
	view: &View,
	debug_text: &mut Option<&mut DebugText>,
) {
	puffin::profile_function!();
	#[cfg(not(target_arch = "wasm32"))]
	const ZOOM_ADJUST: f64 = 0.0;
	#[cfg(target_arch = "wasm32")]
	const ZOOM_ADJUST: f64 = -0.5;

	puffin::profile_function!();
	let mut imagery_fetcher = imagery_fetcher.lock().unwrap();
	dbgtxt!(
		debug_text,
		"Local cached tiles: {}",
		imagery_fetcher.persistent_cached_count()
	);
	dbgtxt!(
		debug_text,
		"Memory cached tiles: {} (≈{}MiB)",
		imagery_fetcher.memory_cached_count(),
		imagery_fetcher.vram_used()
	);
	dbgtxt!(debug_text, "Pending tiles: {}", imagery_fetcher.pending_count());

	let corners = get_world_corners(view.viewport, view.camera_pos, view.scale);
	let imagery = imagery_fetcher
		.imageries()
		.lock()
		.unwrap()
		.get(imagery_ref)
		.expect("Imagery reference should be valid.")
		.clone();

	let tile_zoom_level = u32::min(
		(view.zoom_level + (view.ui_scale as f64 - 1.0) + ZOOM_ADJUST).round() as u32,
		imagery.max_zoom(),
	);

	let tile_size =
		((CIRCUMFERENCE_EARTH) / f64::exp2(tile_zoom_level as f64) / 256.0) * imagery.tile_size().unwrap_or(256) as f64;
	let (x_tiles, y_tiles) = tiles_in_rect(corners, tile_zoom_level);

	dbgtxt!(
		debug_text,
		"Tiles in View: {}",
		x_tiles.clone().count() * y_tiles.clone().count()
	);

	let mut drawn_tiles = HashSet::with_capacity(100);

	// TODO: don't fetch new tiles when moving fast

	for (x, y) in x_tiles.cartesian_product(y_tiles) {
		draw_tile(
			Tile::new(imagery_ref, (x, y, tile_zoom_level)),
			MIN_STEP,
			view,
			tile_size,
			&imagery,
			painter,
			&mut drawn_tiles,
			&mut imagery_fetcher,
		);
	}

	dbgtxt!(debug_text, "Drawn tiles: {}", drawn_tiles.len());
}

const MAX_STEP: i32 = 2;
const MIN_STEP: i32 = -MAX_STEP;

fn draw_tile(
	tile: Tile,
	extra_step: i32,
	view: &View,
	tile_size: f64,
	imagery: &Imagery,
	painter: &Painter,
	drawn_tiles: &mut HashSet<Tile>,
	imagery_fetcher: &mut ImageryFetcher,
) {
	puffin::profile_scope!("Draw Tile", format!("{:?}", tile.coords));
	if !drawn_tiles.insert(tile) {
		return;
	}
	let tile_texture = imagery_fetcher.get_tile(tile, extra_step == MIN_STEP);
	if let Some(image) = tile_texture {
		let coords = get_tile_pos(tile);
		let coords = (coords - (view.camera_pos * dvec2(1.0, -1.0))).to_pos2();
		let rect = Rect::from_min_size(coords, vec2(tile_size as f32, tile_size as f32)) / view.scale as f32;
		let rect = rect.translate(view.viewport.size() * 0.5);

		let tint = Color32::WHITE;

		#[cfg(any())]
		let tint = if extra_step == -1 {
			Color32::LIGHT_BLUE
		} else if extra_step == 0 {
			Color32::BLUE
		} else if extra_step == 2 {
			Color32::LIGHT_RED
		} else if extra_step == 3 {
			Color32::RED
		} else {
			Color32::WHITE
		};

		painter.image(image, rect, UV, tint);
		return;
	}

	if (MIN_STEP..0).contains(&extra_step) {
		let extra_step = extra_step + 1;
		if let Some(larger) = tile.get_larger(imagery.min_zoom) {
			draw_tile(
				larger,
				extra_step,
				view,
				tile_size * 2.0,
				imagery,
				painter,
				drawn_tiles,
				imagery_fetcher,
			);
		}
	} else if (0..MAX_STEP).contains(&extra_step) {
		// FIXME:
		let extra_step = extra_step + 1;
		let Some(smaller) = tile.get_smaller(imagery.max_zoom) else {
			return;
		};

		for tile in smaller {
			draw_tile(
				tile,
				extra_step,
				view,
				tile_size / 2.0,
				imagery,
				painter,
				drawn_tiles,
				imagery_fetcher,
			);
		}
	}
}

fn closest_element<'a>(render_data: &'a [DetailedElement], scale: &f64) -> Option<&'a DetailedElement> {
	render_data
		.iter()
		.filter(|e| matches!(e.pointer_distance, Some(dist) if dist < e.pointer_threshold * scale))
		.sorted_by(|a, b| match (a, b) {
			(
				DetailedElement {
					pointer_distance: Some(_),
					element: RenderElement::Node(_),
					..
				},
				DetailedElement {
					pointer_distance: Some(_),
					element: RenderElement::Node(_),
					..
				},
			) => a.pointer_distance.partial_cmp(&b.pointer_distance).expect("NaN"),
			(
				DetailedElement {
					pointer_distance: Some(_),
					element: RenderElement::Node(_),
					..
				},
				_,
			) => Ordering::Less,
			(
				_,
				DetailedElement {
					pointer_distance: Some(_),
					element: RenderElement::Node(_),
					..
				},
			) => Ordering::Greater,
			_ => a.pointer_distance.partial_cmp(&b.pointer_distance).expect("NaN"),
		})
		.next()
}

fn get_tile(coords: DVec2, zoom_level: u32) -> UVec2 {
	const TILE_SIZE: f64 = 256.0;
	let lat_lon = LatLon::from_web_mercator(coords);
	let scale = 2.0f64.powi(zoom_level as i32);

	let mut siny = lat_lon.lat.to_radians().sin();

	siny = siny.clamp(-0.9999, 0.9999);
	let world_coords = dvec2(
		0.5 + lat_lon.lon / 360.0,
		0.5 - f64::ln((1.0 + siny) / (1.0 - siny)) / (4.0 * PI),
	);

	(world_coords * scale).floor().as_uvec2()
}

fn tiles_in_rect(corners: (DVec2, DVec2), zoom_level: u32) -> (RangeInclusive<u32>, RangeInclusive<u32>) {
	let tiles = (get_tile(corners.0, zoom_level), get_tile(corners.1, zoom_level));
	let min = uvec2(tiles.0.x.min(tiles.1.x), tiles.0.y.min(tiles.1.y));
	let max = uvec2(tiles.0.x.max(tiles.1.x), tiles.0.y.max(tiles.1.y));
	(min.x..=max.x, min.y..=max.y)
}

fn get_tile_pos(tile: Tile) -> DVec2 {
	let n = 2.0f64.powi(tile.coords.2 as i32);
	let lon_deg = (tile.coords.0 as f64 / n).mul_add(360.0, -180.0);
	let lat_rad = (PI - 2.0 * PI * tile.coords.1 as f64 / n).sinh().atan();

	latlon(-lat_rad.to_degrees(), lon_deg).to_web_mercator()
}

fn get_world_corners(viewport: Rect, camera_pos: DVec2, scale: f64) -> (DVec2, DVec2) {
	let half = ((viewport.min + viewport.size() * 0.5).to_dvec2()) * scale;
	(camera_pos - half, camera_pos + half)
}

fn view_to_world_pos(pos: Pos2, view: Rect, camera_pos: DVec2, scale: f64) -> DVec2 {
	let pos = pos - view.size() * 0.5;
	(dvec2(pos.x as f64, -pos.y as f64)) * scale + dvec2(camera_pos.x, camera_pos.y)
}

fn pointer_world_pos(camera_pos: DVec2, response: &Response, viewport: Rect, scale: f64) -> Option<DVec2> {
	if !response.contains_pointer() {
		return None;
	}
	response
		.ctx
		.input(|i| i.pointer.latest_pos())
		.map(|pos| view_to_world_pos(pos, viewport, camera_pos, scale))
}

fn get_tile_bound(view: Rect, camera_pos: DVec2, scale: f64) -> (DVec2, DVec2) {
	const EXPAND: f64 = 1.2; // Load some extra tiles
	let half = (dvec2(view.width() as f64 * 0.5, view.height() as f64 * 0.5) + view.min.to_dvec2()) * scale * EXPAND;
	(camera_pos - half, camera_pos + half)
}

/// tile width in meters at specified latitude and zoom level
pub fn tile_width(zoom_level: u32, lat: f64) -> f64 {
	(CIRCUMFERENCE_EARTH * f64::cos(lat.to_radians())) / f64::exp2(zoom_level as f64)
}

/// aka Resolution in meters per pixel
pub fn scale(zoom_level: f64) -> f64 {
	(CIRCUMFERENCE_EARTH * f64::cos(0.0f64.to_radians())) / f64::exp2(zoom_level) / 256.0
}

pub fn world_size(zoom_level: u32) -> f64 {
	2f64.powi(zoom_level as i32)
}
