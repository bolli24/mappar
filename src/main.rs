#![warn(clippy::all, rust_2018_idioms)]
// hide console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use log::{error, info};
use mappar::MyApp;

// When compiling natively:
#[cfg(not(target_arch = "wasm32"))]
#[tokio::main]
async fn main() -> eframe::Result<()> {
	let mut args = pico_args::Arguments::from_env();
	let no_cache = args.contains("--no-cache");

	let start = web_time::Instant::now();
	init_logger();
	info!("Logger initialized in {:?}", start.elapsed());

	dbg!(eframe::wgpu::Instance::enabled_backend_features());

	let server_addr = format!("127.0.0.1:{}", puffin_http::DEFAULT_PORT);
	match puffin_http::Server::new(&server_addr) {
		Ok(server) => {
			info!("Run this to view profiling data: puffin_viewer {server_addr}");
			puffin::set_scopes_on(true);
			std::mem::forget(server);
		}
		Err(err) => {
			error!("Failed to start puffin server on {server_addr}: {err:?}",);
		}
	}

	let native_options = eframe::NativeOptions {
		viewport: egui::ViewportBuilder::default()
			// .with_inner_size([1920.0, 1080.0])
			.with_min_inner_size([600.0, 400.0])
			.with_icon(
				eframe::icon_data::from_png_bytes(&include_bytes!("../assets/icon-256.png")[..])
					.expect("Failed to load icon"),
			)
			.with_app_id(MyApp::APP_NAME)
			.with_maximized(true),
		wgpu_options: eframe::egui_wgpu::WgpuConfiguration {
			// desired_maximum_frame_latency: Some(1),
			present_mode: eframe::wgpu::PresentMode::AutoNoVsync,
			..Default::default()
		},
		persist_window: true,
		..Default::default()
	};

	eframe::run_native(
		MyApp::APP_NAME,
		native_options,
		Box::new(|cc| Ok(Box::new(MyApp::new(cc, no_cache)))),
	)
}

#[cfg(target_arch = "wasm32")]
fn main() {
	use wasm_bindgen::JsCast;
	eframe::WebLogger::init(log::LevelFilter::Info).expect("Error initializing logger.");
	info!("Logger initialized");
	let web_options = eframe::WebOptions {
		wgpu_options: eframe::egui_wgpu::WgpuConfiguration {
			present_mode: eframe::wgpu::PresentMode::AutoVsync,
			..Default::default()
		},
		..Default::default()
	};

	wasm_bindgen_futures::spawn_local(async {
		let document = web_sys::window().expect("No window").document().expect("No document");

		let canvas = document
			.get_element_by_id("the_canvas_id") // hardcoded canvas id
			.expect("No canvas found")
			.dyn_into::<web_sys::HtmlCanvasElement>()
			.expect("the_canvas_id is not a canvas element");

		let result = eframe::WebRunner::new()
			.start(canvas, web_options, Box::new(|cc| Ok(Box::new(MyApp::new(cc, false)))))
			.await;

		let loading_text = document.get_element_by_id("loading_text");

		match result {
			Ok(_) => {
				if let Some(loading_text) = loading_text {
					loading_text.remove();
				}
			}
			Err(e) => {
				let error_msg = format!("Failed to start eframe: {e:?}");
				web_sys::console::error_1(&error_msg.clone().into());
				error!("{error_msg}");
				if let Some(loading_text) = loading_text {
					loading_text.set_inner_html("<p> The app has crashed. See developer console for details. </p>");
				}
				panic!("{error_msg}");
			}
		}
	});
}

#[cfg(not(target_arch = "wasm32"))]
fn init_logger() {
	use chrono::{DateTime, Utc};
	use flate2::write::GzEncoder;
	use simplelog::ConfigBuilder;
	use simplelog::{ColorChoice, CombinedLogger, LevelFilter, TermLogger, TerminalMode, WriteLogger};
	use std::fs::File;

	let project_dirs = MyApp::project_dirs();
	let storage_path = project_dirs.data_local_dir().join("logs");
	std::fs::create_dir_all(&storage_path).expect("Failed to create log directory");
	let latest = storage_path.join("latest.log");

	if latest.exists() {
		let creation_time = latest.metadata().map(|m| m.created());
		if let Ok(Ok(creation_time)) = creation_time {
			let time: DateTime<Utc> = creation_time.into();
			let time_string = time.format("%Y-%m-%d_%H-%M-%S").to_string();
			let new_path = storage_path.join(format!("{time_string}.log"));
			std::fs::rename(&latest, &new_path).expect("Failed to rename log file");
			let log_gz = storage_path.join(format!("{time_string}.log.gz"));
			let mut e = GzEncoder::new(
				File::create(&log_gz).expect("Failed to create log archive"),
				flate2::Compression::best(),
			);
			let mut builder = tar::Builder::new(&mut e);
			builder
				.append_file(
					new_path.file_name().unwrap(),
					&mut File::open(&new_path).expect("Failed to open log file"),
				)
				.expect("Failed to append file to tar");
			builder.finish().expect("Failed to finish tar archive");
			std::fs::remove_file(&new_path).expect("Failed to remove temporary log file");
		} else {
			eprintln!("Failed to get creation time of latest.log");
			std::fs::remove_file(&latest).expect("Failed to remove old log file");
		}
	}
	let latest_file = File::create(&latest).expect("Failed to create log file");

	#[cfg(target_os = "windows")]
	update_log_time(&latest_file).expect("Failed to update log file time");

	// Remove annoying messages of wgpu
	let config = ConfigBuilder::new().add_filter_ignore_str("wgpu").build();

	let log_level = if cfg!(debug_assertions) {
		LevelFilter::Info
	} else {
		LevelFilter::Warn
	};

	CombinedLogger::init(vec![
		TermLogger::new(log_level, config.clone(), TerminalMode::Mixed, ColorChoice::Auto),
		WriteLogger::new(log_level, config, latest_file),
	])
	.expect("Error initializing logger");
}

#[cfg(target_os = "windows")]
// This is a workaround of file system tunneling on Windows
// Essentially Windows doesn't update the file's creation time when it's overwritten or deleted and quickly recreated
// https://devblogs.microsoft.com/oldnewthing/20050715-14/?p=34923
fn update_log_time(file: &std::fs::File) -> std::io::Result<()> {
	{
		use std::os::windows::fs::FileTimesExt;
		file.set_times(
			std::fs::FileTimes::new()
				.set_modified(file.metadata().map(|m| m.modified())??)
				.set_accessed(file.metadata().map(|m| m.modified())??)
				.set_created(std::time::SystemTime::now()),
		)
	}
}
