use crate::imagery::imagery_fetcher::Tile;
use bytes::Bytes;
use image::ImageFormat;
use js_sys::Reflect;
use log::{error, info};
use std::collections::HashMap;
use std::sync::{Arc};
use tokio::sync::Mutex;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;
use web_sys::IdbFactory;

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

#[derive(Clone)]
pub struct WebCache {
	cached_images: Arc<Mutex<HashMap<Tile, ImageFormat>>>,
	estimate: Arc<Mutex<(f64, f64)>>,
    last_count: usize,
}

impl WebCache {
	pub fn new() -> Self {
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

		let cached_images = Arc::new(Mutex::new(HashMap::new()));

		Self {
			cached_images,
			estimate,
            last_count: 0
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

		*estimate.lock().await = (quota, usage);

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

	pub fn cached_count(&mut self) -> usize {
		self.last_count = self.cached_images.try_lock().map_or(self.last_count, |h| h.len());
		self.last_count
	}

	pub async fn save_image(&mut self, _tile: Tile, _image_data: &Bytes, _image_format: ImageFormat) -> Option<()> {
		let _window = web_sys::window().unwrap();
		None
	}

	pub async fn get_image(&self, _tile: Tile) -> Option<Bytes> {
		None
	}
}
