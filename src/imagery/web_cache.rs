use crate::imagery::cache::{Cache, FileFormat};
use crate::imagery::imagery_fetcher::Tile;
use bytes::Bytes;
use js_sys::Reflect;
use log::{error, info};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;
use web_sys::IdbFactory;

pub struct WebCache {
	cached_images: HashSet<(Tile, FileFormat)>,
	estimate: Arc<Mutex<(f64, f64)>>,
}

impl WebCache {
	pub fn new() -> Self {
		// let closure = Closure::new(closure);

		let estimate = Arc::new(Mutex::new((0.0, 0.0)));
		let estimate_clone = estimate.clone();
		wasm_bindgen_futures::spawn_local(async move {
			match Self::init_database(estimate_clone).await {
				Ok(_) => {
					info!("IndexedDB initialized");
				}
				Err(e) => {
					error!("Failed to initialize IndexedDB: {}", e);
				}
			}
		});

		let cached_images = HashSet::new();

		Self {
			cached_images,
			estimate,
		}
	}

	async fn init_database(estimate: Arc<Mutex<(f64, f64)>>) -> Result<(), Error> {
		let window = web_sys::window().ok_or(Error::GetWindow)?;
		let navigator = window.navigator();
		let storage = navigator.storage();
		let promise = storage.estimate().map_err(|_| Error::GetStorageEstimate)?;

		let result = JsFuture::from(promise).await.map_err(|_| Error::GetStorageEstimate)?;
		let quota = Reflect::get(&result, &JsValue::from_str("quota"))
			.unwrap()
			.as_f64()
			.unwrap();
		let usage = Reflect::get(&result, &JsValue::from_str("usage"))
			.unwrap()
			.as_f64()
			.unwrap();

		*estimate.lock().unwrap() = (quota, usage);

		info!(
			"Quota: {:.0} MiB, Usage: {:.0} MiB",
			quota / 1024.0 / 1024.0,
			usage / 1024.0 / 1024.0
		);

		let idb_factory: IdbFactory = window
			.indexed_db()
			.map_err(|_| Error::GetIndexedDB)?
			.ok_or(Error::GetIndexedDB)?;
		let open_request = idb_factory.open("cache").map_err(|_| Error::GetOpenRequest)?.result();
		if open_request.is_err() {
			return Err(Error::GetOpenRequest);
		}
		Ok(())
	}
}
#[derive(Debug, thiserror::Error)]
enum Error {
	#[error("Failed to get window object")]
	GetWindow,
	#[error("Failed to get storage estimate")]
	GetStorageEstimate,
	#[error("Failed to get indexed db")]
	GetIndexedDB,
	#[error("Failed to get open request")]
	GetOpenRequest,
}

impl Cache for WebCache {
	fn cached_count(&self) -> usize {
		self.cached_images.len()
	}

	async fn save_image(&mut self, tile: Tile, image_data: &Bytes, file_format: FileFormat) {
		let window = web_sys::window().unwrap();
	}

	async fn get_image(&self, tile: Tile, file_format: FileFormat) -> Option<Bytes> {
		return None;
	}
}
