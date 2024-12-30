pub mod css;
mod draw_list;
pub mod map_memory;
mod polygon;
pub mod render;
mod style;

use crate::imagery::imagery_fetcher::ImageryFetcher;
use crate::map::map_data::{latlon, Element, LatLon, MapData, CIRCUMFERENCE_EARTH};
use crate::ui::debug_text::DebugText;
use crate::util::convert::{ToDVec2, ToPos2};
use css::Styles;
use draw_list::DrawList;
use eframe::emath::RectTransform;
use egui::{
	pos2, vec2, Color32, Frame, Id, Modifiers, Painter, PointerButton, Pos2, Rect, Response, ScrollArea, Sense, Vec2,
	Widget,
};
use epaint::{PathShape, PathStroke, Rounding, Shape};
use glam::{dvec2, uvec2, DVec2, UVec2};
use itertools::Itertools;
use map_memory::MapMemory;
use render::{DetailedElement, RenderElement};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::f64::consts::PI;
use std::ops::RangeInclusive;
use std::sync::{Arc, Mutex};

const ZOOM_SPEED: f64 = 5.0;

pub struct MapWidget<'a> {
	map_data: &'a mut MapData,
	styles: &'a mut Option<Styles>,
	imagery_fetcher: Arc<Mutex<ImageryFetcher>>,
	id: Id,
	debug_text: &'a mut DebugText,
	mem: &'a mut MapMemory,
}

#[derive(Debug, Copy, Clone)]
pub struct View {
	pub transform: RectTransform,
	pub viewport: Rect,
	pub camera_pos: DVec2,
	pub world_view_rect: Rect,
	pub zoom_level: f64,
	pub scale: f64,
	pub ui_scale: f32,
	pub pointer_world_pos: Option<DVec2>,
}

const UV: Rect = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));

impl<'a> MapWidget<'a> {
	pub fn new(
		map_data: &'a mut MapData,
		styles: &'a mut Option<Styles>,
		imagery_fetcher: Arc<Mutex<ImageryFetcher>>,
		id: Id,
		debug_text: &'a mut DebugText,
		mem: &'a mut MapMemory,
	) -> Self {
		Self {
			map_data,
			styles,
			imagery_fetcher,
			id,
			debug_text,
			mem,
		}
	}
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
pub enum MapState {
	Default,
	BoxSelect(DVec2),
	Move,
}

impl Widget for MapWidget<'_> {
	#[allow(clippy::too_many_lines)]
	fn ui(self, ui: &mut egui::Ui) -> egui::Response {
		ScrollArea::neither()
			.show(ui, |ui| {
				Frame::canvas(ui.style()).show(ui, |ui| {
					puffin::profile_function!("Draw Map");
					let ui_scale = ui.ctx().pixels_per_point();
					let (response, painter) = ui.allocate_painter(ui.available_size(), Sense::click_and_drag());
					ui.ctx().request_repaint();
					let clip_rect = ui.clip_rect();
					self.debug_text.add(format!("Clip Rect: {clip_rect:?}"));
					let viewport = response.interact_rect;
					self.debug_text.add(format!("Viewport: {viewport:?}"));

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

					let scale = scale(self.mem.zoom_level);

					if response.dragged_by(PointerButton::Middle)
						|| (response.ctx.input(|i| i.modifiers == Modifiers::ALT)
							&& response.dragged_by(PointerButton::Primary))
					{
						let delta = response.drag_delta();
						let delta = delta * scale as f32;
						let delta = dvec2(f64::from(delta.x), -f64::from(delta.y));
						self.mem.camera_pos -= delta;
					}

					self.debug_text.add(format!("Camera: {:?}", self.mem.camera_pos));
					self.debug_text.add(format!("Zoom: {}", self.mem.zoom_level));
					self.debug_text.add(format!("Scale: {scale}"));
					self.debug_text.add(format!("UI Scale: {ui_scale}"));

					let camera_rect = viewport.translate(-viewport.size() / 2.0) * scale as f32;
					let transform = RectTransform::from_to(camera_rect, viewport);
					let world_corners = get_world_corners(viewport, self.mem.camera_pos, scale);
					let world_view_rect = Rect::from_two_pos(world_corners.0.to_pos2(), world_corners.1.to_pos2());
					let pointer_world_pos = pointer_world_pos(self.mem.camera_pos, &response, viewport, scale); // We get this here again to reduce latency

					self.debug_text.add(format!("World View Rect: {:?}", world_view_rect));
					self.debug_text.add(format!(
						"Pointer Pos: {:?}",
						pointer_world_pos.map(|p| p.round().as_ivec2())
					));

					let view = View {
						transform,
						viewport,
						camera_pos: self.mem.camera_pos,
						world_view_rect,
						scale,
						zoom_level: self.mem.zoom_level,
						ui_scale,
						pointer_world_pos,
					};

					draw_tiles(&painter, &self.imagery_fetcher, &view, self.debug_text);

					let mut render_data = Vec::with_capacity(self.map_data.len());
					let select_threshold = 5.0 * scale;

					{
						puffin::profile_scope!("Calculate Render data");
						// TODO: Relations
						for element in self
							.map_data
							.iter()
							.filter(|e| !matches!(e.element, Element::Relation { .. }))
						{
							DetailedElement::add_elements(&mut render_data, element, self.map_data, self.styles, &view);
						}
					}

					let hover_element = {
						puffin::profile_scope!("Get hover element");
						match pointer_world_pos {
							Some(_) => closest_element(&render_data, &scale),
							None => None,
						}
					};

					if response.clicked() {
						if let Some(hover_element) = hover_element {
							self.mem.selection.clear(self.map_data);
							self.mem.selection.selected_elements.insert(hover_element.id);
						} else {
							self.mem.selection.clear(self.map_data);
						}
					}

					self.debug_text
						.add(format!("Closest element: {:?}", hover_element.map(|e| e.id)));
					self.debug_text.add(format!("Select threshold: {select_threshold:.3}"));

					let mut shapes = DrawList::new();

					for element in &render_data {
						let hovered = hover_element.is_some_and(|e| e.id == element.id);
						if hovered {
							draw_element(&element.hovered(), &mut shapes, &view);
						}
						let selected = self.mem.selection.selected_elements.contains(&element.id);
						if selected {
							// TODO: Selection Style
							draw_element(&element.hovered(), &mut shapes, &view)
						}
						draw_element(element, &mut shapes, &view);
					}

					self.debug_text.add(format!("Drawn Elements: {}", shapes.count()));

					shapes.draw(&painter);

					response
				})
			})
			.inner
			.response
	}
}

