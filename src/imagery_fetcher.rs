use std::collections::HashSet;
use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use log::{error, info};
use lru::LruCache;
use skia_safe::{Data, Image};
use tokio::{runtime, spawn, time};
use tokio::runtime::Runtime;
use tokio::task::JoinSet;

use crate::imagery::Imagery;

pub struct ImageryFetcher {
	cache: Arc<Mutex<LruCache<(u32, u32, u32), Image>>>,
	imagery: Arc<Mutex<dyn Imagery>>,
	pending: Arc<Mutex<HashSet<(u32, u32, u32)>>>,
}

impl ImageryFetcher {
	pub fn new(imagery: impl Imagery + 'static) -> Self {
		let fetcher = Self {
			cache: Arc::new(Mutex::new(LruCache::new(NonZeroUsize::new(256).unwrap()))),
			imagery: Arc::new(Mutex::new(imagery)),
			pending: Arc::new(Mutex::new(HashSet::new())),
		};

		let pending = fetcher.pending.clone();
		let cache = fetcher.cache.clone();
		let imagery = fetcher.imagery.clone();

		spawn(Self::fetching_loop(pending, cache, imagery));

		fetcher
	}

	async fn fetching_loop(
		pending: Arc<Mutex<HashSet<(u32, u32, u32)>>>,
		cache: Arc<Mutex<LruCache<(u32, u32, u32), Image>>>,
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
			let tile = *pending.iter().next().unwrap();
			pending.remove(&tile);
			let cache = cache.clone();
			let imagery = imagery.clone();
			join_set.spawn(async move {
				info!("Fetching tile: {tile:?}");
				let Some(image) = Self::fetch_tile(imagery, tile).await else {
					error!("Error fetching tile: {tile:?}");
					return;
				};
				cache.lock().unwrap().put(tile, image);
			});
		}
	}

	pub fn get_tile(&mut self, x: u32, y: u32, zoom: u32) -> Option<Image> {
		if let Some(image) = self.cache.lock().unwrap().get(&(x, y, zoom)) {
			return Some(image.clone());
		}
		self.pending.lock().unwrap().insert((x, y, zoom));
		return None;
	}

	async fn fetch_tile(imagery: Arc<Mutex<dyn Imagery>>, tile: (u32, u32, u32)) -> Option<Image> {
		let url = imagery.lock().unwrap().tile_url(tile.0, tile.1, tile.2);
		info!("Fetching tile: {url}");

		let result = reqwest::Client::new().get(url.clone()).send().await;

		let Ok(image_file) = result else {
			error!("Failed to fetch image from: {url}");
			error!("Error: {:?}", result.err().unwrap());
			return None;
		};

		let Ok(bytes) = image_file.bytes().await else {
			error!("Failed to read image bytes from: {url}");
			return None;
		};

		let Some(data) = Data::from_stream(&mut bytes.as_ref(), bytes.len()) else {
			error!("Failed to create image data from bytes");
			return None;
		};

		let Some(image) = Image::from_encoded(data) else {
			error!("Failed to create image from data");
			return None;
		};

		Some(image)
	}
}

#[cfg(test)]
mod tests {
	use std::sync::Once;
	use std::thread::sleep;
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
