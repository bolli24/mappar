use egui::{TextureHandle, Widget};
use skia_safe::Canvas;

use crate::render::RenderElement;
use crate::skia::SkiaWidget;

pub struct MapWidget<'a> {
	texture_handle: TextureHandle,
	render_data: &'a [RenderElement],
	pub zoom_level: f32,
	pub camera_pos: (f32, f32),
}

impl<'a> MapWidget<'a> {
	pub fn new(
		texture_handle: TextureHandle,
		render_data: &'a [RenderElement],
		zoom_level: f32,
		camera_pos: (f32, f32),
	) -> Self {
		Self {
			texture_handle,
			render_data,
			zoom_level,
			camera_pos,
		}
	}

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
				panic!()
			}
		}
	}
}

impl Widget for MapWidget<'_> {
	fn ui(self, ui: &mut egui::Ui) -> egui::Response {
		return ui.add(SkiaWidget::new(
			self.texture_handle,
			self.render_data,
			Box::new(move |canvas, ctx, render_data| {
				let size = canvas.image_info().dimensions();
				/* let si = (ctx.frame_nr() as f32 / 30.0).sin();

				let center = Point::new(size.width as f32 / 2.0, size.height as f32 / 2.0);
				canvas.save();
				canvas.rotate(si * 5.0, Some(center));

				let mut paint = Paint::default();
				paint.set_color(Color::DARK_GRAY);
				let font = Font::from_typeface(default_typeface(), 30.0);
				let text = TextBlob::from_str("Hello, Skia!", &font).unwrap();
				canvas.draw_text_blob(
					&text,
					center_text(
						(size.width as f32 / 2.0, size.height as f32 / 2.0),
						"Hello, Skia!",
						&font,
					),
					&paint,
				);

				let mut circle_paint = Paint::default();
				circle_paint.set_color(Color::LIGHT_GRAY);
				circle_paint.set_path_effect(PathEffect::dash(&[si * 5.0 + 20.0, si * 5.0 + 20.0], 1.0));
				circle_paint.set_style(skia_safe::PaintStyle::Stroke);
				circle_paint.set_anti_alias(true);
				circle_paint.set_stroke_width(10.0);
				circle_paint.set_stroke_cap(skia_safe::PaintCap::Round);
				canvas.draw_circle(center, 100.0, &circle_paint);

				canvas.restore(); */

				canvas.translate((size.width as f32 / 2.0, size.height as f32 / 2.0));
				canvas.scale((self.zoom_level, -self.zoom_level));
				canvas.translate((-self.camera_pos.0, -self.camera_pos.1));

				for element in render_data {
					Self::draw_element(canvas, element);
				}
			}),
		));
	}
}
