use std::collections::HashSet;
use std::num::NonZeroUsize;
#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use bytes::Bytes;
use epaint::mutex::RwLock;
use epaint::textures::TextureOptions;
use epaint::{ColorImage, ImageData, TextureId, TextureManager};
#[cfg(target_arch = "wasm32")]
use gloo_worker::{Spawnable, WorkerBridge};
use image::{EncodableLayout, RgbaImage};
use log::error;
use lru::LruCache;
#[cfg(not(target_arch = "wasm32"))]
use tokio::task::JoinSet;
#[cfg(not(target_arch = "wasm32"))]
use tokio::time;

use crate::imagery::cache::Cache;
#[cfg(target_arch = "wasm32")]
use crate::imagery::image_worker::ImageWorker;
#[cfg(not(target_arch = "wasm32"))]
use crate::imagery::local_cache;
#[cfg(target_arch = "wasm32")]
use crate::imagery::web_cache;
use crate::imagery::Imagery;
#[cfg(target_arch = "wasm32")]
use crate::util::sleep::timer;

pub struct ImageryFetcher {
	memory_cache: MemoryCache,
	imagery: Arc<Mutex<dyn Imagery>>,
	pending: Arc<Mutex<HashSet<Tile>>>,
	#[cfg(not(target_arch = "wasm32"))]
	persistent_cache: Arc<tokio::sync::Mutex<local_cache::LocalCache>>,
	#[cfg(target_arch = "wasm32")]
	persistent_cache: Arc<tokio::sync::Mutex<web_cache::WebCache>>,
	last_persistant_cache_count: usize,
	#[cfg(target_arch = "wasm32")]
	worker_bridge: Arc<Mutex<WorkerBridge<ImageWorker>>>,
}

pub type Tile = (u32, u32, u32);

