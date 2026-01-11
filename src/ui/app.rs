use super::imagery_widget::ImageriesWidget;
use super::layer_list::Layers;
use super::map_widget::MapWidget;
use super::map_widget::css::Styles;
use super::map_widget::map_state::MapStateMachine;
use super::messages::Messages;
use crate::imagery::imagery_fetcher::ImageryFetcher;
use crate::imagery::{Imageries, get_imagery};
use crate::map::api::get_data;
use crate::map::map_data::{Bounds, LatLon, MapData, latlon};
#[cfg(not(target_arch = "wasm32"))]
use crate::message_error;
use crate::ui::debug_text::DebugText;
use crate::ui::edit_bar::EditBar;
use crate::ui::layer_list::{Layer, LayerList};
use crate::ui::map_widget::css::get_default_styles;
use crate::ui::map_widget::map_memory::MapMemory;
use crate::{dbgtxt, unreachable_log};
#[cfg(not(target_arch = "wasm32"))]
use directories::ProjectDirs;
use egui::{Id, Modifiers, ScrollArea, Sides, Widget, Window};
use log::{error, info};
use statig::prelude::{IntoStateMachineExt, StateMachine};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use web_time::Instant;

use epaint::mutex::RwLock;

const MAX_RECENT_FRAME_COUNT: usize = 1000;

pub struct MyApp {
	map_data: MapData,
	pending_map_data: Arc<Mutex<Option<MapData>>>,
	styles: Option<Styles>,
	map_widget_id: Id,
	imagery_fetcher: Arc<Mutex<ImageryFetcher>>,
	imageries: Arc<RwLock<Imageries>>,
	layers: Layers,
	debug_text: DebugText,
	startup_time: Instant,
	last_update: Instant,
	recent_frame_times: egui::util::History<Duration>,
	inspection: bool,
	debug_panel: bool,
	map_memory: MapMemory,
	messages: Messages,
	imagery_settings: bool,
	#[cfg(not(target_arch = "wasm32"))]
	project_dirs: directories::ProjectDirs,
	map_state: StateMachine<MapStateMachine>,
	backend: String,
	no_cache: bool,
}

impl MyApp {
	pub const APP_NAME: &'static str = "mappar";
	#[must_use]
	pub fn new(cc: &eframe::CreationContext<'_>, no_cache: bool) -> Self {
		#[cfg(not(target_arch = "wasm32"))]
		cc.egui_ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(true));

		let mut fonts = egui::FontDefinitions::default();
		egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Bold);

		cc.egui_ctx.set_fonts(fonts);

		let bounds = Bounds {
			min: latlon(51.82887, 12.19731),
			max: latlon(51.82344, 12.21268),
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

		let messages = Messages::new();

		#[cfg(not(target_arch = "wasm32"))]
		let project_dirs = Self::project_dirs();

		let imageries = Arc::new(RwLock::new(Imageries::new()));

		let imagery_fetcher = {
			Arc::new(Mutex::new(ImageryFetcher::new(
				imageries.clone(),
				cc.egui_ctx.tex_manager(),
				#[cfg(not(target_arch = "wasm32"))]
				project_dirs.cache_dir().to_path_buf(),
				messages.clone(),
				no_cache,
			)))
		};

		let backend = cc.wgpu_render_state.as_ref().map_or_else(
			|| "Unknown".to_string(),
			|state| state.adapter.get_info().backend.to_string(),
		);

		let pending_map_data = Arc::new(Mutex::new(None));

		crate::spawn_task!({
			let pending_map_data = pending_map_data.clone();
			async move {
				let new_map_data = get_data(bounds).await;
				match new_map_data {
					Ok(new_map_data) => {
						let mut pending = pending_map_data.lock().unwrap();
						*pending = Some(new_map_data);
					}
					Err(err) => {
						error!("Error fetching data:");
						error!("{err:?}");
					}
				}
			}
		});

		let mut app = Self {
			map_data: MapData::new(),
			pending_map_data,
			styles: get_default_styles(cc.egui_ctx.tex_manager()),
			imagery_fetcher,
			imageries,
			layers: Layers::new(),
			map_widget_id,
			debug_text: DebugText::new(),
			startup_time: Instant::now(),
			last_update: Instant::now(),
			recent_frame_times: egui::util::History::new(10..MAX_RECENT_FRAME_COUNT, 1.0),
			inspection: false,
			debug_panel: true,
			map_memory,
			messages,
			imagery_settings: false,
			#[cfg(not(target_arch = "wasm32"))]
			project_dirs,
			map_state: MapStateMachine::state_machine(MapStateMachine::default()),
			backend,
			no_cache,
		};

		app.layers.0.push_back(Layer("EsriWorldImagery".into(), true));
		app.layers.0.push_back(Layer("standard".into(), true));

		#[cfg(not(target_arch = "wasm32"))]
		{
			use std::fs::create_dir_all;
			create_dir_all(app.project_dirs.config_dir())
				.unwrap_or_else(|e| message_error!(app.messages, "Couldn't create config directory: {e}"));
			create_dir_all(app.project_dirs.data_dir())
				.unwrap_or_else(|e| message_error!(app.messages, "Couldn't create data directory: {e}"));
			create_dir_all(app.project_dirs.cache_dir())
				.unwrap_or_else(|e| message_error!(app.messages, "Couldn't create cache directory: {e}"));
			create_dir_all(app.project_dirs.data_local_dir())
				.unwrap_or_else(|e| message_error!(app.messages, "Couldn't create local data directory: {e}"));
		}

		get_imagery(
			#[cfg(not(target_arch = "wasm32"))]
			app.project_dirs.data_dir().join(crate::imagery::JOSM_FILE),
			app.messages.clone(),
			app.imageries.clone(),
		);

		app
	}

	#[cfg(not(target_arch = "wasm32"))]
	#[must_use]
	pub fn project_dirs() -> ProjectDirs {
		let Some(project_dirs) = ProjectDirs::from("", "", Self::APP_NAME) else {
			error!("Unable to get project directories.");
			std::process::exit(1);
		};

		project_dirs
	}
}

