use eframe::emath::Vec2;
use egui::{vec2, Context, Id, PointerButton, Sense, TextureHandle, Widget};
use serde::{Deserialize, Serialize};
use skia_safe::Canvas;

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
	fn draw_element(canvas: &Canvas, element: &RenderElement) {
		let Some(paint) = element.style().get_paint() else {
			return;
		};
		match element {
			RenderElement::Node(pos, _) => {
				canvas.draw_point(*pos, &paint);
			}
			RenderElement::Way(path, _) => {
				canvas.draw_path(path, &paint);
			}
			RenderElement::Collection(_, _) => {
				todo!()
			}
		}
	}
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

				if response.dragged_by(PointerButton::Middle) {
					let delta = response.drag_delta();
					let delta = vec2(-delta.x, delta.y);
					mem.camera_pos += response.ctx.zoom_factor() / mem.zoom_level * delta;
				}

				let zoom = response.ctx.input(|i| i.smooth_scroll_delta).y;
				if zoom != 0.0 {
					mem.zoom_level *= 1.0 + zoom / 1000.0 * ZOOM_SPEED;
				}

				canvas.translate((size.width as f32 / 2.0, size.height as f32 / 2.0));
				canvas.scale((mem.zoom_level, -mem.zoom_level));
				canvas.translate((-mem.camera_pos.x, -mem.camera_pos.y));

				for element in render_data {
					Self::draw_element(canvas, element);
				}
				mem.store(&response.ctx, self.id);
			}),
		));
	}
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
pub struct MapMemory {
	pub zoom_level: f32,
	pub camera_pos: Vec2,
}

impl MapMemory {
	pub fn new() -> Self {
		Self {
			zoom_level: 1.0,
			camera_pos: vec2(0.0, 0.0),
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
