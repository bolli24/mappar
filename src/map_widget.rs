use std::cell::RefCell;
use std::f64::consts::PI;
use std::ops::RangeInclusive;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use eframe::emath::RectTransform;
use egui::{
	pos2, vec2, Color32, Context, Frame, Id, Modifiers, Painter, PointerButton, Pos2, Rect, ScrollArea, Sense, Vec2, Widget,
};
use epaint::{PathShape, PathStroke, RectShape, Rounding, Shape};
use glam::{dvec2, uvec2, DVec2, UVec2};
use itertools::Itertools;
use serde::{Deserialize, Serialize};

use crate::debug_text::DebugText;
use crate::imagery_fetcher::ImageryFetcher;
use crate::map_data::{LatLon, CIRCUMFERENCE_EARTH};
use crate::render::{RenderElement, Style};

const ZOOM_SPEED: f64 = 5.0;

pub struct MapWidget<'a> {
	render_data: MapRenderData<'a>,
	id: Id,
	debug_text: Rc<RefCell<DebugText>>,
}

pub struct MapRenderData<'a> {
	pub elements: &'a [RenderElement],
	pub imagery_fetcher: Arc<Mutex<ImageryFetcher>>,
}

impl<'a> MapWidget<'a> {
	pub fn new(render_data: MapRenderData<'a>, id: Id, debug_text: Rc<RefCell<DebugText>>) -> Self {
		Self {
			render_data,
			id,
			debug_text,
		}
	}

	// We translate every element instead of moving the canvas to avoid errors
	// due to epaint only supporting 32-bit floating point number
	#[inline]
	fn draw_element(element: &RenderElement, transform: RectTransform, camera_pos: DVec2, scale: f32) -> Option<Shape> {
		if *element.style() == Style::Invisible {
			return None;
		}

		let shape = match element {
			RenderElement::Node(pos, _) => Shape::rect_filled(
				Rect::from_center_size(transform * dvec2_to_pos(*pos - camera_pos), Vec2::splat(5.0 * scale)),
				Rounding::ZERO,
				Color32::BLACK,
			),
			RenderElement::Way(nodes, _) => {
				let nodes: Vec<Pos2> = nodes
					.iter()
					.map(|p| transform * dvec2_to_pos(*p - camera_pos))
					.collect();
				let stroke = PathStroke::new(1.0 * scale, Color32::BLACK);
				if nodes.first() == nodes.last() {
					Shape::Path(PathShape::closed_line(nodes, stroke))
				} else {
					Shape::Path(PathShape::line(nodes, stroke))
				}
			}
			RenderElement::Collection(_, _) => {
				todo!()
			}
		};
		Some(shape)
	}

	fn draw_tiles(
		painter: &Painter,
		viewport: Rect,
		imagery_fetcher: &Arc<Mutex<ImageryFetcher>>,
		camera_pos: DVec2,
		zoom_level: f64,
		debug_text: &mut DebugText,
	) {
		puffin::profile_function!();
		debug_text.add(format!(
			"Local cached tiles: {}",
			imagery_fetcher
				.lock()
				.unwrap()
				.local_cached_count()
				.map_or(-1, |c| c as i32)
		));
		debug_text.add(format!(
			"Memory cached tiles: {}",
			imagery_fetcher.lock().unwrap().memory_cached_count()
		));
		debug_text.add(format!(
			"Pending tiles: {}",
			imagery_fetcher.lock().unwrap().pending_count(),
		));

		let scale = Self::scale(zoom_level);
		let corners = Self::get_world_corners(viewport, camera_pos, scale);

		let base_tile_size = imagery_fetcher.lock().unwrap().imagery().lock().unwrap().tile_size();
		let max_zoom = imagery_fetcher.lock().unwrap().imagery().lock().unwrap().max_zoom();
		let tile_zoom_level = u32::min(zoom_level.round() as u32 + 1, max_zoom);

		let tile_size = ((CIRCUMFERENCE_EARTH) / f64::exp2(tile_zoom_level as f64) / 256.0) * base_tile_size as f64;
		let (x_tiles, y_tiles) = Self::tiles_in_rect(corners, tile_zoom_level);

		debug_text.add(format!(
			"Tiles in View: {}",
			x_tiles.clone().count() * y_tiles.clone().count()
		));

		for (x, y) in x_tiles.cartesian_product(y_tiles) {
			puffin::profile_scope!("Draw Tile", format!("{x} {y}"));
			let tile = imagery_fetcher.lock().unwrap().get_tile(x, y, tile_zoom_level);
			if let Some(image) = tile {
				const UV: Rect = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
				let coords = Self::get_tile_pos(x, y, tile_zoom_level);
				let coords = dvec2_to_pos(coords - (camera_pos * dvec2(1.0, -1.0)));
				let rect = Rect::from_min_size(coords, vec2(tile_size as f32, tile_size as f32)) / scale as f32;
				let rect = rect.translate(viewport.size() * 0.5);
				painter.image(image, rect, UV, Color32::WHITE);
			};
		}
		// TODO: Align properly with viewport.
		// let debug_rect = Rect::from_two_pos(
		// 	transform * dvec2_to_pos(corners.0 - camera_pos),
		// 	transform * dvec2_to_pos(corners.1 - camera_pos),
		// );
		// painter.debug_rect(debug_rect, Color32::RED, "Debug Rect");
	}

