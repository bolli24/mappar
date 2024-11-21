use crate::imagery::imagery_fetcher::Tile;
use bytes::Bytes;

pub trait Cache {
	fn cached_count(&self) -> usize;
	async fn save_image(&mut self, tile: Tile, image_data: &Bytes, file_format: FileFormat) -> Option<()>;
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

impl TryFrom<&str> for FileFormat {
	type Error = ();
	fn try_from(s: &str) -> Result<Self, Self::Error> {
		Ok(match s.to_lowercase().as_str() {
			"png" => Self::PNG,
			"jpg" | "jpeg" => Self::JPG,
			_ => return Err(()),
		})
	}
}
