use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;

use bytes::Bytes;
use image::ImageFormat;
use log::{info, warn};
use regex::Regex;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::imagery::cache::Cache;
use crate::imagery::imagery_fetcher::Tile;

pub struct LocalCache {
	file_path: PathBuf,
	cached_images: HashMap<Tile, ImageFormat>,
}

impl LocalCache {
	pub fn new(file_path: PathBuf) -> Self {
		info!("Using cache directory: {}", file_path.to_str().unwrap());
		let start = Instant::now();
		std::fs::DirBuilder::new()
			.recursive(true)
			.create(&file_path)
			.expect("Failed to create cache directory");

		let files = std::fs::read_dir(&file_path).expect("Failed to read cache directory");
		let file_regex = Regex::new(r"^\d+_\d+_\d+\.(png|jpg)$").expect("Failed to create regex");

		let mut cached_images = HashMap::new();

		for file in files {
			if let Some((tile, image_format)) = try_get_file(file, &file_regex) {
				cached_images.insert(tile, image_format);
			}
		}

		info!(
			"Found {} cached images. Took: {:.1}ms",
			cached_images.len(),
			start.elapsed().as_millis()
		);

		Self {
			file_path,
			cached_images,
		}
	}

	fn file_path(&self, tile: Tile, image_format: ImageFormat) -> PathBuf {
		self.file_path.join(format!(
			"{}_{}_{}.{}",
			tile.2,
			tile.0,
			tile.1,
			image_format.extensions_str()[0]
		))
	}
}

// TODO: Maybe better error handling
fn try_get_file(file: Result<std::fs::DirEntry, std::io::Error>, file_regex: &Regex) -> Option<(Tile, ImageFormat)> {
	let file = file.ok()?;
	let file_path = PathBuf::from(file.path());
	let file_name = file.file_name();
	let file_name = file_name.to_str()?;
	if !file_regex.is_match(file_name) {
		warn!("Cache: Skipping invalid file: {}", file_name);
		return None;
	}
	let image_format = ImageFormat::from_extension(file_path.extension()?)?;
	let tile = file_path.file_stem()?.to_str()?.split('_').collect::<Vec<&str>>();
	let tile = (
		tile[1].parse().unwrap(),
		tile[2].parse().unwrap(),
		tile[0].parse().unwrap(),
	);
	Some((tile, image_format))
}

// TODO: Maybe better error handling
impl Cache for LocalCache {
	fn cached_count(&self) -> usize {
		self.cached_images.len()
	}

	async fn save_image(&mut self, tile: Tile, image_data: &Bytes, image_format: ImageFormat) -> Option<()> {
		let file_path = self.file_path(tile, image_format);
		let mut file = File::create(file_path).await.ok()?;
		file.write_all(image_data).await.ok()?;
		self.cached_images.insert(tile, image_format);
		Some(())
	}

	async fn get_image(&self, tile: Tile) -> Option<Bytes> {
		let Some(image_format) = self.cached_images.get(&tile) else {
			return None;
		};

		let file_path = self.file_path(tile, *image_format);
		let mut file = File::open(file_path).await.ok()?;

		let mut image_data = Vec::new();
		file.read_to_end(&mut image_data)
			.await
			.expect("Failed to read image data");

		Some(Bytes::from(image_data))
	}
}
