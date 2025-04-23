use eframe::emath::RectTransform;
use egui::{Pos2, Rect, Response};
use glam::DVec2;

use crate::util::convert::ToPos2;

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

impl View {
	pub fn new(response: &Response, camera_pos: DVec2, zoom_level: f64) -> Self {
		let viewport = response.interact_rect;
		let scale = super::scale(zoom_level);
		let camera_rect = viewport.translate(-viewport.size() / 2.0) * scale as f32;
		let transform = RectTransform::from_to(camera_rect, viewport);
		let world_corners = super::get_world_corners(viewport, camera_pos, scale);
		let world_view_rect = Rect::from_two_pos(world_corners.0.to_pos2(), world_corners.1.to_pos2());
		Self {
			transform,
			viewport,
			camera_pos,
			world_view_rect,
			scale,
			zoom_level,
			ui_scale: response.ctx.pixels_per_point(),
			pointer_world_pos: super::pointer_world_pos(camera_pos, response, viewport, scale),
		}
	}

	// We translate every position directly instead of using the transform
	// due to epaint only supporting 32-bit floating point number
	#[inline]
	pub fn render_pos(&self, pos: &DVec2) -> Pos2 {
		let pos = (*pos - self.camera_pos).to_pos2();
		self.transform * Pos2::new(pos.x, -pos.y)
	}
}
