use std::cell::RefCell;
use std::f64::consts::PI;
use std::ops::RangeInclusive;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use egui::{vec2, Context, Id, Modifiers, PointerButton, Pos2, Rect, Sense, TextureHandle, Widget};
use glam::{dvec2, uvec2, DVec2, UVec2, Vec2Swizzles};
use itertools::{Itertools, Product};
use log::info;
use serde::{Deserialize, Serialize};
use skia_safe::image_filters::tile;
use skia_safe::path::FillType;
use skia_safe::{Canvas, Color, ISize, Paint, Path, Point};

use crate::debug_text::DebugText;
use crate::imagery_fetcher::ImageryFetcher;
use crate::map_data::{LatLon, CIRCUMFERENCE_EARTH, RADIUS_EARTH};
use crate::render::RenderElement;
use crate::skia::SkiaWidget;

const ZOOM_SPEED: f32 = 5.0;

pub struct MapWidget<'a> {
	texture_handle: TextureHandle,
	render_data: MapRenderData<'a>,
	id: Id,
	debug_text: Rc<RefCell<DebugText>>,
}

pub struct MapRenderData<'a> {
	pub elements: &'a [RenderElement],
	pub imagery_fetcher: Arc<Mutex<ImageryFetcher>>,
}

impl<'a> MapWidget<'a> {
	pub fn new(
		texture_handle: TextureHandle,
		render_data: MapRenderData<'a>,
		id: Id,
		debug_text: Rc<RefCell<DebugText>>,
	) -> Self {
		Self {
			texture_handle,
			render_data,
			id,
			debug_text,
		}
	}

	// We translate every element instead of moving the canvas to avoid errors
	// due to Skia only supporting 32-bit floating point number
	#[inline]
	fn draw_element(canvas: &Canvas, element: &RenderElement, camera_pos: DVec2, scale: f32) {
		let Some(paint) = element.style().get_paint(scale) else {
			return;
		};
		match element {
			RenderElement::Node(pos, _) => {
				canvas.draw_point(dvec2_to_point(*pos - camera_pos), &paint);
			}
			RenderElement::Way(nodes, _) => {
				let nodes: Vec<Point> = nodes.iter().map(|p| dvec2_to_point(*p - camera_pos)).collect();
				let closed = nodes.first() == nodes.last();
				let path = Path::polygon(&nodes, closed, FillType::Winding, true);
				canvas.draw_path(&path, &paint);
			}
			RenderElement::Collection(_, _) => {
				todo!()
			}
		}
	}