impl<'a> MapWidget<'a> {}

#[inline]
fn draw_element(element: &DetailedElement, shapes: &mut DrawList, view: &View) {
	let color = element.style.color;
	let size = element.style.size;

	match &element.element {
		RenderElement::Node(pos) => {
			let rect = Rect::from_center_size(
				render_pos(pos, &view.camera_pos, &view.transform),
				Vec2::splat(size * view.ui_scale),
			);
			if let Some(icon) = element.style.icon {
				shapes.images.push(Shape::image(icon.0, rect, UV, Color32::WHITE))
			} else {
				shapes.nodes.push(Shape::rect_filled(rect, Rounding::ZERO, color));
			}
		}
		RenderElement::Path(nodes, is_closed) => {
			let nodes: Vec<Pos2> = nodes
				.iter()
				.map(|pos| render_pos(pos, &view.camera_pos, &view.transform))
				.collect();
			let stroke = PathStroke::new(size * view.ui_scale, color);
			shapes.paths.push(Shape::Path(PathShape {
				points: nodes,
				stroke,
				fill: Color32::TRANSPARENT,
				closed: *is_closed,
			}));
		}
		RenderElement::Area(_, polygons) => {
			for polygon in polygons {
				let nodes: Vec<Pos2> = polygon
					.iter()
					.map(|pos| render_pos(pos, &view.camera_pos, &view.transform))
					.collect();
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
				draw_element(e, shapes, view);
			}
		}
	};
}

