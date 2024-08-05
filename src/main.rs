#![warn(clippy::all, rust_2018_idioms)]
// hide console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(not(target_arch = "wasm32"))]
use log::{error, info};

use mappar::MyApp;

// When compiling natively:
#[cfg(not(target_arch = "wasm32"))]
#[tokio::main]
async fn main() -> eframe::Result<()> {
	// simple_logger::init_with_level(log::Level::Info).expect("Error initializing logger");
	let start = std::time::Instant::now();
	init_logger();
	info!("Logger initialized in {:?}", start.elapsed());

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
			.with_inner_size([400.0, 300.0])
			.with_min_inner_size([300.0, 220.0])
			.with_icon(
				// NOTE: Adding an icon is optional
				eframe::icon_data::from_png_bytes(&include_bytes!("../assets/icon-256.png")[..])
					.expect("Failed to load icon"),
			)
			.with_app_id(MyApp::APP_NAME),
		..Default::default()
	};

	eframe::run_native(
		MyApp::APP_NAME,
		native_options,
		Box::new(|cc| Ok(Box::new(MyApp::new(cc)))),
	)
}

#[cfg(target_arch = "wasm32")]
fn main() {
	eframe::WebLogger::init(log::LevelFilter::Info).ok();
	let web_options = eframe::WebOptions::default();

	wasm_bindgen_futures::spawn_local(async {
		let result = eframe::WebRunner::new()
			.start(
				"the_canvas_id", // hardcode it
				web_options,
				Box::new(|cc| Ok(Box::new(MyApp::new(cc)))),
			)
			.await;

		let loading_text = web_sys::window()
			.and_then(|w| w.document())
			.and_then(|d| d.get_element_by_id("loading_text"));

		if let Some(loading_text) = loading_text {
			match result {
				Ok(_) => {
					loading_text.remove();
				}
				Err(e) => {
					loading_text.set_inner_html("<p> The app has crashed. See the developer console for details. </p>");
					error!("Failed to start eframe: {e:?}");
					panic!()
				}
			}
		}
	});
}

#[cfg(not(target_arch = "wasm32"))]
fn init_logger() {
	use chrono::{DateTime, Utc};
	use flate2::write::GzEncoder;
	use simplelog::*;
	use std::fs::File;

	let storage_path = eframe::storage_dir(MyApp::APP_NAME).expect("Failed to get storage dir");
	let storage_path = storage_path.join("logs");
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
			tar::Builder::new(&mut e)
				.append_file(
					new_path.file_name().unwrap(),
					&mut File::open(&new_path).expect("Failed to open log file"),
				)
				.expect("Failed to append file to tar");
		} else {
			eprintln!("Failed to get creation time of latest.log");
			std::fs::remove_file(&latest).expect("Failed to remove old log file");
		}
	}

	CombinedLogger::init(vec![
		TermLogger::new(
			LevelFilter::Info,
			Config::default(),
			TerminalMode::Mixed,
			ColorChoice::Auto,
		),
		WriteLogger::new(
			LevelFilter::Info,
			Config::default(),
			File::create(&latest).expect("Failed to create log file"),
		),
	])
	.expect("Error initializing logger");
}
