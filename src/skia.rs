use std::sync::OnceLock;

use egui::load::SizedTexture;
use egui::{
	paint_texture_at, ColorImage, ImageOptions, Response, ScrollArea, Sense, TextureHandle, TextureOptions, Ui, Widget,
};
use skia_safe::surfaces::raster_n32_premul;
use skia_safe::{Canvas, Color, Font, FontMgr, FontStyle, ISize, Point, Typeface};

use crate::render::RenderElement;

pub struct SkiaWidget<'a> {
	texture_handle: TextureHandle,
	render_data: &'a [RenderElement],
	render_callback: Box<SkiaCallback>,
	sense: Sense,
}

pub type SkiaCallback = dyn Fn(&Canvas, &Response, &[RenderElement]);

impl<'a> SkiaWidget<'a> {
	pub fn new(
		texture_handle: TextureHandle,
		render_data: &'a [RenderElement],
		sense: Sense,
		render_callback: Box<SkiaCallback>,
	) -> Self {
		Self {
			texture_handle,
			render_callback,
			sense,
			render_data,
		}
	}
}

impl Widget for SkiaWidget<'_> {
	fn ui(mut self, ui: &mut Ui) -> Response {
		ScrollArea::vertical()
			.show(ui, |ui| {
				ui.ctx().request_repaint();
				let (rect, response) = ui.allocate_at_least(ui.available_size(), self.sense);
				let scale = ui.ctx().pixels_per_point();
				let size = rect.size() * scale;
				// TODO: make widget size be pixel perfect
				let mut surface = raster_n32_premul(ISize::new(size.x as i32, size.y as i32)).unwrap();
				let canvas = surface.canvas();
				// TODO: scale canvas properly with ui
				// canvas.scale((scale, scale));
				canvas.clear(Color::WHITE);

				(self.render_callback)(canvas, &response, self.render_data);

				let image = surface.image_snapshot();

				let pixels = image.peek_pixels().unwrap();
				let color_image = ColorImage::from_rgba_premultiplied(
					[image.width() as _, image.height() as _],
					pixels.bytes().unwrap(),
				);

				self.texture_handle.set(color_image, TextureOptions::default());
				paint_texture_at(
					ui.painter(),
					rect,
					&ImageOptions::default(),
					&SizedTexture::from_handle(&self.texture_handle),
				);
				response
			})
			.inner
	}
}

pub fn center_text(center: impl Into<Point>, text: &str, font: &Font) -> Point {
	let center = center.into();
	let (_, bounds) = font.measure_text(text, None);
	Point::new(center.x - bounds.width() / 2.0, center.y + bounds.height() / 2.0)
}

pub fn default_typeface() -> Typeface {
	DEFAULT_TYPEFACE
		.get_or_init(|| {
			let font_mgr = FontMgr::new();
			font_mgr.legacy_make_typeface(None, FontStyle::default()).unwrap()
		})
		.clone()
}

static DEFAULT_TYPEFACE: OnceLock<Typeface> = OnceLock::new();
