use std::collections::HashSet;
use std::io::Cursor;
use std::num::NonZeroUsize;
#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use bytes::Bytes;
use derive_new::new;
use epaint::mutex::RwLock;
use epaint::textures::TextureOptions;
use epaint::{ColorImage, ImageData, TextureId, TextureManager};
#[cfg(target_arch = "wasm32")]
use gloo_worker::{Spawnable, WorkerBridge};
use image::{EncodableLayout, ImageFormat};
use log::error;
use lru::LruCache;
use reqwest::Client;
use reqwest::ClientBuilder;
#[cfg(not(target_arch = "wasm32"))]
use tokio::task::JoinSet;
#[cfg(not(target_arch = "wasm32"))]
use tokio::time;

#[cfg(target_arch = "wasm32")]
use crate::imagery::image_worker::ImageWorker;
#[cfg(not(target_arch = "wasm32"))]
use crate::imagery::local_cache;
#[cfg(target_arch = "wasm32")]
use crate::imagery::web_cache;
use crate::ui::messages::Messages;
#[cfg(target_arch = "wasm32")]
use crate::util::sleep::timer;

use super::{Imageries, ImageryRef};

#[cfg(not(target_arch = "wasm32"))]
type PersistentCache = local_cache::LocalCache;
#[cfg(target_arch = "wasm32")]
type PersistentCache = web_cache::WebCache;

pub struct ImageryFetcher {
	memory_cache: MemoryCache,
	texture_manager: Arc<RwLock<TextureManager>>,
	imageries: Arc<Mutex<Imageries>>,
	pending: Arc<Mutex<HashSet<Tile>>>,
	persistent_cache: PersistentCache,
	#[cfg(target_arch = "wasm32")]
	worker_bridge: Arc<Mutex<WorkerBridge<ImageWorker>>>,
}

#[derive(PartialEq, Eq, Hash, Debug, Copy, Clone, new)]
pub struct Tile {
	pub imagery_ref: ImageryRef,
	// (x, y, zoom)
	pub coords: (u32, u32, u32),
}

