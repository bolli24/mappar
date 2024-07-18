use bytes::Buf;
use std::collections::HashSet;
use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use log::{error, info};
use lru::LruCache;
use skia_safe::{Data, Image};
use tokio::task::JoinSet;
use tokio::{task, time};

use crate::imagery::Imagery;
use crate::local_cache;
use crate::local_cache::LocalCache;

pub struct ImageryFetcher {
	memory_cache: Arc<Mutex<LruCache<Tile, Option<Image>>>>,
	imagery: Arc<Mutex<dyn Imagery>>,
	pending: Arc<Mutex<HashSet<Tile>>>,
	local_cache: Arc<tokio::sync::Mutex<LocalCache>>,
}

pub type Tile = (u32, u32, u32);
#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error("Reqwest error: {0}")]
	Reqwest(#[from] reqwest::Error),
	#[error("Error creating image data")]
	SkiaImageData,
	#[error("Error creating image")]
	SkiaImage,
}

impl ImageryFetcher {
	pub fn new(imagery: impl Imagery + 'static, cache_directory: PathBuf) -> Self {
		let fetcher = Self {
			memory_cache: Arc::new(Mutex::new(LruCache::new(NonZeroUsize::new(4192).unwrap()))),
			imagery: Arc::new(Mutex::new(imagery)),
			pending: Arc::new(Mutex::new(HashSet::new())),
			local_cache: Arc::new(tokio::sync::Mutex::new(LocalCache::new(cache_directory))),
		};

		let pending = fetcher.pending.clone();
		let cache = fetcher.memory_cache.clone();
		let imagery = fetcher.imagery.clone();
		let local_cache = fetcher.local_cache.clone();

		tokio::spawn(Self::fetching_loop(pending, cache, imagery, local_cache));

		fetcher
	}

	pub fn imagery(&self) -> Arc<Mutex<dyn Imagery>> {
		self.imagery.clone()
	}

	async fn fetching_loop(
		pending: Arc<Mutex<HashSet<Tile>>>,
		cache: Arc<Mutex<LruCache<Tile, Option<Image>>>>,
		imagery: Arc<Mutex<dyn Imagery>>,
		local_cache: Arc<tokio::sync::Mutex<LocalCache>>,
	) {
		let mut join_set: JoinSet<()> = JoinSet::new();
		let mut interval = time::interval(Duration::from_millis(10));
		const MAX_CONCURRENT: usize = 8;
		loop {
			interval.tick().await;
			if pending.lock().unwrap().is_empty() {
				continue;
			}

			while join_set.len() >= MAX_CONCURRENT {
				join_set.join_next().await.unwrap().unwrap();
			}
			let mut pending = pending.lock().unwrap();
			let tile = *pending.iter().next().unwrap();
			pending.remove(&tile);
			cache.lock().unwrap().put(tile, None);
			let cache = cache.clone();
			let imagery = imagery.clone();
			let local_cache = local_cache.clone();
			join_set.spawn(async move {
				let result = Self::fetch_tile(imagery, local_cache, tile).await;
				match result {
					Ok(image) => {
						cache.lock().unwrap().put(tile, Some(image));
					}
					Err(error) => {
						error!("Error fetching tile {tile:?}: {error}");
					}
				}
			});
		}
	}

	async fn fetch_tile(
		imagery: Arc<Mutex<dyn Imagery>>,
		local_cache: Arc<tokio::sync::Mutex<LocalCache>>,
		tile: Tile,
	) -> Result<Image, Error> {
		// dummy implementation
		/*
		sleep(Duration::from_millis(160)).await;
		const TEST_TILE: &[u8] = include_bytes!("../assets/tile.png");
		let data = Data::from_stream(&mut TEST_TILE, TEST_TILE.len()).ok_or(Error::SkiaImageData)?;
		return Image::from_encoded(data).ok_or(Error::SkiaImage);
		*/
		if let Some(image_data) = local_cache
			.lock()
			.await
			.get_image(tile, local_cache::FileFormat::JPG)
			.await
		{
			let data = Data::from_stream(&mut image_data.as_ref(), image_data.len()).ok_or(Error::SkiaImageData)?;
			return Image::from_encoded(data).ok_or(Error::SkiaImage);
		}

		let url = imagery.lock().unwrap().tile_url(tile.0, tile.1, tile.2);
		info!("Fetching tile: {url}");

		let response = reqwest::Client::new().get(url.clone()).send().await?;

		let bytes = response.bytes().await?;
		local_cache
			.lock()
			.await
			.save_image(tile, &bytes, local_cache::FileFormat::JPG)
			.await; // TODO: do this on a separate thread

		let data = Data::from_stream(&mut bytes.as_ref(), bytes.len()).ok_or(Error::SkiaImageData)?;
		Image::from_encoded(data).ok_or(Error::SkiaImage)
	}

	pub fn get_tile(&mut self, x: u32, y: u32, zoom: u32) -> Option<Image> {
		let mut pending = self.pending.lock().unwrap();
		if let Some(image) = self.memory_cache.lock().unwrap().get(&(x, y, zoom)) {
			return image.clone();
		}
		pending.insert((x, y, zoom));
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

	pub fn local_cached_count(&self) -> Option<usize> {
		self.local_cache
			.try_lock()
			.ok()
			.map(|local_cache| local_cache.cached_count())
	}

	pub fn get_closest_tile(&self, tile: Tile) -> Option<Tile> {
		if self.tile_loaded(tile) {
			Some(tile)
		} else {
			self.get_larger_tile(tile)
		}
	}

	fn get_larger_tile(&self, tile: Tile) -> Option<Tile> {
		if tile.2 == 0 {
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
