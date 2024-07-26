use std::cell::RefCell;
use std::collections::HashSet;
use std::num::NonZeroUsize;
use std::ops::{AddAssign, DerefMut, Div, SubAssign};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use bytes::Bytes;
use epaint::mutex::RwLock;
use epaint::textures::TextureOptions;
use epaint::{ColorImage, ImageData, TextureId, TextureManager};
#[cfg(target_arch = "wasm32")]
use gloo_worker::{Spawnable, WorkerBridge};
use image::{EncodableLayout, RgbaImage};
use itertools::Itertools;
use log::{error, info};
use lru::LruCache;
use tokio::task::{JoinHandle, JoinSet, LocalSet};
#[cfg(not(target_arch = "wasm32"))]
use tokio::time;

use crate::cache::{Cache, FileFormat};
#[cfg(target_arch = "wasm32")]
use crate::image_worker::ImageWorker;
use crate::imagery::Imagery;
#[cfg(not(target_arch = "wasm32"))]
use crate::local_cache;
#[cfg(target_arch = "wasm32")]
use crate::sleep::timer;
#[cfg(target_arch = "wasm32")]
use crate::web_cache;

// TODO separate local cache fetching from online fetching
pub struct ImageryFetcher {
	memory_cache: Arc<Mutex<LruCache<Tile, Option<TextureId>>>>,
	imagery: Arc<Mutex<dyn Imagery>>,
	pending: Arc<Mutex<HashSet<Tile>>>,
	#[cfg(not(target_arch = "wasm32"))]
	persistent_cache: Arc<tokio::sync::Mutex<local_cache::LocalCache>>,
	#[cfg(target_arch = "wasm32")]
	persistent_cache: Arc<tokio::sync::Mutex<web_cache::WebCache>>,
	#[cfg(target_arch = "wasm32")]
	worker_bridge: Arc<Mutex<WorkerBridge<ImageWorker>>>,
}

