use futures::StreamExt;
use futures::channel::mpsc::{Receiver, Sender, channel};
use futures::future::{Either, select};
use futures::stream::FuturesUnordered;
use std::io::Cursor;
use std::num::NonZeroUsize;
#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
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

#[cfg(target_arch = "wasm32")]
use crate::imagery::image_worker::ImageWorker;
#[cfg(not(target_arch = "wasm32"))]
use crate::imagery::local_cache;
#[cfg(target_arch = "wasm32")]
use crate::imagery::web_cache;
use crate::ui::messages::Messages;

use super::{Imageries, ImageryRef};

#[cfg(not(target_arch = "wasm32"))]
type PersistentCache = local_cache::LocalCache;
#[cfg(target_arch = "wasm32")]
type PersistentCache = web_cache::WebCache;

pub struct ImageryFetcher {
	memory_cache: MemoryCache,
	texture_manager: Arc<RwLock<TextureManager>>,
	imageries: Arc<Mutex<Imageries>>,
	pending: Arc<AtomicU32>,
	persistent_cache: PersistentCache,
	request_tx: Sender<Tile>,
	exit: Receiver<()>,
}

#[derive(PartialEq, Eq, Hash, Debug, Copy, Clone, new, serde::Serialize, serde::Deserialize)]
pub struct Tile {
	pub imagery_ref: ImageryRef,
	// (x, y, zoom)
	pub coords: (u32, u32, u32),
}

impl Tile {
	pub fn get_larger(&self, min_zoom: u32) -> Option<Tile> {
		if self.coords.2 <= min_zoom {
			None
		} else {
			Some(Tile::new(
				self.imagery_ref,
				(self.coords.0 / 2, self.coords.1 / 2, self.coords.2 - 1),
			))
		}
	}

	pub fn get_smaller(&self, max_zoom: u32) -> Option<[Tile; 4]> {
		if self.coords.2 >= max_zoom {
			None
		} else {
			let zoom = self.coords.2 + 1;
			Some([
				Tile::new(self.imagery_ref, (self.coords.0 * 2, self.coords.1 * 2, zoom)),
				Tile::new(self.imagery_ref, (self.coords.0 * 2 + 1, self.coords.1 * 2, zoom)),
				Tile::new(self.imagery_ref, (self.coords.0 * 2, self.coords.1 * 2 + 1, zoom)),
				Tile::new(self.imagery_ref, (self.coords.0 * 2 + 1, self.coords.1 * 2 + 1, zoom)),
			])
		}
	}
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

#[cfg(not(target_arch = "wasm32"))]
const MAX_CONCURRENT: usize = 16;
#[cfg(target_arch = "wasm32")]
const MAX_CONCURRENT: usize = 6;

impl ImageryFetcher {
	pub fn new(
		imageries: Arc<Mutex<Imageries>>,
		texture_manager: Arc<RwLock<TextureManager>>,
		#[cfg(not(target_arch = "wasm32"))] cache_base_directory: PathBuf,
		messages: Messages,
	) -> Self {
		let memory_cache = Arc::new(Mutex::new(LruCache::new(NonZeroUsize::new(4192).unwrap())));

		let (request_tx, request_rx) = channel(MAX_CONCURRENT);
		let (exit_tx, exit_rx) = channel(0);

		let fetcher = Self {
			#[cfg(not(target_arch = "wasm32"))]
			persistent_cache: local_cache::LocalCache::new(cache_base_directory, messages, imageries.clone()),
			#[cfg(target_arch = "wasm32")]
			persistent_cache: web_cache::WebCache::new(),
			imageries,
			pending: Arc::new(AtomicU32::new(0)),
			memory_cache,
			texture_manager: texture_manager.clone(),
			request_tx,
			exit: exit_rx,
		};

		crate::spawn_task!(Self::fetching_loop(
			request_rx,
			fetcher.memory_cache.clone(),
			fetcher.imageries.clone(),
			fetcher.pending.clone(),
			fetcher.persistent_cache.clone(),
			texture_manager,
			exit_tx,
		));

		fetcher
	}

	#[cfg(target_arch = "wasm32")]
	fn get_worker_bridge(
		texture_manager: Arc<RwLock<TextureManager>>,
		memory_cache: MemoryCache,
	) -> WorkerBridge<ImageWorker> {
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
			.spawn("./dummy_worker.js")
	}

	pub fn update(&mut self) {}

	fn store_texture(
		texture_manager: &Arc<RwLock<TextureManager>>,
		memory_cache: &MemoryCache,
		tile: Tile,
		texture: TextureId,
	) {
		if let Some((tile_b, Some(texture_id))) = memory_cache.lock().unwrap().push(tile, Some(texture)) {
			debug_assert_ne!(tile, tile_b, "Mustn't store the same tile twice.");
			texture_manager.write().free(texture_id);
		}
	}

