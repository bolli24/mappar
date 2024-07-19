#![warn(clippy::all, rust_2018_idioms)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use log::Level;

use mappar::MyApp;

// When compiling natively:
#[cfg(not(target_arch = "wasm32"))]
#[tokio::main]
async fn main() -> eframe::Result<()> {
	simple_logger::init_with_level(Level::Info).expect("Error initializing logger");

	let server_addr = format!("127.0.0.1:{}", puffin_http::DEFAULT_PORT);
	let _puffin_server = puffin_http::Server::new(&server_addr).unwrap();
	eprintln!("Run this to view profiling data:  puffin_viewer {server_addr}");
	puffin::set_scopes_on(true);
	
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

	eframe::run_native(MyApp::APP_NAME, native_options, Box::new(|cc| Ok(Box::new(MyApp::new(cc)))))
}

// When compiling to web using trunk:
#[cfg(target_arch = "wasm32")]
fn main() {
	// Redirect `log` message to `console.log` and friends:
	eframe::WebLogger::init(log::LevelFilter::Debug).ok();

	let web_options = eframe::WebOptions::default();

	wasm_bindgen_futures::spawn_local(async {
		eframe::WebRunner::new()
			.start(
				"the_canvas_id", // hardcode it
				web_options,
				Box::new(|cc| Box::new(eframe_template::TemplateApp::new(cc))),
			)
			.await
			.expect("failed to start eframe");
	});
}
