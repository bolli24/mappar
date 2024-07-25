use crate::cache::{Cache, FileFormat};
use crate::imagery_fetcher::Tile;
use bytes::Bytes;
use js_sys::{Promise, Reflect};
use log::info;
use std::collections::HashSet;
use std::result;
use tokio::task::spawn_blocking;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen::__rt::IntoJsResult;
use wasm_bindgen::closure::Closure;
use wasm_bindgen_futures::JsFuture;
use web_sys::StorageManager;

pub struct WebCache {
	storage: StorageManager,
	cached_images: HashSet<(Tile, FileFormat)>,
}

pub struct Estimate {
	pub usage: f64,
	pub quota: f64,
}

impl JsCast for Estimate {
	fn instanceof(val: &JsValue) -> bool {
		Reflect::has(val, "usage") && Reflect::has(val, &JsValue::from_str("quota"))
	}

	fn unchecked_from_js(val: JsValue) -> Self {
		todo!()
	}

	fn unchecked_from_js_ref(val: &JsValue) -> &Self {
		todo!()
	}
}

impl WebCache {
	pub fn new() -> Self {
		let window = web_sys::window().unwrap();
		let navigator = window.navigator();
		let storage = navigator.storage();
		// let closure = Closure::new(closure);
		let promise = storage
			.estimate()
			.expect("Failed to get storage estimate promise");

		wasm_bindgen_futures::spawn_local(async {
			let result = JsFuture::from(promise).await;
			let estimate = result.unwrap().dyn_into::<Estimate>().expect("Failed to convert storage estimate result")
		});

		let cached_images = HashSet::new();

		Self { storage, cached_images }
	}
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