impl eframe::App for MyApp {
	fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
		let maybe_new_map_data = self.pending_map_data.lock().unwrap().take();
		if let Some(new_map_data) = maybe_new_map_data {
			self.map_data = new_map_data;
		}

		#[cfg(not(target_arch = "wasm32"))]
		let mut reload_styles = false; // This is needed because we can't reload in the input closure
		#[cfg(not(target_arch = "wasm32"))]
		ctx.input(|input| {
			for e in &input.events {
				if let egui::Event::Key {
					pressed: true,
					key: egui::Key::F5,
					repeat: false,
					modifiers: Modifiers::NONE,
					..
				} = e
				{
					reload_styles = true;
				}
				if let egui::Event::Key {
					pressed: true,
					key: egui::Key::F3,
					repeat: false,
					modifiers: Modifiers::NONE,
					..
				} = e
				{
					self.debug_panel = !self.debug_panel;
				}
			}
		});

		dbgtxt!(self.debug_text, "History count: {}", self.map_data.history_len());

		dbgtxt!(
			self.debug_text,
			"Loaded imageries: {}",
			self.imageries.read().entries.len()
		);

		dbgtxt!(self.debug_text, "Backend: {}", self.backend);

		let mut update_edit_mode = None;

		egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
			egui::MenuBar::new().ui(ui, |ui| {
				ui.menu_button("File", |ui| {
					ui.checkbox(&mut self.debug_panel, "Debug Panel");
					ui.checkbox(&mut self.inspection, "Egui Inspector");
					#[cfg(not(target_arch = "wasm32"))]
					{
						if ui.button("Reload Styles").clicked() {
							ui.close();
							reload_styles = true;
						}
						if ui.button("Open Config Directory").clicked() {
							ui.close();
							open::that(self.project_dirs.config_dir())
								.unwrap_or_else(|e| error!("Couldn't open config directory: {e}"));
						}
						if ui.button("Open Data Directory").clicked() {
							ui.close();
							open::that(self.project_dirs.data_dir())
								.unwrap_or_else(|e| error!("Couldn't open data directory: {e}"));
						}
						if ui.button("Open Cache Directory").clicked() {
							ui.close();
							open::that(self.project_dirs.cache_dir())
								.unwrap_or_else(|e| error!("Couldn't open config directory: {e}"));
						}
						if ui.button("Open Logs Directory").clicked() {
							ui.close();
							open::that(self.project_dirs.data_local_dir().join("logs"))
								.unwrap_or_else(|e| error!("Couldn't open logs directory: {e}"));
						}
						if ui.button("Quit").clicked() {
							ctx.send_viewport_cmd(egui::ViewportCommand::Close);
						}
					}
				});
				ui.menu_button("Imagery", |ui| {
					if ui.button("Imagery Settings").clicked() {
						ui.close();
						self.imagery_settings = !self.imagery_settings;
					}
					if ui.button("Update JOSM imagery").clicked() {
						ui.close();
						crate::imagery::update_josm_imagery(
							#[cfg(not(target_arch = "wasm32"))]
							self.project_dirs.data_dir().join(crate::imagery::JOSM_FILE),
							self.messages.clone(),
							self.imageries.clone(),
						);
					}
				});
				ui.add_space(16.0);
				egui::widgets::global_theme_preference_switch(ui);
			});

