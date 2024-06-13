use std::sync::Arc;

use egui::{Context, TextureHandle, Widget};
use skia_safe::{Canvas, Color, Font, Paint, PathEffect, Point, TextBlob};

use crate::render::RenderElement;
use crate::skia::{center_text, default_typeface, SkiaWidget};

pub struct MapWidget<'a> {
    texture_handle: TextureHandle,
    render_data: &'a Vec<RenderElement>,
}

impl<'a> MapWidget<'a> {
    pub fn new(texture_handle: TextureHandle, render_data: &'a Vec<RenderElement>) -> Self {
        Self {
            texture_handle,
            render_data,
        }
    }
}

impl Widget for MapWidget<'_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        return ui.add(SkiaWidget::new(
            self.texture_handle,
            Box::new(|canvas: &Canvas, ctx: &Context| {
                let size = canvas.image_info().dimensions();
                let si = (ctx.frame_nr() as f32 / 30.0).sin();

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
                circle_paint
                    .set_path_effect(PathEffect::dash(&[si * 5.0 + 20.0, si * 5.0 + 20.0], 1.0));
                circle_paint.set_style(skia_safe::PaintStyle::Stroke);
                circle_paint.set_anti_alias(true);
                circle_paint.set_stroke_width(10.0);
                circle_paint.set_stroke_cap(skia_safe::PaintCap::Round);
                canvas.draw_circle(center, 100.0, &circle_paint);
            }),
        ));
    }
}