pub type Tile = (u32, u32, u32);
#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error("Reqwest error: {0}")]
	Reqwest(#[from] reqwest::Error),
	#[error("Error creating image: {0}")]
	ImageDecode(#[from] image::ImageError),
	#[error("Error reading image data: {0}")]
	ImageIo(#[from] std::io::Error),
}

impl ImageryFetcher {
	pub fn new(
		imagery: impl Imagery + 'static,
		texture_manager: Arc<RwLock<TextureManager>>,
		#[cfg(not(target_arch = "wasm32"))] cache_directory: PathBuf,
	) -> Self {
		let memory_cache = Arc::new(Mutex::new(LruCache::new(NonZeroUsize::new(4192).unwrap())));

		let fetcher = Self {
			imagery: Arc::new(Mutex::new(imagery)),
			pending: Arc::new(Mutex::new(HashSet::new())),
			#[cfg(not(target_arch = "wasm32"))]
			persistent_cache: Arc::new(tokio::sync::Mutex::new(local_cache::LocalCache::new(cache_directory))),
			#[cfg(target_arch = "wasm32")]
			persistent_cache: Arc::new(tokio::sync::Mutex::new(web_cache::WebCache::new())),
			#[cfg(target_arch = "wasm32")]
			worker_bridge: Self::get_worker_bridge(texture_manager.clone(), memory_cache.clone()),
			memory_cache,
		};

		let pending = fetcher.pending.clone();
		let cache = fetcher.memory_cache.clone();
		let imagery = fetcher.imagery.clone();
		let persistent_cache = fetcher.persistent_cache.clone();
		#[cfg(not(target_arch = "wasm32"))]
		{
			tokio::spawn(Self::fetching_loop(
				pending,
				cache,
				imagery,
				persistent_cache,
				texture_manager,
			));
		}
		#[cfg(target_arch = "wasm32")]
		let worker_bridge = fetcher.worker_bridge.clone();
		#[cfg(target_arch = "wasm32")]
		wasm_bindgen_futures::spawn_local(Self::fetching_loop(
			pending,
			cache,
			imagery,
			persistent_cache,
			texture_manager,
			worker_bridge,
		));

		fetcher
	}

	#[cfg(target_arch = "wasm32")]
	fn get_worker_bridge(
		texture_manager: Arc<RwLock<TextureManager>>,
		memory_cache: Arc<Mutex<LruCache<Tile, Option<TextureId>>>>,
	) -> Arc<Mutex<WorkerBridge<ImageWorker>>> {
		let worker_bridge = Arc::new(Mutex::new(
			ImageWorker::spawner()
				.callback(move |output| {
					let tile = output.0;
					let result = output.1;
					let Some(image_data) = result else {
						error!("Failed to decode image for tile: {:?}", output.0);
						return;
					};
					let Some(image) = RgbaImage::from_raw(256, 256, image_data) else {
						error!("Failed to create image for tile: {:?}", output.0);
						return;
					};

					let result = Self::create_texture_from_decoded(&texture_manager, output.0, image);

					match result {
						Ok(texture) => {
							if let Some((_, Some(texture_id))) = memory_cache.lock().unwrap().push(tile, Some(texture))
							{
								texture_manager.write().free(texture_id);
							}
						}
						Err(error) => {
							error!("Error fetching tile {tile:?}: {error}");
						}
					}
				})
				.spawn("./dummy_worker.js"),
		));
		worker_bridge
	}

	pub fn imagery(&self) -> Arc<Mutex<dyn Imagery>> {
		self.imagery.clone()
	}

	async fn fetching_loop(
		pending: Arc<Mutex<HashSet<Tile>>>,
		cache: Arc<Mutex<LruCache<Tile, Option<TextureId>>>>,
		imagery: Arc<Mutex<dyn Imagery>>,
		#[cfg(not(target_arch = "wasm32"))] persistent_cache: Arc<tokio::sync::Mutex<local_cache::LocalCache>>,
		#[cfg(target_arch = "wasm32")] persistent_cache: Arc<tokio::sync::Mutex<web_cache::WebCache>>,
		texture_manager: Arc<RwLock<TextureManager>>,
		#[cfg(target_arch = "wasm32")] worker_bridge: Arc<Mutex<WorkerBridge<ImageWorker>>>,
	) {
		#[cfg(not(target_arch = "wasm32"))]
		const MAX_CONCURRENT: usize = 16;
		#[cfg(not(target_arch = "wasm32"))]
		const FETCH_INTERVAL: u64 = 5;
		#[cfg(target_arch = "wasm32")]
		const MAX_CONCURRENT: usize = 8;
		#[cfg(target_arch = "wasm32")]
		const FETCH_INTERVAL: u32 = 20;
		#[cfg(not(target_arch = "wasm32"))]
		let mut join_set: JoinSet<()> = JoinSet::new();

		#[cfg(not(target_arch = "wasm32"))]
		let mut interval = time::interval(time::Duration::from_millis(FETCH_INTERVAL as u64));

		#[cfg(target_arch = "wasm32")]
		let concurrent = Arc::new(Mutex::new(0));

		loop {
			#[cfg(not(target_arch = "wasm32"))]
			interval.tick().await;
			#[cfg(target_arch = "wasm32")]
			timer(FETCH_INTERVAL as i32).await.unwrap();

			if pending.lock().unwrap().is_empty() {
				continue;
			}

			#[cfg(not(target_arch = "wasm32"))]
			while join_set.len() >= MAX_CONCURRENT {
				join_set.join_next().await.unwrap().unwrap();
			}

			#[cfg(target_arch = "wasm32")]
			while *concurrent.lock().unwrap() >= MAX_CONCURRENT {
				timer(FETCH_INTERVAL as i32).await.unwrap();
			}

			let mut pending = pending.lock().unwrap();
			let tile = *pending.iter().next().unwrap();
			pending.remove(&tile);
			drop(pending);
			if let Some((_, Some(texture_id))) = cache.lock().unwrap().push(tile, None) {
				texture_manager.write().free(texture_id);
			}
			#[cfg(not(target_arch = "wasm32"))]
			let cache = cache.clone();
			#[cfg(not(target_arch = "wasm32"))]
			let texture_manager = texture_manager.clone();
			let imagery = imagery.clone();
			let persistent_cache = persistent_cache.clone();
			#[cfg(target_arch = "wasm32")]
			let concurrent_clone = concurrent.clone();
			#[cfg(target_arch = "wasm32")]
			let worker_bridge = worker_bridge.clone();
			let closure = async move {
				#[cfg(not(target_arch = "wasm32"))]
				let result = Self::fetch_tile(&texture_manager, imagery, persistent_cache, tile).await;
				#[cfg(target_arch = "wasm32")]
				let result = Self::fetch_tile(worker_bridge, imagery, persistent_cache, tile).await;

				match result {
					Ok(texture) => {
						#[cfg(not(target_arch = "wasm32"))]
						if let Some((_, Some(texture_id))) = cache.lock().unwrap().push(tile, Some(texture)) {
							texture_manager.write().free(texture_id);
						}
					}
					Err(error) => {
						error!("Error fetching tile {tile:?}: {error}");
					}
				}
				#[cfg(target_arch = "wasm32")]
				concurrent_clone.lock().unwrap().sub_assign(1);
			};
			#[cfg(not(target_arch = "wasm32"))]
			join_set.spawn(closure);
			#[cfg(target_arch = "wasm32")]
			{
				wasm_bindgen_futures::spawn_local(closure);
				concurrent.lock().unwrap().add_assign(1);
			}
		}
	}
	#[cfg(not(target_arch = "wasm32"))]
	async fn fetch_tile(
		texture_manager: &Arc<RwLock<TextureManager>>,
		imagery: Arc<Mutex<dyn Imagery>>,
		persistent_cache: Arc<tokio::sync::Mutex<local_cache::LocalCache>>,
		tile: Tile,
	) -> Result<TextureId, Error> {
		if let Some(image_data) = persistent_cache.lock().await.get_image(tile, FileFormat::JPG).await {
			return Self::create_texture(texture_manager, tile, &image_data);
		}

		let url = imagery.lock().unwrap().tile_url(tile.0, tile.1, tile.2);

		let response = reqwest::Client::new().get(url.clone()).send().await?;

		let bytes = response.bytes().await?;
		persistent_cache
			.lock()
			.await
			.save_image(tile, &bytes, FileFormat::JPG)
			.await; // TODO: do this on a separate thread

		Self::create_texture(texture_manager, tile, &bytes)
	}

	#[cfg(target_arch = "wasm32")]
	async fn fetch_tile(
		worker_bridge: Arc<Mutex<WorkerBridge<ImageWorker>>>,
		imagery: Arc<Mutex<dyn Imagery>>,
		persistent_cache: Arc<tokio::sync::Mutex<web_cache::WebCache>>,
		tile: Tile,
	) -> Result<(), Error> {
		use crate::image_worker::Input;
		if let Some(image_data) = persistent_cache.lock().await.get_image(tile, FileFormat::JPG).await {
			worker_bridge.lock().unwrap().send(Input(tile, image_data.to_vec()));
		}

		let url = imagery.lock().unwrap().tile_url(tile.0, tile.1, tile.2);

		let response = reqwest::Client::new().get(url.clone()).send().await?;

		let bytes = response.bytes().await?;
		persistent_cache
			.lock()
			.await
			.save_image(tile, &bytes, FileFormat::JPG)
			.await;

		worker_bridge.lock().unwrap().send(Input(tile, bytes.to_vec()));
		Ok(())
	}

	fn create_texture(
		texture_manager: &Arc<RwLock<TextureManager>>,
		tile: Tile,
		bytes: &Bytes,
	) -> Result<TextureId, Error> {
		use std::io::Cursor;

		let image = image::ImageReader::new(Cursor::new(bytes))
			.with_guessed_format()?
			.decode()?
			.to_rgba8();

		Self::create_texture_from_decoded(texture_manager, tile, image)
	}

	fn create_texture_from_decoded(
		texture_manager: &Arc<RwLock<TextureManager>>,
		tile: Tile,
		image: RgbaImage,
	) -> Result<TextureId, Error> {
		let color_image = Arc::new(ColorImage::from_rgba_unmultiplied(
			[image.width() as usize, image.height() as usize],
			image.as_bytes(),
		));
		let texture_id = texture_manager.write().alloc(
			format!("tile_{}_{}_{}", tile.0, tile.1, tile.2),
			ImageData::Color(color_image),
			TextureOptions::LINEAR,
		);

		Ok(texture_id)
	}

	pub fn get_tile(&mut self, x: u32, y: u32, zoom: u32, fetch: bool) -> Option<TextureId> {
		puffin::profile_function!("get_tile");
		let mut pending = self.pending.lock().unwrap(); // Lock this before memory_cache
		if let Some(texture) = self.memory_cache.lock().unwrap().get(&(x, y, zoom)) {
			return *texture;
		}
		if fetch {
			pending.insert((x, y, zoom));
		}
		None
	}

	pub fn tile_loaded(&self, tile: Tile) -> bool {
		self.memory_cache.lock().unwrap().contains(&tile)
	}

	pub fn pending_count(&self) -> usize {
		self.pending.lock().unwrap().len()
	}

	pub fn memory_cached_count(&self) -> usize {
		self.memory_cache.lock().unwrap().len()
	}

	/// Estimated VRAM usages of tiles in MiB
	pub fn vram_used(&self) -> usize {
		(self.memory_cached_count() * self.imagery().lock().unwrap().tile_size().pow(2) as usize * 4).div(1024 * 1024)
	}

	pub fn persistent_cached_count(&self) -> Option<usize> {
		self.persistent_cache
			.try_lock()
			.ok()
			.map(|persistent_cache| persistent_cache.cached_count())
	}

	pub fn get_closest_tile(&self, tile: Tile) -> Option<Tile> {
		if self.tile_loaded(tile) {
			Some(tile)
		} else {
			self.get_larger_tile(tile)
		}
	}

	fn get_larger_tile(&self, tile: Tile) -> Option<Tile> {
		if tile.2 <= self.imagery.lock().unwrap().min_zoom() {
			None
		} else {
			Some((tile.0 / 2, tile.1 / 2, tile.0 - 1))
		}
	}
}

#[cfg(test)]
mod tests {
	use std::sync::Once;
	use std::time::Duration;

	use log::info;
	use tokio::time::sleep_until;

	use crate::imagery::esri_imagery;

	use super::*;

	static INIT: Once = Once::new();

	#[tokio::test]
	async fn test_fetch_tile() {
		crate::init_test_logger();
		todo!();
		let mut fetcher = ImageryFetcher::new(esri_imagery(), std::path::PathBuf::from("cache"));
		let image = fetcher.get_tile(0, 0, 0);
		assert!(image.is_none());

		loop {
			sleep_until(tokio::time::Instant::now() + Duration::from_secs(1)).await;
			let image = fetcher.get_tile(0, 0, 0);
			if image.is_some() {
				break;
			}
		}
		let image = fetcher.get_tile(0, 0, 0);
		info!("Image: {:?}", image);
		assert!(image.is_some());
	}
}
