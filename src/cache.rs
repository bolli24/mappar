use crate::imagery_fetcher::Tile;
use bytes::Bytes;

pub trait Cache {
	fn cached_count(&self) -> usize;
	async fn save_image(&mut self, tile: Tile, image_data: &Bytes, file_format: FileFormat);
	async fn get_image(&self, tile: Tile, file_format: FileFormat) -> Option<Bytes>;
}

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub enum FileFormat {
	PNG,
	JPG,
}

impl FileFormat {
	pub const fn file_extension(self) -> &'static str {
		match self {
			Self::PNG => "png",
			Self::JPG => "jpg",
		}
	}
}

impl From<&str> for FileFormat {
	fn from(s: &str) -> Self {
		match s {
			"png" => Self::PNG,
			"jpg" => Self::JPG,
			_ => panic!("Unknown file format: {}", s),
		}
	}
}