	fn draw_tiles(
		canvas: &Canvas,
		imagery_fetcher: Arc<Mutex<ImageryFetcher>>,
		camera_pos: DVec2,
		zoom_level: f32,
		debug_text: &mut DebugText,
	) {
		puffin::profile_function!();
		debug_text.add(format!(
			"Local Cached: {} tiles",
			imagery_fetcher
				.lock()
				.unwrap()
				.local_cached_count()
				.map(|c| c as i32)
				.unwrap_or(-1)
		));
		debug_text.add(format!(
			"Memory Cached: {} tiles",
			imagery_fetcher.lock().unwrap().memory_cached_count()
		));
		debug_text.add(format!(
			"Pending: {} tiles",
			imagery_fetcher.lock().unwrap().pending_count(),
		));

		let camera_lat_lon = LatLon::from_web_mercator(camera_pos);
		debug_text.add(format!("Camera pos: {camera_pos:?}"));
		debug_text.add(format!("Zoom level: {zoom_level}"));
		let scale = Self::scale(zoom_level, camera_lat_lon.0);
		let rounded_zoom = zoom_level.round() as u32;
		let corners = Self::get_world_corners(canvas.image_info().dimensions(), camera_pos, scale);
		let corners_lat_lon = (
			LatLon::from_web_mercator(corners.0),
			LatLon::from_web_mercator(corners.1),
		);
		debug_text.add(format!("Corners: {corners_lat_lon:?}"));
		let base_tile_size = imagery_fetcher.lock().unwrap().imagery().lock().unwrap().tile_size();
		let zoom_level1 = zoom_level.round();
		let tile_size = ((CIRCUMFERENCE_EARTH) / f64::exp2(zoom_level1 as f64) / 256.0) * base_tile_size as f64;
		let world_size = Self::world_size(rounded_zoom);

		debug_text.add(format!("Tile size: {tile_size}"));
		debug_text.add(format!("World size: {world_size}"));

		let (x_tiles, y_tiles) = Self::tiles_in_rect(corners, rounded_zoom);
		let center_tile = Self::get_tile(camera_pos, rounded_zoom);
		//let x_tiles = center_tile.x..=center_tile.x;
		//let y_tiles = center_tile.y..=center_tile.y;

		let paint = Paint::default();

		debug_text.add(format!(
			"{} {x_tiles:?} -> {y_tiles:?} = {}",
			rounded_zoom,
			x_tiles.clone().collect::<Vec<_>>().len() * y_tiles.clone().collect::<Vec<_>>().len()
		));

		let center_tile_pos = Self::get_tile_pos(center_tile.x, center_tile.y, rounded_zoom);
		let offset = center_tile_pos - camera_pos;
		debug_text.add(format!("Center tile: {center_tile:?}"));
		debug_text.add(format!("Center tile pos: {center_tile_pos:?}"));
		debug_text.add(format!("Offset: {offset:?}"));

		for (x, y) in x_tiles.cartesian_product(y_tiles) {
			puffin::profile_scope!("Draw Tile", format!("{x} {y}"));
			let tile = imagery_fetcher
				.lock()
				.unwrap()
				.get_tile(x, y, zoom_level.round() as u32);
			if let Some(image) = tile {
				let coords = Self::get_tile_pos(x, y, zoom_level.round() as u32);
				let coords = dvec2_to_point(coords - Self::to_skia_dvec2(camera_pos));
				let dst_rect = skia_safe::Rect::from_xywh(coords.x, coords.y, tile_size as f32, tile_size as f32);
				canvas.draw_image_rect(image, None, dst_rect, &paint);
			};
		}
		let debug_rect = Rect::from_two_pos(Self::dvec_to_pos(corners.0), Self::dvec_to_pos(corners.1))
			.translate(-vec2(camera_pos.x as f32, camera_pos.y as f32));

		let mut path = Path::new();
		path.move_to(Self::pos_to_point(debug_rect.left_top()));
		path.line_to(Self::pos_to_point(debug_rect.right_top()));
		path.line_to(Self::pos_to_point(debug_rect.right_bottom()));
		path.line_to(Self::pos_to_point(debug_rect.left_bottom()));
		path.close();

		let mut paint = Paint::default();
		paint.set_color(Color::RED);
		paint.set_anti_alias(true);
		paint.set_stroke(true);
		paint.set_stroke_width(4.0 * scale as f32);
		canvas.draw_path(&path, &paint);
	}

	const CENTER: DVec2 = dvec2(2.0037508342787E7, 2.0037508342787E7);

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
		let lat_lon = LatLon::from_web_mercator(coords);
		let scale = 2.0f64.powi(zoom_level as i32);

		let mut siny = lat_lon.0.to_radians().sin();

		siny = siny.clamp(-0.9999, 0.9999);
		const TILE_SIZE: f64 = 256.0;
		let world_coords = dvec2(
			0.5 + lat_lon.1 / 360.0,
			0.5 - f64::ln((1.0 + siny) / (1.0 - siny)) / (4.0 * PI),
		);

