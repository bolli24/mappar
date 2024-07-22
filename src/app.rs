use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use egui::{Id, ViewportCommand};
use log::info;
use poll_promise::Promise;

use crate::api;
use crate::memory::Memory;
use crate::debug_text::DebugText;
use crate::imagery::esri_imagery;
use crate::imagery_fetcher::ImageryFetcher;
use crate::map_data::{Bounds, LatLon, MapData};
use crate::map_widget::{MapMemory, MapRenderData, MapWidget};

use crate::render::{DetailedElement, RenderElement};

pub struct MyApp {
	map_data_promise: Option<Promise<Result<MapData, api::Error>>>,
	render_data_promise: Promise<Vec<DetailedElement>>,
	map_data: Arc<Mutex<MapData>>,
	map_widget_id: Id,
	imagery_fetcher: Arc<Mutex<ImageryFetcher>>,
	debug_text: Rc<RefCell<DebugText>>,
	last_update: Instant,
}

impl MyApp {
	pub const APP_NAME: &'static str = "mappar";

	pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
		cc.egui_ctx.send_viewport_cmd(ViewportCommand::Maximized(true));

		let bounds = Bounds {
			min: LatLon(51.82887, 12.19731),
			max: LatLon(51.82344, 12.21268),
		};

		let camera_pos = bounds.center().to_web_mercator();

		info!("Camera: {camera_pos:?}");
		info!("Camera: {:?}", LatLon::from_web_mercator(camera_pos));

		let map_widget_id = Id::new("map_widget");
		let map_mem = MapMemory {
			camera_pos: glam::dvec2(camera_pos.x, camera_pos.y),
			zoom_level: 15.0,
		};
		map_mem.store(&cc.egui_ctx, map_widget_id);

		let mut cache_path = eframe::storage_dir(Self::APP_NAME).expect("Failed to get storage dir");
		cache_path.push("cache");

		Self {
			map_data_promise: Some(Promise::spawn_async(api::get_data(Bounds {
				min: LatLon(51.82887, 12.19731),
				max: LatLon(51.82344, 12.21268),
			}))),
			render_data_promise: Promise::spawn_blocking(Vec::new),
			imagery_fetcher: Arc::new(Mutex::new(ImageryFetcher::new(
				esri_imagery(),
				cc.egui_ctx.tex_manager(),
				cache_path,
			))),
			map_data: Arc::new(Mutex::new(MapData::new())),
			map_widget_id,
			debug_text: Rc::new(RefCell::new(DebugText::new())),
			last_update: Instant::now(),
		}
	}
}

impl eframe::App for MyApp {
	fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
		if self.map_data_promise.is_some() {
			if let Some(Ok(_)) = self.map_data_promise.as_mut().unwrap().ready() {
				let new_map_data = self
					.map_data_promise
					.take()
					.unwrap()
					.block_and_take()
					.expect("Error fetching data");
				*self.map_data.lock().unwrap() = new_map_data;
				let mutex_data = self.map_data.clone();
				self.render_data_promise = Promise::spawn_thread("render_data_calc", move || {
					let start = Instant::now();
					let mut render_data = Vec::new();
					let map_data = mutex_data.lock().unwrap();
					for element in map_data.iter() {
						render_data.push(DetailedElement::from_map_element(element, &map_data));
					}
					info!("Calculated render data. Took: {:?} ", start.elapsed());

					render_data
				});
			}
		}

		egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
			egui::menu::bar(ui, |ui| {
				ui.menu_button("File", |ui| {
					if ui.button("Quit").clicked() {
						ctx.send_viewport_cmd(ViewportCommand::Close);
					}
				});
				ui.add_space(16.0);
				egui::widgets::global_dark_light_mode_buttons(ui);
			});
		});

		egui::SidePanel::right("debug_panel").min_width(240.0).show(ctx, |ui| {
			ui.heading("Debug Panel");
			let response = ui.add(self.debug_text.borrow().clone());
			self.debug_text.borrow_mut().clear();
			response
		});

		egui::CentralPanel::default().show(ctx, |ui| {
			// The central panel the region left after adding TopPanel's and SidePanel's
			/*ui.heading("eframe template");

			ui.horizontal(|ui| {
				ui.label("Write something: ");
				ui.text_edit_singleline(&mut self.label);
			});

			ui.add(egui::Slider::new(&mut self.value, 0.0..=10.0).text("value"));
			if ui.button("Increment").clicked() {
				self.value += 1.0;
			}*/

			ui.add(MapWidget::new(
				MapRenderData {
					elements: self.render_data_promise.ready().unwrap_or(&Vec::new()),
					imagery_fetcher: self.imagery_fetcher.clone(),
				},
				self.map_widget_id,
				self.debug_text.clone(),
			));

			/*ui.separator();

			ui.add(egui::github_link_file!(
				"https://github.com/emilk/eframe_template/blob/main/",
				"Source code."
			));

			ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
				powered_by_egui_and_eframe(ui);
				egui::warn_if_debug_build(ui);
			});*/
		});
		puffin::GlobalProfiler::lock().new_frame();
		let diff = self.last_update.elapsed();
		self.debug_text
			.borrow_mut()
			.add(format!("Frame Time: {:.1}ms", diff.as_millis()));
		self.last_update = Instant::now();
	}
}

fn powered_by_egui_and_eframe(ui: &mut egui::Ui) {
	ui.horizontal(|ui| {
		ui.spacing_mut().item_spacing.x = 0.0;
		ui.label("Powered by ");
		ui.hyperlink_to("egui", "https://github.com/emilk/egui");
		ui.label(" and ");
		ui.hyperlink_to("eframe", "https://github.com/emilk/egui/tree/master/crates/eframe");
		ui.label(".");
	});
}
