use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use eframe::epaint::textures::TextureOptions;
use eframe::epaint::{ColorImage, ImageData, TextureHandle};
use skia_safe::{scalar, Color, ColorType, Font, Paint, PathEffect, Point, Size, Surface};

use crate::skia::SkiaWidget;

pub struct MyApp {
    // Example stuff:
    label: String,
    value: f32,
    skia_texture: Rc<RefCell<TextureHandle>>,
}

impl MyApp {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let tex_manager = cc.egui_ctx.tex_manager();
        let skia_tex = tex_manager.write().alloc(
            "skia_texture".into(),
            ImageData::Color(Arc::new(ColorImage::from_rgba_premultiplied(
                [256, 256],
                &vec![0; 256 * 256 * 4],
            ))),
            TextureOptions::default(),
        );

        MyApp {
            label: "Hello, World!".to_owned(),
            skia_texture: Rc::new(RefCell::new(TextureHandle::new(
                cc.egui_ctx.tex_manager(),
                skia_tex,
            ))),
            value: 0.0,
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Put your widgets into a `SidePanel`, `TopBottomPanel`, `CentralPanel`, `Window` or `Area`.
        // For inspiration and more examples, go to https://emilk.github.io/egui

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:

            egui::menu::bar(ui, |ui| {
                // NOTE: no File->Quit on web pages!
                let is_web = cfg!(target_arch = "wasm32");
                if !is_web {
                    ui.menu_button("File", |ui| {
                        if ui.button("Quit").clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });
                    ui.add_space(16.0);
                }

                egui::widgets::global_dark_light_mode_buttons(ui);
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            // The central panel the region left after adding TopPanel's and SidePanel's
            ui.heading("eframe template");

            ui.horizontal(|ui| {
                ui.label("Write something: ");
                ui.text_edit_singleline(&mut self.label);
            });

            ui.add(egui::Slider::new(&mut self.value, 0.0..=10.0).text("value"));
            if ui.button("Increment").clicked() {
                self.value += 1.0;
            }

            ui.add(SkiaWidget::new(self.skia_texture.clone()));

            // let si = (2.0 as f32 / 120.0).sin();

            /*ui.painter().add(egui::PaintCallback {
                rect,
                callback: Arc::new(SkiaPaintCallback::new(move |canvas| {
                    info!("Draw");
                    let center = Point::new(150.0, 150.0);
                    canvas.save();
                    canvas.rotate(si * 5.0, Some(center));
                    let mut paint = Paint::default();
                    paint.set_color(Color::from_argb(255, 255, 255, 255));
                    canvas.draw_str(
                        "Hello Skia!",
                        Point::new(100.0, 150.0),
                        &Font::default().with_size(20.0).unwrap(),
                        &paint,
                    );

                    let mut circle_paint = Paint::default();
                    circle_paint.set_path_effect(PathEffect::dash(
                        &[si * 5.0 + 20.0, si * 5.0 + 20.0],
                        1.0,
                    ));
                    circle_paint.set_style(skia_safe::PaintStyle::Stroke);
                    circle_paint.set_stroke_width(10.0);
                    circle_paint.set_stroke_cap(skia_safe::PaintCap::Round);
                    canvas.draw_circle(center, 100.0, &circle_paint);
                })),
            });*/

            ui.separator();

            ui.add(egui::github_link_file!(
                "https://github.com/emilk/eframe_template/blob/main/",
                "Source code."
            ));

            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                powered_by_egui_and_eframe(ui);
                egui::warn_if_debug_build(ui);
            });
        });
    }
}

fn powered_by_egui_and_eframe(ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.label("Powered by ");
        ui.hyperlink_to("egui", "https://github.com/emilk/egui");
        ui.label(" and ");
        ui.hyperlink_to(
            "eframe",
            "https://github.com/emilk/egui/tree/master/crates/eframe",
        );
        ui.label(".");
    });
}