fn tile_to_string(tile: &Tile) -> String {
	format!("tile_{}_{}_{}", tile.0, tile.1, tile.2)
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error("Reqwest error: {0}")]
	Reqwest(#[from] reqwest::Error),
	#[error("Error creating image: {0}")]
	ImageDecode(#[from] image::ImageError),
	#[error("Error reading image data: {0}")]
	ImageIo(#[from] std::io::Error),
}

type MemoryCache = Arc<Mutex<LruCache<Tile, Option<TextureId>>>>;
#[cfg(not(target_arch = "wasm32"))]
type PersistentCache = Arc<tokio::sync::Mutex<local_cache::LocalCache>>;
#[cfg(target_arch = "wasm32")]
type PersistentCache = Arc<tokio::sync::Mutex<web_cache::WebCache>>;

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
			last_persistant_cache_count: 0,
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
		memory_cache: MemoryCache,
	) -> Arc<Mutex<WorkerBridge<ImageWorker>>> {
		// TODO: Replace Arc<Mutex>
		let worker_bridge = Arc::new(Mutex::new(
			ImageWorker::spawner()
				.callback(move |output| {
					let tile = output.0;
					let result = output.1;
					let Some(image_data) = result else {
						error!("Failed to decode image for tile: {:?}", tile);
						return;
					};
					let Some(image) = RgbaImage::from_raw(256, 256, image_data) else {
						error!("Failed to create image for tile: {:?}", tile);
						return;
					};

					let texture =
						Self::create_texture_from_decoded(&texture_manager, tile_to_string(&output.0), &image);
					Self::store_texture(&texture_manager, &memory_cache, tile, texture);
				})
				.spawn("./dummy_worker.js"),
		));
		worker_bridge
	}

	fn store_texture(
		texture_manager: &Arc<RwLock<TextureManager>>,
		memory_cache: &MemoryCache,
		tile: Tile,
		texture: TextureId,
	) {
		if let Some((_, Some(texture_id))) = memory_cache.lock().unwrap().push(tile, Some(texture)) {
			texture_manager.write().free(texture_id);
		}
	}

	#[must_use]
	pub fn imagery(&self) -> Arc<Mutex<dyn Imagery>> {
		self.imagery.clone()
	}

	async fn fetching_loop(
		pending: Arc<Mutex<HashSet<Tile>>>,
		memory_cache: MemoryCache,
		imagery: Arc<Mutex<dyn Imagery>>,
		persistent_cache: PersistentCache,
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
		let mut interval = time::interval(time::Duration::from_millis(FETCH_INTERVAL));

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
			if let Some((_, Some(texture_id))) = memory_cache.lock().unwrap().push(tile, None) {
				texture_manager.write().free(texture_id);
			}
			#[cfg(not(target_arch = "wasm32"))]
			let memory_cache = memory_cache.clone();
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
				let result = Self::fetch_tile(&texture_manager, imagery, persistent_cache.clone(), tile).await;
				#[cfg(target_arch = "wasm32")]
				let result = Self::fetch_tile(worker_bridge, imagery, persistent_cache, tile).await;

				match result {
					#[allow(unused_variables)]
					Ok(texture) => {
						#[cfg(not(target_arch = "wasm32"))]
						Self::store_texture(&texture_manager, &memory_cache, tile, texture);
					}
					Err(error) => {
						error!("Error fetching tile {tile:?}: {error}");
					}
				}
				#[cfg(target_arch = "wasm32")]
				{
					*concurrent_clone.lock().unwrap() -= 1;
				}
			};
			#[cfg(not(target_arch = "wasm32"))]
			join_set.spawn(closure);
			#[cfg(target_arch = "wasm32")]
			{
				wasm_bindgen_futures::spawn_local(closure);
				*concurrent.lock().unwrap() += 1;
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
		let file_format = imagery.lock().unwrap().file_format();
		if let Some(image_data) = persistent_cache.lock().await.get_image(tile, file_format).await {
			return Self::create_texture(texture_manager, tile_to_string(&tile), &image_data);
		}

		let url = imagery.lock().unwrap().tile_url(tile.0, tile.1, tile.2);

		let response = reqwest::Client::new().get(url.clone()).send().await?;
		match response.error_for_status() {
			Err(err) => Err(Error::Reqwest(err)),
			Ok(response) => {
				let bytes = response.bytes().await?;
				persistent_cache
					.lock()
					.await
					.save_image(tile, &bytes, file_format)
					.await; // TODO: Maybe do this on a separate thread

				Self::create_texture(texture_manager, tile_to_string(&tile), &bytes)
			}
		}
	}

	#[cfg(target_arch = "wasm32")]
	async fn fetch_tile(
		worker_bridge: Arc<Mutex<WorkerBridge<ImageWorker>>>,
		imagery: Arc<Mutex<dyn Imagery>>,
		persistent_cache: Arc<tokio::sync::Mutex<web_cache::WebCache>>,
		tile: Tile,
	) -> Result<(), Error> {
		use crate::imagery::image_worker::Input;
		let file_format = imagery.lock().unwrap().file_format();
		if let Some(image_data) = persistent_cache.lock().await.get_image(tile, file_format).await {
			worker_bridge.lock().unwrap().send(Input(tile, image_data.to_vec()));
		}

		let url = imagery.lock().unwrap().tile_url(tile.0, tile.1, tile.2);

		let response = reqwest::Client::new().get(url.clone()).send().await?;

		match response.error_for_status() {
			Err(err) => Err(Error::Reqwest(err)),
			Ok(response) => {
				let bytes = response.bytes().await?;
				// TODO
				// persistent_cache
				// 	.lock()
				// 	.await
				// 	.save_image(tile, &bytes, file_format)
				// 	.await;

				worker_bridge.lock().unwrap().send(Input(tile, bytes.to_vec()));
				Ok(())
			}
		}
	}

	pub fn create_texture(
		texture_manager: &Arc<RwLock<TextureManager>>,
		name: String,
		bytes: &Bytes,
	) -> Result<TextureId, Error> {
		use std::io::Cursor;

		let image = image::ImageReader::new(Cursor::new(bytes))
			.with_guessed_format()?
			.decode()?
			.to_rgba8();

		Ok(Self::create_texture_from_decoded(texture_manager, name, &image))
	}

	fn create_texture_from_decoded(
		texture_manager: &Arc<RwLock<TextureManager>>,
		name: String,
		image: &RgbaImage,
	) -> TextureId {
		let color_image = Arc::new(ColorImage::from_rgba_unmultiplied(
			[image.width() as usize, image.height() as usize],
			image.as_bytes(),
		));
		texture_manager
			.write()
			.alloc(name, ImageData::Color(color_image), TextureOptions::LINEAR)
	}

	pub fn get_tile(&self, x: u32, y: u32, zoom: u32, fetch: bool) -> Option<TextureId> {
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
		let tile_size = self.imagery().lock().unwrap().tile_size().pow(2) as usize;
		self.memory_cached_count() * tile_size * 4 / 1024 / 1024
	}

	pub fn persistent_cached_count(&mut self) -> usize {
		let count = self
			.persistent_cache
			.try_lock()
			.ok()
			.map(|persistent_cache| persistent_cache.cached_count());

		count.map_or(self.last_persistant_cache_count, |count| {
			self.last_persistant_cache_count = count;
			count
		})
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
