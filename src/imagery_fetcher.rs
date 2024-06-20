use std::collections::HashSet;
use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use log::{error, info};
use lru::LruCache;
use skia_safe::{AlphaType, Bitmap, ColorSpace, ColorType, Data, Image, ImageInfo, ISize};
use skia_safe::images::raster_from_data;
use tokio::{runtime, spawn, time};
use tokio::runtime::Runtime;
use tokio::task::JoinSet;
use tokio::time::sleep;

use crate::imagery::Imagery;

pub struct ImageryFetcher {
	cache: Arc<Mutex<LruCache<Tile, Option<Image>>>>,
	imagery: Arc<Mutex<dyn Imagery>>,
	pending: Arc<Mutex<HashSet<Tile>>>,
}

pub type Tile = (u32, u32, u32);

pub enum Error {
	Reqwest(reqwest::Error),
	SkiaImageData,
	SkiaImage,
}

impl std::fmt::Display for Error {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Error::Reqwest(e) => write!(f, "Reqwest error: {e}"),
			Error::SkiaImageData => write!(f, "Skia error creating image data"),
			Error::SkiaImage => write!(f, "Skia error creating image"),
		}
	}
}

impl ImageryFetcher {
	pub fn new(imagery: impl Imagery + 'static) -> Self {
		let fetcher = Self {
			cache: Arc::new(Mutex::new(LruCache::new(NonZeroUsize::new(4192).unwrap()))),
			imagery: Arc::new(Mutex::new(imagery)),
			pending: Arc::new(Mutex::new(HashSet::new())),
		};

		let pending = fetcher.pending.clone();
		let cache = fetcher.cache.clone();
		let imagery = fetcher.imagery.clone();

		spawn(Self::fetching_loop(pending, cache, imagery));

		fetcher
	}

	pub fn imagery(&self) -> Arc<Mutex<dyn Imagery>> {
		self.imagery.clone()
	}

	async fn fetching_loop(
		pending: Arc<Mutex<HashSet<Tile>>>,
		cache: Arc<Mutex<LruCache<Tile, Option<Image>>>>,
		imagery: Arc<Mutex<dyn Imagery>>,
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
			info!("Pending: {}, Cache {}", pending.len(), cache.lock().unwrap().len());
			let tile = *pending.iter().next().unwrap();
			pending.remove(&tile);
			cache.lock().unwrap().put(tile, None);
			let cache = cache.clone();
			let imagery = imagery.clone();
			join_set.spawn(async move {
				info!("Fetching tile: {tile:?}.");
				let result = Self::fetch_tile(imagery, tile).await;
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

	async fn fetch_tile(imagery: Arc<Mutex<dyn Imagery>>, tile: Tile) -> Result<Image, Error> {
		// dummy implementation
		/* sleep(Duration::from_millis(160)).await;
		const TEST_TILE: &[u8] = include_bytes!("../assets/tile.png");
		let data = Data::from_stream(&mut TEST_TILE, TEST_TILE.len()).ok_or(Error::SkiaImageData)?;
		return Image::from_encoded(data).ok_or(Error::SkiaImage);*/

		let url = imagery.lock().unwrap().tile_url(tile.0, tile.1, tile.2);
		info!("Fetching tile: {url}");

		let response = reqwest::Client::new()
			.get(url.clone())
			.send()
			.await
			.map_err(Error::Reqwest)?;

		let bytes = response.bytes().await.map_err(Error::Reqwest)?;

		let data = Data::from_stream(&mut bytes.as_ref(), bytes.len()).ok_or(Error::SkiaImageData)?;
		Image::from_encoded(data).ok_or(Error::SkiaImage)
	}

	pub fn get_tile(&mut self, x: u32, y: u32, zoom: u32) -> Option<Image> {
		let mut pending = self.pending.lock().unwrap();
		if let Some(image) = self.cache.lock().unwrap().get(&(x, y, zoom)) {
			return image.clone();
		}
		pending.insert((x, y, zoom));
		None
	}

	pub fn tile_loaded(&self, tile: Tile) -> bool {
		self.cache.lock().unwrap().contains(&tile)
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

		let mut fetcher = ImageryFetcher::new(esri_imagery());
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
