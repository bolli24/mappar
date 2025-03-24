use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use bytes::Bytes;
use image::ImageFormat;
use log::{error, warn};
use regex::Regex;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_stream::StreamExt;

use crate::imagery::ImageryId;
use crate::imagery::imagery_fetcher::Tile;
use crate::message_info;
use crate::ui::messages::Messages;

#[derive(Clone)]
pub struct LocalCache {
	file_path: PathBuf,
	cached_images: Arc<tokio::sync::Mutex<HashMap<Tile, ImageFormat>>>,
	last_count: usize,
}

impl LocalCache {
	pub fn new(file_path: PathBuf, mut messages: Messages) -> Self {
		message_info!(messages, "Using cache directory: {}", file_path.to_str().unwrap());

		let cached_images = Arc::new(tokio::sync::Mutex::new(HashMap::new()));

		let new = Self {
			file_path: file_path.clone(),
			cached_images: cached_images.clone(),
			last_count: 0,
		};

		crate::spawn_task!(async move {
			let start = Instant::now();
			std::fs::DirBuilder::new()
				.recursive(true)
				.create(&file_path)
				.expect("Failed to create cache directory");

			let mut dirs = tokio_stream::wrappers::ReadDirStream::new(
				tokio::fs::read_dir(&file_path)
					.await
					.expect("Failed to read cache directory"),
			);

			let file_regex = Regex::new(r"^\d+_\d+_\d+\.(png|jpg)$").expect("Failed to create regex");

			while let Some(dir) = dirs.next().await {
				let dir_path = match dir {
					Ok(dir) => {
						let path = dir.path();
						if !path.is_dir() {
							continue;
						}
						path
					}
					Err(err) => {
						error!("Skipping invalid directory: {err}");
						continue;
					}
				};

				let mut files = tokio_stream::wrappers::ReadDirStream::new(
					tokio::fs::read_dir(&dir_path)
						.await
						.expect("Failed to read cache directory"),
				);

				let imagery_id = ImageryId(dir_path.file_name().unwrap().to_string_lossy().to_string());

				while let Some(file) = files.next().await {
					if let Some((coords, image_format)) = try_get_file(file, &file_regex) {
						let tile = Tile::new(imagery_id.clone(), coords);
						cached_images.lock().await.insert(tile, image_format);
					}
				}
			}

			message_info!(
				messages,
				"Found {} cached images. Took: {:.1}ms",
				cached_images.lock().await.len(),
				start.elapsed().as_millis()
			);
		});

		new
	}

	fn file_path(&self, tile: &Tile, image_format: ImageFormat) -> PathBuf {
		self.file_path.join(&tile.imagery_id.0).join(format!(
			"{}_{}_{}.{}",
			tile.coords.2,
			tile.coords.0,
			tile.coords.1,
			image_format.extensions_str()[0]
		))
	}

	pub fn cached_count(&mut self) -> usize {
		self.last_count = self.cached_images.try_lock().map_or(self.last_count, |h| h.len());
		self.last_count
	}

	pub async fn save_image(&mut self, tile: &Tile, image_data: &Bytes, image_format: ImageFormat) -> Option<()> {
		let file_path = self.file_path(tile, image_format);
		let mut file = File::create(file_path).await.ok()?;
		file.write_all(image_data).await.ok()?;
		self.cached_images.lock().await.insert(tile.clone(), image_format);
		Some(())
	}

	pub async fn get_image(&self, tile: &Tile) -> Option<Bytes> {
		let &image_format = self.cached_images.lock().await.get(tile)?;

		let file_path = self.file_path(tile, image_format);
		let mut file = File::open(file_path).await.ok()?;

		let mut image_data = Vec::new();
		file.read_to_end(&mut image_data)
			.await
			.expect("Failed to read image data");

		Some(Bytes::from(image_data))
	}
}

// TODO: Maybe better error handling
fn try_get_file(
	file: Result<tokio::fs::DirEntry, std::io::Error>,
	file_regex: &Regex,
) -> Option<((u32, u32, u32), ImageFormat)> {
	puffin::profile_function!();
	let file = file.ok()?;
	let file_path = file.path();
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