		(world_coords * scale).floor().as_uvec2()
	}

	fn get_tile_pos(tile_x: u32, tile_y: u32, zoom_level: u32) -> DVec2 {
		let n = 2.0f64.powi(zoom_level as i32);
		let lon_deg = tile_x as f64 / n * 360.0 - 180.0;
		let lat_rad = (PI - 2.0 * PI * tile_y as f64 / n).sinh().atan();

		LatLon(-lat_rad.to_degrees(), lon_deg).to_web_mercator()
	}

	fn get_world_corners(view: ISize, camera_pos: DVec2, scale: f64) -> (DVec2, DVec2) {
		let half = dvec2(view.width as f64 / 2.0, view.height as f64 / 2.0) * scale;
		(camera_pos - half, camera_pos + half)
	}

	// tile width in meters at specified latitude and zoom level
	pub fn tile_width(zoom_level: u32, lat: f64) -> f64 {
		(CIRCUMFERENCE_EARTH * f64::cos(lat.to_radians())) / f64::exp2(zoom_level as f64)
	}

	// aka Resolution in meters per pixel
	pub fn scale(zoom_level: f32, lat: f64) -> f64 {
		(CIRCUMFERENCE_EARTH * f64::cos(50.0f64.to_radians())) / f64::exp2(zoom_level as f64) / 256.0
	}

	pub fn world_size(zoom_level: u32) -> f64 {
		2f64.powi(zoom_level as i32)
	}

	pub fn scale_cam(zoom_level: f32, camera_pos: DVec2) -> f64 {
		Self::scale(zoom_level, LatLon::from_web_mercator(camera_pos).0)
	}

	fn pos_to_point(pos: Pos2) -> Point {
		Point::new(pos.x, pos.y)
	}

	fn pos_floor_uvec2(pos: Pos2) -> UVec2 {
		uvec2(pos.x.floor() as u32, pos.y.floor() as u32)
	}

	fn dvec_to_pos(dvec: DVec2) -> Pos2 {
		Pos2::new(dvec.x as f32, dvec.y as f32)
	}

	pub fn to_skia_dvec2(v: DVec2) -> DVec2 {
		dvec2(v.x, -v.y)
	}
}

pub fn dvec2_to_point(v: DVec2) -> Point {
	Point::new(v.x as f32, v.y as f32)
}
impl Widget for MapWidget<'_> {
	fn ui(self, ui: &mut egui::Ui) -> egui::Response {
		return ui.add(SkiaWidget::new(
			self.texture_handle,
			self.render_data,
			Sense::click_and_drag(),
			Box::new(move |canvas, response, render_data| {
				let size = canvas.image_info().dimensions();
				let mut mem = MapMemory::load_or_new(&response.ctx, self.id);
				let camera_lat_lon = LatLon::from_web_mercator(mem.camera_pos);

				let zoom = response.ctx.input(|i| i.smooth_scroll_delta).y;
				if zoom != 0.0 {
					mem.zoom_level += zoom / 1000.0 * ZOOM_SPEED;
					mem.zoom_level = mem.zoom_level.clamp(1.0, 25.0);
				}

				let scale = 1.0 / Self::scale(mem.zoom_level, camera_lat_lon.0);

				if response.dragged_by(PointerButton::Middle)
					|| (response.ctx.input(|i| i.modifiers == Modifiers::ALT)
						&& response.dragged_by(PointerButton::Primary))
				{
					let delta = response.drag_motion() / scale as f32;
					let delta = dvec2(f64::from(-delta.x), f64::from(delta.y));
					mem.camera_pos += delta;
				}

				canvas.translate((size.width as f32 / 2.0, size.height as f32 / 2.0));
				canvas.scale((scale as f32, scale as f32));

				Self::draw_tiles(
					canvas,
					render_data.imagery_fetcher.clone(),
					mem.camera_pos,
					mem.zoom_level,
					&mut self.debug_text.borrow_mut(),
				);

				let offset = Self::to_skia_dvec2(mem.camera_pos);
				for element in render_data.elements {
					Self::draw_element(canvas, element, offset, scale as f32);
				}
				mem.store(&response.ctx, self.id);
			}),
		));
	}
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
pub struct MapMemory {
	pub zoom_level: f32,
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
