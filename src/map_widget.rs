use std::sync::{Arc, Mutex};

use egui::{Context, Id, Modifiers, PointerButton, Pos2, Rect, Sense, TextureHandle, vec2, Widget};
use glam::{dvec2, DVec2, uvec2, UVec2};
use log::info;
use serde::{Deserialize, Serialize};
use skia_safe::{Canvas, Color, Image, ISize, Paint, Path, Point};
use skia_safe::path::FillType;

use crate::imagery_fetcher::ImageryFetcher;
use crate::map_data::{CIRCUMFERENCE_EARTH, LatLon};
use crate::render::RenderElement;
use crate::skia::SkiaWidget;

const ZOOM_SPEED: f32 = 5.0;

pub struct MapWidget<'a> {
	texture_handle: TextureHandle,
	render_data: MapRenderData<'a>,
	id: Id,
}

pub struct MapRenderData<'a> {
	pub elements: &'a [RenderElement],
	pub imagery_fetcher: Arc<Mutex<ImageryFetcher>>,
}

impl<'a> MapWidget<'a> {
	pub fn new(texture_handle: TextureHandle, render_data: MapRenderData<'a>, id: Id) -> Self {
		Self {
			texture_handle,
			render_data,
			id,
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

	fn draw_tiles(canvas: &Canvas, imagery_fetcher: Arc<Mutex<ImageryFetcher>>, camera_pos: DVec2, zoom_level: f32) {
		let camera_lat_lon = LatLon::from_web_mercator(camera_pos);
		let scale = Self::scale(zoom_level, camera_lat_lon.1);
		let world_rect = Self::get_world_rect(canvas.image_info().dimensions(), camera_pos, scale as f32);
		let tile_size = Self::scale(zoom_level.round_ties_even(), camera_lat_lon.1)
			* imagery_fetcher.lock().unwrap().imagery().lock().unwrap().tile_size() as f64;
		let first = Self::pos_floor_uvec2(world_rect.left_top() / tile_size as f32);
		let last = Self::pos_floor_uvec2(world_rect.right_bottom() / tile_size as f32);
		let paint = Paint::default();

		info!(
			"{} {first} -> {last} = {}",
			zoom_level as u32,
			(last.x - first.x) * (last.y - first.y)
		);

		for y in first.y..=last.y {
			for x in first.x..=last.x {
				let tile = imagery_fetcher.lock().unwrap().get_tile(x, y, zoom_level as u32);
				if let Some(image) = tile {
					let coords = Point::new(
						(x as f64 * tile_size - camera_pos.x) as f32,
						(y as f64 * tile_size - camera_pos.y) as f32,
					);
					let dst_rect = skia_safe::Rect::from_xywh(coords.x, coords.y, tile_size as f32, tile_size as f32);
					canvas.draw_image_rect(image, None, dst_rect, &paint);
				};
			}
		}

		let debug_rect = world_rect.translate(-vec2(camera_pos.x as f32, camera_pos.y as f32));

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
		paint.set_stroke_width(2.0 * scale as f32);
		canvas.draw_path(&path, &paint);
	}

	fn get_world_rect(view: ISize, camera_pos: DVec2, scale: f32) -> Rect {
		let center = Pos2::new(camera_pos.x as f32, camera_pos.y as f32);
		let size = vec2(view.width as f32 * scale, view.height as f32 * scale);
		Rect::from_center_size(center, size)
	}

	pub fn scale(zoom_level: f32, lat: f64) -> f64 {
		(CIRCUMFERENCE_EARTH * f64::cos(lat.to_radians())) / f64::exp2(zoom_level as f64 + 7.0)
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

				let scale = 1.0 / Self::scale(mem.zoom_level, camera_lat_lon.1);

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
				);

				for element in render_data.elements {
					Self::draw_element(canvas, element, mem.camera_pos, scale as f32);
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
