use std::sync::Arc;

use egui::{ColorImage, TextureId, mutex::RwLock};
use epaint::TextureManager;
use log::error;

use crate::imagery::imagery_fetcher;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Icon(pub TextureId);

#[derive(Debug, thiserror::Error)]
pub enum IconError {
	#[error("{0}")]
	TextureFetchError(#[from] imagery_fetcher::Error),
	#[error("Error getting website url")]
	NoBaseUrl,
	#[error("Error Parsing Url: {0}")]
	UrlParseError(#[from] url::ParseError),
	#[error("Unable to parse SVG: {0}")]
	SvgParseError(#[from] resvg::usvg::Error),
	#[error("Unable to allocate PixMap")]
	PixmapAllocation,
	#[error("File '{0}' has no valid file extension.")]
	InvalidFileExtension(String),
	#[error("File type {0} not supported. Must be JPEG/JPG, PNG or SVG.")]
	UnsupportedFileType(String),
}

// TODO: Maybe use async io
pub fn try_get_icon_texture(
	file_name: &str,
	texture_manager: &Arc<RwLock<TextureManager>>,
	#[cfg(target_arch = "wasm32")] icons: &Arc<RwLock<std::collections::HashMap<String, Icon>>>,
	#[cfg(target_arch = "wasm32")] fallback_texture: TextureId,
) -> Result<TextureId, IconError> {
	#[cfg(not(target_arch = "wasm32"))]
	{
		use std::io::Read;
		let file_path = std::path::Path::new("./assets/styles").join(file_name);
		let mut file = std::fs::File::open(&file_path).map_err(imagery_fetcher::Error::ImageIo)?;
		let mut image_data = Vec::new();
		file.read_to_end(&mut image_data)
			.map_err(imagery_fetcher::Error::ImageIo)?;
		let bytes = bytes::Bytes::from(image_data);

		let extension = file_path
			.extension()
			.ok_or(IconError::InvalidFileExtension(file_name.to_string()))?
			.to_str()
			.ok_or(IconError::InvalidFileExtension(file_name.to_string()))?
			.to_lowercase();

		let color_image = decode_or_render(&extension, bytes)?;

		Ok(imagery_fetcher::ImageryFetcher::create_texture_from_decoded(
			texture_manager,
			format!("icon {file_name}"),
			color_image,
		))
	}

	#[cfg(target_arch = "wasm32")]
	{
		let icons = icons.clone();
		let texture_manager = texture_manager.clone();
		let base_url = web_sys::window()
			.ok_or(IconError::NoBaseUrl)?
			.location()
			.href()
			.map_err(|_| IconError::NoBaseUrl)?;
		let url = reqwest::Url::parse(&base_url)?.join("styles/")?.join(file_name)?;
		let file_name = file_name.to_string();

		log::info!("Fetching icon file: {url}");

		wasm_bindgen_futures::spawn_local(async move {
			let result = fetch_icon(url, &file_name, &texture_manager).await;

			match result {
				Ok(texture_id) => {
					icons.write().insert(file_name, Icon(texture_id));
				}

				Err(err) => error!("Unable to get icon file '{file_name}': {err:?}"),
			}
		});

		Ok(fallback_texture)
	}
}

#[cfg(target_arch = "wasm32")]
pub async fn fetch_icon(
	url: url::Url,
	file_name: &str,
	texture_manager: &Arc<RwLock<TextureManager>>,
) -> Result<TextureId, IconError> {
	let extension = file_name
		.split(".")
		.last()
		.ok_or(IconError::InvalidFileExtension(file_name.to_string()))?;

	let response = reqwest::get(url).await.map_err(imagery_fetcher::Error::Reqwest)?;
	let response = response.error_for_status().map_err(imagery_fetcher::Error::Reqwest)?;
	log::info!("{response:?}");

	let bytes = response.bytes().await.map_err(imagery_fetcher::Error::Reqwest)?;
	let color_image = decode_or_render(&extension, bytes)?;
	Ok(imagery_fetcher::ImageryFetcher::create_texture_from_decoded(
		texture_manager,
		format!("icon {file_name}"),
		color_image,
	))
}

fn decode_or_render(extension: &str, bytes: bytes::Bytes) -> Result<ColorImage, IconError> {
	if extension == "svg" {
		Ok(render_svg(&bytes)?)
	} else {
		Ok(imagery_fetcher::ImageryFetcher::decode_image(&bytes)?.0)
	}
}

fn render_svg(bytes: &[u8]) -> Result<ColorImage, IconError> {
	use resvg::{
		tiny_skia::Pixmap,
		usvg::{Options, Size, Transform, Tree},
	};

	const SVG_MIN_SIZE: f32 = 100.0; // TODO: Config

	let tree = Tree::from_data(bytes, &Options { ..Default::default() })?;

	let size = tree.size();
	let scaled_size = size.scale_to(Size::from_wh(SVG_MIN_SIZE, SVG_MIN_SIZE).unwrap());
	let scale_factor = Size::from_wh(scaled_size.width() / size.width(), scaled_size.height() / size.height()).unwrap();

	let mut pixmap =
		Pixmap::new(scaled_size.width() as u32, scaled_size.height() as u32).ok_or(IconError::PixmapAllocation)?;
	resvg::render(
		&tree,
		Transform::from_scale(scale_factor.width(), scale_factor.height()),
		&mut pixmap.as_mut(),
	);

	Ok(ColorImage::from_rgba_unmultiplied(
		[pixmap.width() as usize, pixmap.height() as usize],
		bytemuck::cast_slice(pixmap.pixels()),
	))
}
