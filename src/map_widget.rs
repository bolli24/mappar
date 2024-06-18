use egui::{Context, Id, Modifiers, PointerButton, Sense, TextureHandle, Widget};
use glam::{dvec2, DVec2};
use serde::{Deserialize, Serialize};
use skia_safe::{Canvas, Path, Point};
use skia_safe::path::FillType;

use crate::render::RenderElement;
use crate::skia::SkiaWidget;

const ZOOM_SPEED: f32 = 3.0;

pub struct MapWidget<'a> {
	texture_handle: TextureHandle,
	render_data: &'a [RenderElement],
	id: Id,
}

impl<'a> MapWidget<'a> {
	pub fn new(texture_handle: TextureHandle, render_data: &'a [RenderElement], id: Id) -> Self {
		Self {
			texture_handle,
			render_data,
			id,
		}
	}
	#[inline]
	fn draw_element(canvas: &Canvas, element: &RenderElement, camera_pos: DVec2, zoom_level: f32) {
		let Some(paint) = element.style().get_paint(zoom_level) else {
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

				if response.dragged_by(PointerButton::Middle)
					|| (response.ctx.input(|i| i.modifiers == Modifiers::ALT)
						&& response.dragged_by(PointerButton::Primary))
				{
					let delta = response.drag_delta() * 1.5;
					let delta = dvec2(-delta.x as f64, delta.y as f64);
					mem.camera_pos += (response.ctx.zoom_factor() / mem.zoom_level) as f64 * delta;
				};

				let zoom = response.ctx.input(|i| i.smooth_scroll_delta).y;
				if zoom != 0.0 {
					mem.zoom_level *= 1.0 + zoom / 1000.0 * ZOOM_SPEED;
				}

				canvas.translate((size.width as f32 / 2.0, size.height as f32 / 2.0));
				canvas.scale((mem.zoom_level, -mem.zoom_level));
				// canvas.translate((-mem.camera_pos.x as f32, -mem.camera_pos.y as f32));

				for element in render_data {
					Self::draw_element(canvas, element, mem.camera_pos, mem.zoom_level);
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
	pub fn new() -> Self {
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
