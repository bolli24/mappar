use crate::imagery::cache::FileFormat;

mod cache;
#[cfg(target_arch = "wasm32")]
pub mod image_worker;
pub mod imagery_fetcher;
#[cfg(not(target_arch = "wasm32"))]
mod local_cache;
#[cfg(target_arch = "wasm32")]
mod web_cache;

pub trait Imagery: Send {
	fn tile_size(&self) -> u32;
	fn max_zoom(&self) -> u32;
	fn min_zoom(&self) -> u32;
	fn tile_url(&self, x: u32, y: u32, zoom: u32) -> String;
	fn file_format(&self) -> FileFormat;
}

#[must_use]
pub fn esri_imagery() -> UrlImagery {
	UrlImagery::new(
		"https://server.arcgisonline.com/arcgis/rest/services/World_Imagery/MapServer/tile/{zoom}/{y}/{x}".to_string(),
		256,
		23,
		0,
		FileFormat::JPG,
	)
	.unwrap()
}

#[cfg(not(target_arch = "wasm32"))]
#[must_use]
pub fn local_image_imagery() -> UrlImagery {
	UrlImagery::new(
		"http://localhost:6969/{zoom}/{x}/{y}.png".to_string(),
		256,
		19,
		12,
		FileFormat::PNG,
	)
	.unwrap()
}

pub struct UrlImagery {
	url: String,
	tile_size: u32,
	max_zoom: u32,
	min_zoom: u32,
	file_format: FileFormat,
}

impl UrlImagery {
	#[must_use]
	pub fn new(url: String, tile_size: u32, max_zoom: u32, min_zoom: u32, file_format: FileFormat) -> Option<Self> {
		if max_zoom < min_zoom {
			return None;
		}
		if tile_size == 0 {
			return None;
		}

		if !url.contains("{x}") || !url.contains("{y}") || !url.contains("{zoom}") {
			return None;
		}

		Some(Self {
			url,
			tile_size,
			max_zoom,
			min_zoom,
			file_format,
		})
	}
}

impl Imagery for UrlImagery {
	fn tile_size(&self) -> u32 {
		self.tile_size
	}

	fn max_zoom(&self) -> u32 {
		self.max_zoom
	}

	fn min_zoom(&self) -> u32 {
		self.min_zoom
	}

	fn tile_url(&self, x: u32, y: u32, zoom: u32) -> String {
		self.url
			.replace("{x}", &x.to_string())
			.replace("{y}", &y.to_string())
			.replace("{zoom}", &zoom.to_string())
	}

	fn file_format(&self) -> FileFormat {
		self.file_format
	}
}
