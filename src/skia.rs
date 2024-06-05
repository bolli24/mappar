use std::cell::RefCell;
use std::rc::Rc;

use egui::load::SizedTexture;
use egui::{
    paint_texture_at, pos2, Color32, ColorImage, ImageOptions, Rect, Response, TextureHandle,
    TextureOptions, Ui, Widget,
};
use skia_safe::surfaces::raster_n32_premul;
use skia_safe::ISize;

pub struct SkiaWidget {
    texture_handle: Rc<RefCell<TextureHandle>>,
}

impl SkiaWidget {
    pub(crate) fn new(texture_handle: Rc<RefCell<TextureHandle>>) -> Self {
        Self { texture_handle }
    }
}

impl Widget for SkiaWidget {
    fn ui(self, ui: &mut Ui) -> Response {
        let response = ui.allocate_response(egui::Vec2::splat(300.0), egui::Sense::click());
        let rect = response.rect;
        let mut surface =
            raster_n32_premul(ISize::new(rect.width() as i32, rect.height() as i32)).unwrap();
        let canvas = surface.canvas();
        canvas.clear(skia_safe::Color::LIGHT_GRAY);
        let mut paint = skia_safe::Paint::default();
        paint.set_color(skia_safe::Color::RED);
        paint.set_anti_alias(true);
        paint.set_stroke_width(10.0);
        canvas.draw_circle((rect.width() / 2.0, rect.height() / 2.0), 100.0, &paint);
        let image = surface.image_snapshot();
        /* let png = image.encode_to_data(skia_safe::EncodedImageFormat::PNG).unwrap();
        let mut file = std::fs::File::create("output.png").unwrap();
        std::io::Write::write_all(&mut file, &png).unwrap();
        panic!();*/

        let pixels = image.peek_pixels().unwrap();
        let color_image = ColorImage::from_rgba_premultiplied(
            [image.width() as _, image.height() as _],
            pixels.bytes().unwrap(),
        );

        self.texture_handle
            .borrow_mut()
            .set(color_image, TextureOptions::default());
        paint_texture_at(
            ui.painter(),
            rect,
            &ImageOptions::default(),
            &SizedTexture::from_handle(&self.texture_handle.borrow()),
        );
        response
    }
}
