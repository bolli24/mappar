use super::key_value_table::KeyValueTable;
use super::map_widget::css::Styles;
use super::map_widget::render::DetailedElement;
use super::map_widget::MapWidget;
use crate::imagery::esri_imagery;
use crate::imagery::imagery_fetcher::ImageryFetcher;
use crate::map::api::get_data;
use crate::map::edit::UpdateData;
use crate::map::map_data::{Bounds, LatLon, MapData};
use crate::ui::debug_text::DebugText;
use crate::ui::map_widget::css::get_default_styles;
use crate::ui::map_widget::map_memory::MapMemory;
use egui::{Id, Modifiers, ScrollArea, Sides, Widget, Window};
use log::{error, info};
#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;
use tokio::sync::watch::{channel, Receiver, Sender};
use undo::Record;
#[cfg(target_arch = "wasm32")]
use web_time::Instant;

pub struct Channel<T> {
	sender: Sender<T>,
	receiver: Receiver<T>,
}

impl<T> From<(Sender<T>, Receiver<T>)> for Channel<T> {
	fn from((sender, receiver): (Sender<T>, Receiver<T>)) -> Self {
		Self { sender, receiver }
	}
}

const MAX_RECENT_FRAME_COUNT: usize = 1000;

pub struct MyApp {
	map_data: Arc<Mutex<MapData>>,
	styles: Option<Styles>,
	render_data: Channel<Vec<DetailedElement>>,
	map_widget_id: Id,
	imagery_fetcher: Arc<Mutex<ImageryFetcher>>,
	debug_text: DebugText,
	startup_time: Instant,
	last_update: Instant,
	recent_frame_times: egui::util::History<Duration>,
	inspection: bool,
	debug_panel: bool,
	map_memory: MapMemory,
	#[cfg(not(target_arch = "wasm32"))]
	storage_dir: PathBuf,
	history: Record<UpdateData>,
}

impl MyApp {
	pub const APP_NAME: &'static str = "mappar";
	pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
		#[cfg(target_arch = "wasm32")]
		tokio::runtime::Builder::new_current_thread().build().unwrap();
		#[cfg(not(target_arch = "wasm32"))]
		cc.egui_ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(true));

		let bounds = Bounds {
			min: LatLon(51.82887, 12.19731),
			max: LatLon(51.82344, 12.21268),
		};

		let camera_pos = bounds.center().to_web_mercator();

		info!("Camera: {camera_pos:?}");
		info!("Camera: {:?}", LatLon::from_web_mercator(camera_pos));

		let map_widget_id = Id::new("map_widget");
		let map_memory = MapMemory {
			camera_pos: glam::dvec2(camera_pos.x, camera_pos.y),
			zoom_level: 15.0,
			..Default::default()
		};
		#[cfg(not(target_arch = "wasm32"))]
		let storage_dir = eframe::storage_dir(Self::APP_NAME).expect("Failed to get storage dir");

		#[cfg(not(target_arch = "wasm32"))]
		let imagery_fetcher = {
			let mut cache_path = storage_dir.clone();
			cache_path.push("cache");
			Arc::new(Mutex::new(ImageryFetcher::new(
				esri_imagery(),
				cc.egui_ctx.tex_manager(),
				cache_path,
			)))
		};

		#[cfg(target_arch = "wasm32")]
		let imagery_fetcher = Arc::new(Mutex::new(ImageryFetcher::new(
			esri_imagery(),
			cc.egui_ctx.tex_manager(),
		)));

		let app = Self {
			map_data: Arc::new(Mutex::new(MapData::new())),
			styles: get_default_styles(cc.egui_ctx.tex_manager().clone()),
			render_data: channel(Vec::new()).into(),
			imagery_fetcher,
			map_widget_id,
			debug_text: DebugText::new(),
			startup_time: Instant::now(),
			last_update: Instant::now(),
			recent_frame_times: egui::util::History::new(10..MAX_RECENT_FRAME_COUNT, 1.0),
			inspection: false,
			debug_panel: false,
			map_memory,
			#[cfg(not(target_arch = "wasm32"))]
			storage_dir,
			history: Record::new(),
		};

		let map_data = app.map_data.clone();
		let closure = async move {
			let new_map_data = get_data(bounds).await;
			match new_map_data {
				Ok(new_map_data) => {
					let mut data = map_data.lock().unwrap();
					*data = new_map_data;
				}
				Err(err) => {
					error!("Error fetching data:");
					error!("{:?}", err);
				}
			}
		};

		#[cfg(not(target_arch = "wasm32"))]
		tokio::spawn(closure);
		#[cfg(target_arch = "wasm32")]
		wasm_bindgen_futures::spawn_local(closure);

		app
	}
}