fn tile_to_string(tile: &Tile) -> String {
	format!(
		"tile_{:?}_{}_{}_{}",
		tile.imagery_ref.0, tile.coords.0, tile.coords.1, tile.coords.2
	)
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

impl ImageryFetcher {
	pub fn new(
		imageries: Arc<Mutex<Imageries>>,
		texture_manager: Arc<RwLock<TextureManager>>,
		#[cfg(not(target_arch = "wasm32"))] cache_base_directory: PathBuf,
		messages: Messages,
	) -> Self {
		let memory_cache = Arc::new(Mutex::new(LruCache::new(NonZeroUsize::new(4192).unwrap())));
		// let cache_directory = cache_base_directory.join(&imagery.id.0);

		let fetcher = Self {
			pending: Arc::new(Mutex::new(HashSet::new())),
			#[cfg(not(target_arch = "wasm32"))]
			persistent_cache: local_cache::LocalCache::new(cache_base_directory, messages, imageries.clone()),
			#[cfg(target_arch = "wasm32")]
			persistent_cache: web_cache::WebCache::new(),
			#[cfg(target_arch = "wasm32")]
			worker_bridge: Self::get_worker_bridge(texture_manager.clone(), memory_cache.clone()),
			imageries,
			memory_cache,
			texture_manager: texture_manager.clone(),
		};

		let pending = fetcher.pending.clone();
		let cache = fetcher.memory_cache.clone();
		let imageries = fetcher.imageries.clone();
		let persistent_cache = fetcher.persistent_cache.clone();
		#[cfg(not(target_arch = "wasm32"))]
		{
			tokio::spawn(Self::fetching_loop(
				pending,
				cache,
				imageries,
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
					let Some(image) = image::RgbaImage::from_raw(256, 256, image_data) else {
						error!("Failed to create image for tile: {:?}", tile);
						return;
					};

					let image = rgba_image_to_color_image(&image);

					let texture = Self::create_texture_from_decoded(&texture_manager, tile_to_string(&output.0), image);
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
	pub fn imageries(&self) -> Arc<Mutex<Imageries>> {
		self.imageries.clone()
	}

	async fn fetching_loop(
		pending: Arc<Mutex<HashSet<Tile>>>,
		memory_cache: MemoryCache,
		imageries: Arc<Mutex<Imageries>>,
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
			let tile = pending.iter().next().unwrap().clone();
			pending.remove(&tile);
			drop(pending);

			if let Some((_, Some(texture_id))) = memory_cache.lock().unwrap().push(tile.clone(), None) {
				texture_manager.write().free(texture_id);
			}
			#[cfg(not(target_arch = "wasm32"))]
			let memory_cache = memory_cache.clone();
			#[cfg(not(target_arch = "wasm32"))]
			let texture_manager = texture_manager.clone();
			let imageries = imageries.clone();
			let persistent_cache = persistent_cache.clone();
			#[cfg(target_arch = "wasm32")]
			let concurrent_clone = concurrent.clone();
			#[cfg(target_arch = "wasm32")]
			let worker_bridge = worker_bridge.clone();
			#[cfg(not(target_arch = "wasm32"))]
			let client = ClientBuilder::new()
				.user_agent(crate::map::api::USER_AGENT)
				.build()
				.unwrap();
			// Don't add user-agent on browser because of cors preflight shenanigans:
			// https://developer.mozilla.org/en-US/docs/Web/HTTP/CORS#examples_of_access_control_scenarios
			#[cfg(target_arch = "wasm32")]
			let client = ClientBuilder::new().build().unwrap();

			let closure = async move {
				#[cfg(not(target_arch = "wasm32"))]
				let result = Self::fetch_tile(
					&texture_manager,
					imageries,
					persistent_cache.clone(),
					&tile,
					client.clone(),
				)
				.await;
				#[cfg(target_arch = "wasm32")]
				let result = Self::fetch_tile(worker_bridge, imagery, persistent_cache, tile, client.clone()).await;

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
		imageries: Arc<Mutex<Imageries>>,
		mut persistent_cache: local_cache::LocalCache,
		tile: &Tile,
		client: Client,
	) -> Result<TextureId, Error> {
		if let Some(image_data) = persistent_cache.get_image(tile).await {
			return Self::create_texture(texture_manager, tile_to_string(tile), &image_data);
		}

		let url = imageries
			.lock()
			.unwrap()
			.get(tile.imagery_ref)
			.expect("Imagery reference should be valid.")
			.tile_url(tile.coords.0, tile.coords.1, tile.coords.2);

		let response = client.get(url.clone()).send().await?;
		match response.error_for_status() {
			Err(err) => Err(Error::Reqwest(err)),
			Ok(response) => {
				let bytes = response.bytes().await?;
				let image_format = image::guess_format(&bytes)?;
				persistent_cache.save_image(tile, &bytes, image_format).await; // TODO: Maybe do this on a separate thread
				Self::create_texture(texture_manager, tile_to_string(tile), &bytes)
			}
		}
	}

	#[cfg(target_arch = "wasm32")]
	async fn fetch_tile(
		worker_bridge: Arc<Mutex<WorkerBridge<ImageWorker>>>,
		imagery: Arc<Mutex<Imagery>>,
		persistent_cache: web_cache::WebCache,
		tile: Tile,
		client: Client,
	) -> Result<(), Error> {
		use crate::imagery::image_worker::Input;
		//	let file_format = imagery.lock().unwrap().file_format();
		if let Some(image_data) = persistent_cache.get_image(tile).await {
			worker_bridge.lock().unwrap().send(Input(tile, image_data.to_vec()));
		}

		let url = imagery.lock().unwrap().tile_url(tile.0, tile.1, tile.2);

		let response = client.get(url.clone()).send().await?;

		match response.error_for_status() {
			Err(err) => Err(Error::Reqwest(err)),
			Ok(response) => {
				let bytes = response.bytes().await?;
				// TODO:
				// persistent_cache
				// 	.save_image(tile, &bytes, file_format)
				// 	.await;

				worker_bridge.lock().unwrap().send(Input(tile, bytes.to_vec()));
				Ok(())
			}
		}
	}
	pub fn decode_image(bytes: &[u8]) -> Result<(ColorImage, ImageFormat), Error> {
		let image_format = image::guess_format(bytes)?;
		let image = image::ImageReader::with_format(Cursor::new(bytes), image_format)
			.decode()?
			.to_rgba8();
		let image = rgba_image_to_color_image(&image);
		Ok((image, image_format))
	}

	pub fn create_texture(
		texture_manager: &Arc<RwLock<TextureManager>>,
		name: String,
		bytes: &Bytes,
	) -> Result<TextureId, Error> {
		Ok(Self::create_texture_from_decoded(
			texture_manager,
			name,
			Self::decode_image(bytes)?.0,
		))
	}

	pub fn create_texture_from_decoded(
		texture_manager: &Arc<RwLock<TextureManager>>,
		name: String,
		image: ColorImage,
	) -> TextureId {
		texture_manager
			.write()
			.alloc(name, ImageData::Color(Arc::new(image)), TextureOptions::LINEAR)
	}

	pub fn get_tile(&self, tile: &Tile, fetch: bool) -> Option<TextureId> {
		puffin::profile_function!("get_tile");
		let mut pending = self.pending.lock().unwrap(); // Lock this before memory_cache
		if let Some(texture) = self.memory_cache.lock().unwrap().get(&tile) {
			return *texture;
		}
		if fetch {
			pending.insert(tile.clone());
		}
		None
	}

	pub fn tile_loaded(&self, tile: &Tile) -> bool {
		self.memory_cache.lock().unwrap().contains(tile)
	}

	pub fn pending_count(&self) -> usize {
		self.pending.lock().unwrap().len()
	}

	pub fn memory_cached_count(&self) -> usize {
		self.memory_cache.lock().unwrap().len()
	}

	/// Estimated VRAM usages of tiles in MiB
	pub fn vram_used(&self) -> usize {
		self.texture_manager
			.read()
			.allocated()
			.fold(0, |sum, (_, meta)| sum + meta.bytes_used())
			/ 1024 / 1024
	}

	pub fn persistent_cached_count(&mut self) -> usize {
		self.persistent_cache.cached_count()
	}

	// TODO:

	/* pub fn get_closest_tile(&self, tile: Tile) -> Option<Tile> {
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
	*/
}

fn rgba_image_to_color_image(image: &image::ImageBuffer<image::Rgba<u8>, Vec<u8>>) -> ColorImage {
	ColorImage::from_rgba_unmultiplied([image.width() as usize, image.height() as usize], image.as_bytes())
}
