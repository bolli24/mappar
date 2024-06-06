use std::sync::OnceLock;

use egui::load::SizedTexture;
use egui::{
    paint_texture_at, ColorImage, ImageOptions, Response, TextureHandle, TextureOptions, Ui, Widget,
};
use skia_safe::surfaces::raster_n32_premul;
use skia_safe::{
    Color, Font, FontMgr, FontStyle, ISize, Paint, PathEffect, Point, TextBlob, Typeface,
};

pub struct SkiaWidget {
    texture_handle: TextureHandle,
}

impl SkiaWidget {
    pub fn new(texture_handle: TextureHandle) -> Self {
        Self { texture_handle }
    }
}

impl Widget for SkiaWidget {
    fn ui(mut self, ui: &mut Ui) -> Response {
        let (rect, response) = ui.allocate_at_least(egui::Vec2::splat(300.0), egui::Sense::click());
        let size = rect.size() * ui.ctx().pixels_per_point();
        let mut surface = raster_n32_premul(ISize::new(size.x as i32, size.y as i32)).unwrap();
        let canvas = surface.canvas();
        ui.ctx().request_repaint();
        let si = (ui.ctx().frame_nr() as f32 / 30.0).sin();

        let center = Point::new(size.x / 2.0, size.y / 2.0);
        canvas.save();
        canvas.rotate(si * 5.0, Some(center));

        let mut paint = Paint::default();
        paint.set_color(Color::WHITE);
        let font = Font::from_typeface(default_typeface(), 30.0);
        let text = TextBlob::from_str("Hello, Skia!", &font).unwrap();
        canvas.draw_text_blob(
            &text,
            center_text((size.x / 2.0, size.y / 2.0), "Hello, Skia!", &font),
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

        let image = surface.image_snapshot();

        let pixels = image.peek_pixels().unwrap();
        let color_image = ColorImage::from_rgba_premultiplied(
            [image.width() as _, image.height() as _],
            pixels.bytes().unwrap(),
        );

        self.texture_handle
            .set(color_image, TextureOptions::default());
        paint_texture_at(
            ui.painter(),
            rect,
            &ImageOptions::default(),
            &SizedTexture::from_handle(&self.texture_handle),
        );
        response
    }
}

fn center_text(center: impl Into<Point>, text: &str, font: &Font) -> Point {
    let center = center.into();
    let (_, bounds) = font.measure_text(text, None);
    Point::new(
        center.x - bounds.width() / 2.0,
        center.y + bounds.height() / 2.0,
    )
}

pub fn default_typeface() -> Typeface {
    DEFAULT_TYPEFACE
        .get_or_init(|| {
            let font_mgr = FontMgr::new();
            font_mgr
                .legacy_make_typeface(None, FontStyle::default())
                .unwrap()
        })
        .clone()
}

static DEFAULT_TYPEFACE: OnceLock<Typeface> = OnceLock::new();
