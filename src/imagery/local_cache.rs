use std::collections::HashSet;
use std::path::PathBuf;
use std::time::Instant;

use bytes::Bytes;
use log::{info, warn};
use regex::Regex;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::imagery::cache::{Cache, FileFormat};
use crate::imagery::imagery_fetcher::Tile;

pub struct LocalCache {
	file_path: PathBuf,
	cached_images: HashSet<(Tile, FileFormat)>,
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

		let mut cached_images = HashSet::new();

		for file in files {
			if let Some(entry) = try_get_file(file, &file_regex) {
				cached_images.insert(entry);
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

// TODO: Maybe better error handling
fn try_get_file(file: Result<std::fs::DirEntry, std::io::Error>, file_regex: &Regex) -> Option<(Tile, FileFormat)> {
	let file = file.ok()?;
	let file_name = file.file_name();
	let file_name = file_name.to_str()?;
	if !file_regex.is_match(file_name) {
		warn!("Cache: Skipping invalid file: {}", file_name);
		return None;
	}
	let file_end = file_name.split('.').last().unwrap();
	let file_format = FileFormat::try_from(file_end).ok()?;
	let tile = file_name.split('.').take(1).collect::<Vec<&str>>()[0]
		.split('_')
		.collect::<Vec<&str>>();
	let tile = (
		tile[1].parse().unwrap(),
		tile[2].parse().unwrap(),
		tile[0].parse().unwrap(),
	);
	Some((tile, file_format))
}

// TODO: Maybe better error handling
impl Cache for LocalCache {
	fn cached_count(&self) -> usize {
		self.cached_images.len()
	}

	async fn save_image(&mut self, tile: Tile, image_data: &Bytes, file_format: FileFormat) -> Option<()> {
		let file_path = self.file_path(tile, file_format);
		let mut file = File::create(file_path).await.ok()?;
		file.write_all(image_data).await.ok()?;
		self.cached_images.insert((tile, file_format));
		Some(())
	}

	async fn get_image(&self, tile: Tile, file_format: FileFormat) -> Option<Bytes> {
		if !self.cached_images.contains(&(tile, file_format)) {
			return None;
		}

		let file_path = self.file_path(tile, file_format);
		let mut file = File::open(file_path).await.ok()?;

		let mut image_data = Vec::new();
		file.read_to_end(&mut image_data)
			.await
			.expect("Failed to read image data");

		Some(Bytes::from(image_data))
	}
}