	fn tiles_in_rect(corners: (DVec2, DVec2), zoom_level: u32) -> (RangeInclusive<u32>, RangeInclusive<u32>) {
		let tiles = (
			Self::get_tile(corners.0, zoom_level),
			Self::get_tile(corners.1, zoom_level),
		);
		let min = uvec2(tiles.0.x.min(tiles.1.x), tiles.0.y.min(tiles.1.y));
		let max = uvec2(tiles.0.x.max(tiles.1.x), tiles.0.y.max(tiles.1.y));
		(min.x..=max.x, min.y..=max.y)
	}

	fn get_tile(coords: DVec2, zoom_level: u32) -> UVec2 {
		const TILE_SIZE: f64 = 256.0;
		let lat_lon = LatLon::from_web_mercator(coords);
		let scale = 2.0f64.powi(zoom_level as i32);

		let mut siny = lat_lon.0.to_radians().sin();

		siny = siny.clamp(-0.9999, 0.9999);
		let world_coords = dvec2(
			0.5 + lat_lon.1 / 360.0,
			0.5 - f64::ln((1.0 + siny) / (1.0 - siny)) / (4.0 * PI),
		);

		(world_coords * scale).floor().as_uvec2()
	}

	fn get_tile_pos(tile_x: u32, tile_y: u32, zoom_level: u32) -> DVec2 {
		let n = 2.0f64.powi(zoom_level as i32);
		let lon_deg = (tile_x as f64 / n).mul_add(360.0, -180.0);
		let lat_rad = (PI - 2.0 * PI * tile_y as f64 / n).sinh().atan();

		LatLon(-lat_rad.to_degrees(), lon_deg).to_web_mercator()
	}

	fn get_world_corners(view: Rect, camera_pos: DVec2, scale: f64) -> (DVec2, DVec2) {
		let half = dvec2(view.width() as f64 * 0.5, view.height() as f64 * 0.5) * scale;
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
}

impl Widget for MapWidget<'_> {
	fn ui(self, ui: &mut egui::Ui) -> egui::Response {
		ScrollArea::vertical()
			.show(ui, |ui| {
				Frame::canvas(ui.style()).show(ui, |ui| {
					let ui_scale = ui.ctx().pixels_per_point();
					let mut debug_text = self.debug_text.borrow_mut();
					let (viewport, response) = ui.allocate_at_least(ui.available_size(), Sense::click_and_drag());
					let painter = ui.painter_at(viewport);
					ui.ctx().request_repaint();

					let mut shapes: Vec<Shape> = Vec::new();

					let mut mem = MapMemory::load_or_new(&response.ctx, self.id);

					let zoom = response.ctx.input(|i| i.smooth_scroll_delta).y as f64;
					if zoom != 0.0 {
						mem.zoom_level += zoom / 1000.0 * ZOOM_SPEED;
						mem.zoom_level = mem.zoom_level.clamp(1.0, 25.0);
					}

					let scale = 1.0 / Self::scale(mem.zoom_level);

					if response.dragged_by(PointerButton::Middle)
						|| (response.ctx.input(|i| i.modifiers == Modifiers::ALT)
							&& response.dragged_by(PointerButton::Primary))
					{
						let delta = response.drag_motion() / ui_scale;
						let delta = delta / scale as f32;
						let delta = dvec2(f64::from(delta.x), -f64::from(delta.y));
						mem.camera_pos -= delta;
					}

					debug_text.add(format!("Camera: {:?}", mem.camera_pos));
					debug_text.add(format!("Zoom: {}", mem.zoom_level));
					debug_text.add(format!("Scale: {scale}"));

					let camera_rect = viewport.translate(-viewport.size() / 2.0) / scale as f32;
					let transform = RectTransform::from_to(camera_rect, viewport);

					ui.painter().add(Shape::Rect(RectShape::filled(
						viewport,
						Rounding::ZERO,
						Color32::LIGHT_GRAY,
					)));

					drop(debug_text);

					Self::draw_tiles(
						&painter,
						viewport,
						&self.render_data.imagery_fetcher,
						mem.camera_pos,
						mem.zoom_level,
						&mut self.debug_text.borrow_mut(),
					);

					let camera_skia = dvec2(mem.camera_pos.x, -mem.camera_pos.y);
					for element in self.render_data.elements {
						if let Some(shape) = Self::draw_element(element, transform, camera_skia, scale as f32) {
							shapes.push(shape);
						}
					}

					painter.extend(shapes);
					mem.store(&response.ctx, self.id);

					response
				})
			})
			.inner
			.response
	}
}

pub const fn dvec2_to_pos(dvec: DVec2) -> Pos2 {
	Pos2::new(dvec.x as f32, dvec.y as f32)
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
pub struct MapMemory {
	pub zoom_level: f64,
	pub camera_pos: DVec2,
}

impl MapMemory {
	pub const fn new() -> Self {
		Self {
			zoom_level: 1.0,
			camera_pos: dvec2(0.0, 0.0),
		}
	}

	pub fn load_or_new(ctx: &Context, id: Id) -> Self {
		ctx.data_mut(|d| *d.get_persisted_mut_or(id, Self::new()))
	}
	pub fn load(ctx: &Context, id: Id) -> Option<Self> {
		ctx.data_mut(|d| d.get_persisted(id))
	}

	pub fn store(self, ctx: &Context, id: Id) {
		ctx.data_mut(|d| d.insert_persisted(id, self));
	}
}
