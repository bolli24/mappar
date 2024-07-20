use crate::imagery_fetcher::Tile;
use bytes::Bytes;
use log::info;
use regex::Regex;
use std::collections::HashSet;
use std::path::PathBuf;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub struct LocalCache {
	file_path: PathBuf,
	cached_images: HashSet<Tile>,
}

#[derive(Copy, Clone)]
pub enum FileFormat {
	PNG,
	JPG,
}

impl FileFormat {
	pub const fn file_extension(&self) -> &'static str {
		match self {
			Self::PNG => "png",
			Self::JPG => "jpg",
		}
	}
}
impl LocalCache {
	pub fn new(file_path: PathBuf) -> Self {
		info!("Using cache directory: {}", file_path.to_str().unwrap());
		let start = std::time::Instant::now();
		std::fs::DirBuilder::new()
			.recursive(true)
			.create(&file_path)
			.expect("Failed to create cache directory");

		let files = std::fs::read_dir(&file_path).expect("Failed to read cache directory");
		let file_regex = Regex::new(r"^\d+_\d+_\d+\.(png|jpg)$").expect("Failed to create regex");

		let mut cached_images = HashSet::new();

		for file in files {
			let file = file.expect("Failed to read file");
			let file_name = file.file_name();
			let file_name = file_name.to_str().expect("Failed to convert file name to string");
			if !file_regex.is_match(file_name) {
				info!("Cache: Skipping invalid file: {}", file_name);
				continue;
			}

			let tile = file_name.split('.').take(1).collect::<Vec<&str>>()[0]
				.split('_')
				.collect::<Vec<&str>>();
			let tile = (
				tile[1].parse().unwrap(),
				tile[2].parse().unwrap(),
				tile[0].parse().unwrap(),
			);
			cached_images.insert(tile);
		}

		info!("Found {} cached images. Took: {:.1}ms", cached_images.len(), start.elapsed().as_millis());

		Self {
			file_path,
			cached_images,
		}
	}

	pub fn cached_count(&self) -> usize {
		self.cached_images.len()
	}

	pub async fn save_image(&mut self, tile: Tile, image_data: &Bytes, file_format: FileFormat) {
		let file_path = self.file_path(tile, file_format);
		let mut file = File::create(file_path).await.expect("Failed to create file");
		file.write_all(image_data).await.expect("Failed to write image data");
		self.cached_images.insert(tile);
	}

	pub async fn get_image(&self, tile: Tile, file_format: FileFormat) -> Option<Bytes> {
		if !self.cached_images.contains(&tile) {
			return None;
		}

		let file_path = self.file_path(tile, file_format);
		let mut file = File::open(file_path).await.expect("Failed to open file");

		let mut image_data = Vec::new();
		file.read_to_end(&mut image_data)
			.await
			.expect("Failed to read image data");

		Some(Bytes::from(image_data))
	}

	fn file_path(&self, tile: Tile, file_format: FileFormat) -> PathBuf {
		self.file_path.join(format!(
			"{}_{}_{}.{}",
			tile.2,
			tile.0,
			tile.1,
			file_format.file_extension()
		))
	}
}