impl eframe::App for MyApp {
	fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
		#[cfg(not(target_arch = "wasm32"))]
		let mut reload_styles = false; // This is needed because we can't reload in the input closure
		#[cfg(not(target_arch = "wasm32"))]
		ctx.input(|input| {
			for e in &input.events {
				if let egui::Event::Key {
					pressed: true,
					key: egui::Key::F5,
					repeat: false,
					modifiers: egui::Modifiers::NONE,
					..
				} = e
				{
					reload_styles = true;
				}
				if let egui::Event::Key {
					pressed: true,
					key: egui::Key::F3,
					repeat: false,
					modifiers: egui::Modifiers::NONE,
					..
				} = e
				{
					self.debug_panel = !self.debug_panel;
				}
			}
		});

		egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
			egui::menu::bar(ui, |ui| {
				ui.menu_button("File", |ui| {
					ui.checkbox(&mut self.debug_panel, "Debug Panel");
					ui.checkbox(&mut self.inspection, "Egui Inspector");
					#[cfg(not(target_arch = "wasm32"))]
					{
						if ui.button("Reload Styles").clicked() {
							reload_styles = true
						}
						if ui.button("Open Config Folder").clicked() {
							open::that(&self.storage_dir)
								.unwrap_or_else(|e| error!("Couldn't open config folder: {e}"));
						}
						if ui.button("Quit").clicked() {
							ctx.send_viewport_cmd(egui::ViewportCommand::Close);
						}
					}
				});
				ui.add_space(16.0);
				egui::widgets::global_theme_preference_switch(ui);
			});
		});

		#[cfg(not(target_arch = "wasm32"))]
		if reload_styles {
			info!("Reloading MapCSS styles.");
			self.styles = get_default_styles(ctx.tex_manager())
		}

		egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
			Sides::new().show(
				ui,
				|ui| ui.hyperlink("https://gitlab.com/bolli24/mappar"),
				|ui| {
					ui.spacing_mut().item_spacing.x = 0.0;
					let commit_hash = env!("VERGEN_GIT_SHA");
					if ui.link(&commit_hash[0..7]).clicked() {
						ui.output_mut(|o| o.copied_text = commit_hash.to_string())
					}
					ui.label(format!("{}/", env!("CARGO_PKG_VERSION")));
				},
			);
		});
		egui::SidePanel::right("edit_panel")
			.width_range(80.0..=800.0)
			.show(ctx, |ui| {
				KeyValueTable::new(self.map_data.clone(), &mut self.map_memory.selection, &mut self.history).ui(ui);
			});

		if self.debug_panel {
			egui::SidePanel::right("debug_panel")
				.width_range(80.0..=400.0)
				.show(ctx, |ui| {
					ui.heading("Debug Panel");
					ui.separator();
					ScrollArea::vertical().show(ui, |ui| {
						ui.add(self.debug_text.clone());
						self.debug_text.clear();
					})
				});
		}

		egui::CentralPanel::default().show(ctx, |ui| {
			ui.add(MapWidget::new(
				&self.map_data.lock().unwrap(),
				&mut self.styles,
				self.imagery_fetcher.clone(),
				self.map_widget_id,
				&mut self.debug_text,
				&mut self.map_memory,
			));
		});

		Window::new("Egui Inpector")
			.open(&mut self.inspection)
			.vscroll(true)
			.show(ctx, |ui| ctx.inspection_ui(ui));

		ctx.input(|i| {
			if i.modifiers == Modifiers::CTRL | Modifiers::COMMAND {
				let change_result = if i.key_pressed(egui::Key::Z) {
					let mut map_data = self.map_data.lock().unwrap();
					self.history.undo(&mut map_data)
				} else if i.key_pressed(egui::Key::Y) {
					let mut map_data = self.map_data.lock().unwrap();
					self.history.redo(&mut map_data)
				} else {
					None
				};

				if let Some(change_result) = change_result {
					match change_result {
						Ok(id) => {
							self.map_memory.selection.selected_elements.clear();
							self.map_memory.selection.selected_elements.insert(id);
						}
						Err(_err) => {
							todo!()
						}
					}
				}
			}
		});

		self.recent_frame_times.flush(self.startup_time.elapsed().as_secs_f64());
		let last_frame = self.last_update.elapsed();
		let last_frame_ms = last_frame.as_micros() as f32 / 1000.0;
		self.recent_frame_times
			.add(self.startup_time.elapsed().as_secs_f64(), last_frame);
		let average_ms = self
			.recent_frame_times
			.values()
			.fold(Duration::ZERO, |a, b| a + b)
			.as_micros() as f64
			/ self.recent_frame_times.len() as f64
			/ 1000.0;
		let worst_ms = self
			.recent_frame_times
			.values()
			.max()
			.unwrap_or(Duration::ZERO)
			.as_micros() as f32
			/ 1000.0;
		self.debug_text.add(format!(
			"Frame Time: {:.1}ms\nAverage: {:.1}ms ({:.1} FPS)\nWorst: {:.1}ms",
			last_frame_ms,
			average_ms,
			1000.0 / average_ms,
			worst_ms
		));
		puffin::GlobalProfiler::lock().new_frame();
		self.last_update = Instant::now();
	}
}