	#[must_use]
	pub fn imageries(&self) -> Arc<Mutex<Imageries>> {
		self.imageries.clone()
	}

	async fn fetching_loop(
		mut request_rx: Receiver<Tile>,
		memory_cache: MemoryCache,
		imageries: Arc<Mutex<Imageries>>,
		pending: Arc<AtomicU32>,
		persistent_cache: PersistentCache,
		texture_manager: Arc<RwLock<TextureManager>>,
		exit: Sender<()>,
	) {
		let mut join_set/*: FuturesUnordered<Pin<Box<dyn Future<Output = ()>>>> */ = FuturesUnordered::new();

		#[cfg(target_arch = "wasm32")]
		let worker_bridge = Arc::new(Mutex::new(Self::get_worker_bridge(
			texture_manager.clone(),
			memory_cache.clone(),
		)));

		loop {
			if exit.is_closed() {
				break;
			}

			let tile = if join_set.len() >= MAX_CONCURRENT {
				join_set.next().await;
				continue;
			} else if join_set.is_empty() {
				let Some(tile) = request_rx.next().await else {
					continue;
				};
				tile
			} else {
				match select(request_rx.next(), join_set.next()).await {
					Either::Left((Some(tile), _)) => tile,
					_ => continue,
				}
			};

			let mut cache = memory_cache.lock().unwrap();
			if cache.get(&tile).is_some() {
				continue;
			}

			pending.fetch_add(1, Ordering::Relaxed);

			if let Some((evicted_tile, Some(texture_id))) = cache.push(tile, None) {
				if tile != evicted_tile {
					texture_manager.write().free(texture_id);
				}
			}

			drop(cache);

			#[cfg(not(target_arch = "wasm32"))]
			let memory_cache = memory_cache.clone();
			#[cfg(not(target_arch = "wasm32"))]
			let texture_manager = texture_manager.clone();
			let imageries = imageries.clone();
			let pending = pending.clone();
			let persistent_cache = persistent_cache.clone();
			#[cfg(target_arch = "wasm32")]
			let worker_bridge = worker_bridge.clone();
			let client_builder = ClientBuilder::new();
			#[cfg(not(target_arch = "wasm32"))]
			let client_builder = client_builder.user_agent(crate::map::api::USER_AGENT);
			// Don't add user-agent on browser because of cors preflight shenanigans:
			// https://developer.mozilla.org/en-US/docs/Web/HTTP/CORS#examples_of_access_control_scenarios
			let client = client_builder.build().unwrap();

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
				let result = Self::fetch_tile(worker_bridge, imageries, persistent_cache, tile, client.clone()).await;

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
				pending.fetch_sub(1, Ordering::Relaxed);
				#[cfg(target_arch = "wasm32")]
				{
					// *concurrent_clone.lock().unwrap() -= 1;
				}
			};
			// #[cfg(not(target_arch = "wasm32"))]
			join_set.push(closure);
			#[cfg(target_arch = "wasm32")]
			{
				//	wasm_bindgen_futures::spawn_local(closure);
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
		imageries: Arc<Mutex<Imageries>>,
		persistent_cache: web_cache::WebCache,
		tile: Tile,
		client: Client,
	) -> Result<(), Error> {
		use crate::imagery::image_worker::Input;
		//	let file_format = imagery.lock().unwrap().file_format();
		if let Some(image_data) = persistent_cache.get_image(tile).await {
			worker_bridge.lock().unwrap().send(Input(tile, image_data.to_vec()));
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
				// TODO:
				// persistent_cache
				// 	.save_image(tile, &bytes, file_format)
				// 	.await;
				//
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

	// TODO: allow fetching of local tiles.
	pub fn get_tile(&mut self, tile: Tile, fetch: bool) -> Option<TextureId> {
		puffin::profile_function!("get_tile");
		if let Some(texture) = self.memory_cache.lock().unwrap().get(&tile) {
			return *texture;
		}
		if fetch {
			match self.request_tx.try_send(tile) {
				Err(err) if err.is_disconnected() => {
					error!("Tile fetching task disconnected.");
				}
				_ => (),
			}
		}
		None
	}

	pub fn tile_loaded(&self, tile: &Tile) -> bool {
		self.memory_cache.lock().unwrap().contains(tile)
	}

	pub fn pending_count(&self) -> u32 {
		self.pending.load(Ordering::Relaxed)
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
	}*/
}

fn rgba_image_to_color_image(image: &image::ImageBuffer<image::Rgba<u8>, Vec<u8>>) -> ColorImage {
	ColorImage::from_rgba_unmultiplied([image.width() as usize, image.height() as usize], image.as_bytes())
}
