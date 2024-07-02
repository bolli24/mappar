use std::sync::OnceLock;

use egui::load::SizedTexture;
use egui::{
	paint_texture_at, Color32, ColorImage, ImageOptions, Response, ScrollArea, Sense, TextureHandle, TextureOptions,
	Ui, Widget,
};
use skia_safe::surfaces::raster;
use skia_safe::{
	AlphaType, Canvas, Color, ColorSpace, ColorType, Font, FontMgr, FontStyle, ISize, ImageInfo, Point, Typeface,
};

pub struct SkiaWidget<T> {
	texture_handle: TextureHandle,
	render_data: T,
	render_callback: Box<SkiaCallback<T>>,
	sense: Sense,
}

pub type SkiaCallback<T> = dyn Fn(&Canvas, &Response, &T);

impl<T> SkiaWidget<T> {
	pub fn new(
		texture_handle: TextureHandle,
		render_data: T,
		sense: Sense,
		render_callback: Box<SkiaCallback<T>>,
	) -> Self {
		Self {
			texture_handle,
			render_data,
			render_callback,
			sense,
		}
	}
}

impl<T> Widget for SkiaWidget<T> {
	fn ui(mut self, ui: &mut Ui) -> Response {
		ScrollArea::vertical()
			.show(ui, |ui| {
				ui.ctx().request_repaint();
				let (rect, response) = ui.allocate_at_least(ui.available_size(), self.sense);
				let scale = ui.ctx().pixels_per_point();
				let size = rect.size() * scale;
				// TODO: make widget size be pixel perfect
				let image_info = ImageInfo::new_n32(
					ISize::new(size.x as i32, size.y as i32),
					AlphaType::Premul,
					ColorSpace::new_srgb(),
				)
				.with_color_type(ColorType::SRGBA8888); // Need to set Colortype to RGB because the default is BGRA
				let mut surface = raster(&image_info, None, None).unwrap();
				let canvas = surface.canvas();
				// TODO: scale canvas properly with ui
				canvas.clear(Color::TRANSPARENT);

				(self.render_callback)(canvas, &response, &self.render_data);

				let image = surface.image_snapshot();

				let pixels = image.peek_pixels().unwrap();
				let color_image = ColorImage::from_rgba_premultiplied(
					[image.width() as _, image.height() as _],
					pixels.bytes().unwrap(),
				);
				self.texture_handle.set(color_image, TextureOptions::default());
				let image_options = ImageOptions {
					bg_fill: Color32::WHITE,
					rounding: 3.0.into(),
					..ImageOptions::default()
				};
				paint_texture_at(
					ui.painter(),
					rect,
					&image_options,
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
