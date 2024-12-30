use crate::imagery::imagery_fetcher::Tile;
use bytes::Bytes;
use image::ImageFormat;

pub trait Cache {
	fn cached_count(&self) -> usize;
	async fn save_image(&mut self, tile: Tile, image_data: &Bytes, image_format: ImageFormat) -> Option<()>;
	async fn get_image(&self, tile: Tile) -> Option<Bytes>;
}
