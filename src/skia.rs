use std::sync::OnceLock;

use egui::load::SizedTexture;
use egui::{
    paint_texture_at, ColorImage, Context, ImageOptions, Response, Sense, TextureHandle,
    TextureOptions, Ui, Vec2, Widget,
};
use skia_safe::surfaces::raster_n32_premul;
use skia_safe::{Canvas, Color, Font, FontMgr, FontStyle, ISize, Point, Typeface};

pub struct SkiaWidget {
    texture_handle: TextureHandle,
    render_callback: Box<SkiaCallback>,
}

pub type SkiaCallback = dyn Fn(&Canvas, &Context);

impl SkiaWidget {
    pub fn new(
        texture_handle: TextureHandle,
        render_callback: Box<dyn Fn(&Canvas, &Context)>,
    ) -> Self {
        Self {
            texture_handle,
            render_callback,
        }
    }
}

impl Widget for SkiaWidget {
    fn ui(mut self, ui: &mut Ui) -> Response {
        ui.ctx().request_repaint();
        let (rect, response) = ui.allocate_at_least(Vec2::splat(300.0), Sense::click());
        let scale = ui.ctx().pixels_per_point();
        let size = rect.size() * scale;
        let mut surface = raster_n32_premul(ISize::new(size.x as i32, size.y as i32)).unwrap();
        let canvas = surface.canvas();
        // TODO: scale canvas properly with ui
        // canvas.scale((scale, scale));
        canvas.clear(Color::WHITE);

        (self.render_callback)(canvas, ui.ctx());

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

pub fn center_text(center: impl Into<Point>, text: &str, font: &Font) -> Point {
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