fn draw_tiles(
	painter: &Painter,
	imagery_fetcher: &Arc<Mutex<ImageryFetcher>>,
	view: &View,
	debug_text: &mut DebugText,
) {
	puffin::profile_function!();
	#[cfg(not(target_arch = "wasm32"))]
	const ZOOM_ADJUST: f64 = 0.0;
	#[cfg(target_arch = "wasm32")]
	const ZOOM_ADJUST: f64 = -0.5;

	puffin::profile_function!();
	let mut imagery_fetcher = imagery_fetcher.lock().unwrap();
	debug_text.add(format!(
		"Local cached tiles: {}",
		imagery_fetcher.persistent_cached_count()
	));
	debug_text.add(format!(
		"Memory cached tiles: {} (≈{}MiB)",
		imagery_fetcher.memory_cached_count(),
		imagery_fetcher.vram_used()
	));
	debug_text.add(format!("Pending tiles: {}", imagery_fetcher.pending_count(),));

	let corners = get_world_corners(view.viewport, view.camera_pos, view.scale);

	let base_tile_size = imagery_fetcher.imagery().lock().unwrap().tile_size().unwrap();
	let max_zoom = imagery_fetcher.imagery().lock().unwrap().max_zoom();

	let tile_zoom_level = u32::min(
		(view.zoom_level + (view.ui_scale as f64 - 1.0) + ZOOM_ADJUST).round() as u32,
		max_zoom,
	);

	let tile_size = ((CIRCUMFERENCE_EARTH) / f64::exp2(tile_zoom_level as f64) / 256.0) * base_tile_size as f64;
	let (x_tiles, y_tiles) = tiles_in_rect(corners, tile_zoom_level);

	debug_text.add(format!(
		"Tiles in View: {}",
		x_tiles.clone().count() * y_tiles.clone().count()
	));

	for (x, y) in x_tiles.cartesian_product(y_tiles) {
		puffin::profile_scope!("Draw Tile", format!("{x} {y}"));
		// TODO: don't fetch new tiles when moving fast
		let tile = imagery_fetcher.get_tile(x, y, tile_zoom_level, true);
		if let Some(image) = tile {
			let coords = get_tile_pos(x, y, tile_zoom_level);
			let coords = (coords - (view.camera_pos * dvec2(1.0, -1.0))).to_pos2();
			let rect = Rect::from_min_size(coords, vec2(tile_size as f32, tile_size as f32)) / view.scale as f32;
			let rect = rect.translate(view.viewport.size() * 0.5);
			painter.image(image, rect, UV, Color32::WHITE);
		};
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

// We translate every position directly instead of using the transform
// due to epaint only supporting 32-bit floating point number
#[inline]
fn render_pos(pos: &DVec2, camera_offset: &DVec2, transform: &RectTransform) -> Pos2 {
	let pos = (*pos - *camera_offset).to_pos2();
	transform * Pos2::new(pos.x, -pos.y)
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

fn get_tile_pos(tile_x: u32, tile_y: u32, zoom_level: u32) -> DVec2 {
	let n = 2.0f64.powi(zoom_level as i32);
	let lon_deg = (tile_x as f64 / n).mul_add(360.0, -180.0);
	let lat_rad = (PI - 2.0 * PI * tile_y as f64 / n).sinh().atan();

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
	let pointer_world_pos = response
		.contains_pointer()
		.then(|| {
			response
				.ctx
				.input(|i| i.pointer.latest_pos())
				.map(|pos| view_to_world_pos(pos, viewport, camera_pos, scale))
		})
		.flatten();
	pointer_world_pos
}

fn get_tile_bound(view: Rect, camera_pos: DVec2, scale: f64) -> (DVec2, DVec2) {
	const EXPAND: f64 = 1.2; // Load some extra tiles
	let half = (dvec2(view.width() as f64 * 0.5, view.height() as f64 * 0.5) + view.min.to_dvec2()) * scale * EXPAND;
	(camera_pos - half, camera_pos + half)
}

// tile width in meters at specified latitude and zoom level
pub fn tile_width(zoom_level: u32, lat: f64) -> f64 {
	(CIRCUMFERENCE_EARTH * f64::cos(lat.to_radians())) / f64::exp2(zoom_level as f64)
}

// aka Resolution in meters per pixel
pub fn scale(zoom_level: f64) -> f64 {
	(CIRCUMFERENCE_EARTH * f64::cos(0.0f64.to_radians())) / f64::exp2(zoom_level) / 256.0
}

pub fn world_size(zoom_level: u32) -> f64 {
	2f64.powi(zoom_level as i32)
}