			ui.add(EditBar::new(
				self.map_state.edit_mode(),
				&mut update_edit_mode,
				&mut self.map_memory,
				&mut self.map_data,
			));
		});

		#[cfg(not(target_arch = "wasm32"))]
		if reload_styles {
			info!("Reloading MapCSS styles.");
			self.styles = get_default_styles(ctx.tex_manager());
		}

		egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
			Sides::new().show(
				ui,
				|ui| ui.hyperlink("https://gitlab.com/bolli24/mappar"),
				|ui| {
					if env!("VERGEN_GIT_DIRTY") == "true" {
						ui.spacing_mut().item_spacing.x = 0.0;
						ui.label("+dirty");
					} else {
						ui.label(format!("({})", env!("VERGEN_GIT_COMMIT_DATE")));
						ui.spacing_mut().item_spacing.x = 0.0;
					}

					let commit_hash = env!("VERGEN_GIT_SHA");
					ui.hyperlink_to(
						&commit_hash[0..7],
						format!("https://gitlab.com/bolli24/mappar/tree/{commit_hash}"),
					);
					ui.label(format!("{}/", env!("CARGO_PKG_VERSION")));
				},
			);
		});
		egui::SidePanel::right("edit_panel")
			.width_range(80.0..=800.0)
			.default_width(200.0)
			.show(ctx, |ui| {
				LayerList::new(&mut self.layers, &self.imageries.read(), &mut self.imagery_settings).ui(ui);
			});

		if self.debug_panel {
			egui::SidePanel::right("debug_panel")
				.width_range(80.0..=400.0)
				.show(ctx, |ui| {
					ui.heading("Debug Panel");
					ui.separator();
					ScrollArea::vertical().show(ui, |ui| {
						ui.add(self.debug_text.clone());
					})
				});
		}
		self.debug_text.clear();

		egui::CentralPanel::default().show(ctx, |ui| {
			let active_imagery_ref = self
				.layers
				.get_active_imagery()
				.and_then(|id| self.imageries.write().get_ref(id));

			let mut rect = ui
				.add(MapWidget::new(
					&mut self.map_data,
					&mut self.styles,
					self.imagery_fetcher.clone(),
					self.map_widget_id,
					Some(&mut self.debug_text),
					&mut self.map_memory,
					true,
					active_imagery_ref,
					self.messages.clone(),
					&mut self.map_state,
					update_edit_mode,
				))
				.rect;
			rect.min += egui::Vec2::new(5.0, 5.0);
			ui.put(rect, self.messages.clone());
		});

		// TODO: Use native window
		Window::new("Egui Inpector")
			.open(&mut self.inspection)
			.vscroll(true)
			.show(ctx, |ui| ctx.inspection_ui(ui));

		// TODO use native window with proper positioning.
		/* show_as_window(ctx, "Imagery Settings", &mut self.imagery_settings, |ui| {
			ui.add(ImageriesWidget::new(&self.imageries.lock().unwrap()))
		});*/

		Window::new("Imagery Settings")
			.open(&mut self.imagery_settings)
			.resizable(true)
			.show(ctx, |ui| {
				ui.add(ImageriesWidget::new(
					self.imageries.clone(),
					&self.messages,
					#[cfg(not(target_arch = "wasm32"))]
					&self.project_dirs,
					&mut self.layers,
				))
			});

		ctx.input(|i| {
			if i.modifiers == Modifiers::CTRL | Modifiers::COMMAND {
				let change_result = if i.key_pressed(egui::Key::Z) {
					self.map_data.undo().or_else(|| {
						self.messages.add("Nothing to undo.");
						None
					})
				} else if i.key_pressed(egui::Key::Y) {
					self.map_data.redo().or_else(|| {
						self.messages.add("Nothing to redo.");
						None
					})
				} else {
					None
				};

				if let Some(change_result) = change_result {
					match change_result {
						Ok(ids) => {
							self.map_memory.selection.elements.clear();
							for id in ids {
								self.map_memory.selection.elements.insert(id);
							}
						}
						Err(_) => {
							unreachable_log!("Undo should not error.");
						}
					}
				}
			}
		});

		#[allow(clippy::cast_precision_loss)]
		{
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
			dbgtxt!(
				self.debug_text,
				"Frame Time: {:.1}ms\nAverage: {:.1}ms ({:.1} FPS)\nWorst: {:.1}ms",
				last_frame_ms,
				average_ms,
				1000.0 / average_ms,
				worst_ms
			);
		}
		puffin::GlobalProfiler::lock().new_frame();
		self.last_update = Instant::now();
	}
}
