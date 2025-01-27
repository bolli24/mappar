use crate::map::api::USER_AGENT;
use crate::map::map_data::LatLon;
use crate::ui::messages::Messages;
use crate::{message_error, message_info};
use celes::Country;
use quick_xml::de::Deserializer;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_inline_default::serde_inline_default;
use std::collections::HashMap;
use std::fmt::Display;
use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex};
use web_time::Instant;

#[cfg(target_arch = "wasm32")]
pub mod image_worker;
pub mod imagery_fetcher;
#[cfg(not(target_arch = "wasm32"))]
mod local_cache;
#[cfg(target_arch = "wasm32")]
mod web_cache;

mod schema;

pub const JOSM_URL: &str = "https://josm.openstreetmap.de/maps";
pub const JOSM_FILE: &str = "josm_imagery.xml";

pub fn get_imagery(
	#[cfg(not(target_arch = "wasm32"))] josm_xml_path: std::path::PathBuf,
	mut messages: Messages,
	imageries: Arc<Mutex<Imageries>>,
) {
	crate::spawn_task!(async move {
		if let Ok(xml) = get_local_xml(
			#[cfg(not(target_arch = "wasm32"))]
			josm_xml_path.clone(),
		)
		.await
		{
			message_info!(messages, "Found local imagery info.");
			imageries_from_str(&xml, messages, imageries).await;
		} else {
			message_info!(messages, "Could not find local iamgery info. Fetching...");
			update_josm_imagery(
				#[cfg(not(target_arch = "wasm32"))]
				josm_xml_path,
				messages,
				imageries,
			);
		}
	});
}

#[cfg(not(target_arch = "wasm32"))]
async fn get_local_xml(josm_xml_path: std::path::PathBuf) -> std::io::Result<String> {
	tokio::fs::read_to_string(&josm_xml_path).await
}

#[cfg(target_arch = "wasm32")]
async fn get_local_xml() -> Result<String, ()> {
	return Err(());
}

async fn fetch_josm_xml() -> Result<String, crate::map::api::Error> {
	let client = Client::new();
	let request = client.get(JOSM_URL).header("User-Agent", USER_AGENT).build()?;
	let xml = client.execute(request).await?.error_for_status()?.text().await?;
	Ok(xml)
}

pub fn update_josm_imagery(
	#[cfg(not(target_arch = "wasm32"))] josm_xml_path: std::path::PathBuf,
	mut messages: Messages,
	imageries: Arc<Mutex<Imageries>>,
) {
	crate::spawn_task!(async move {
		let xml = match fetch_josm_xml().await {
			Ok(xml) => xml,
			Err(err) => {
				message_error!(messages, "Unable to fetch JOSM imagery data: {err}");
				return;
			}
		};

		// FIXME: Wasm32
		#[cfg(not(target_arch = "wasm32"))]
		if let Err(err) = std::fs::write(josm_xml_path, &xml) {
			message_error!(messages, "Unable to write JOSM imagery data to file: {err}");
			return;
		};

		#[cfg(target_arch = "wasm32")]
		todo!("Cache JOSM XML locally.");

		imageries_from_str(&xml, messages, imageries).await;
	});
}

async fn imageries_from_str(xml: &str, mut messages: Messages, imageries: Arc<Mutex<Imageries>>) {
	let start = Instant::now();

	let mut deserializer = Deserializer::from_str(xml);
	deserializer.event_buffer_size(Some(NonZeroUsize::new(10000).unwrap()));
	let result = serde_path_to_error::deserialize(&mut deserializer);

	let parsed: Imageries = match result {
		Ok(imageries) => imageries,
		Err(err) => {
			let path = err.path().to_string();
			message_error!(messages, "Unable to parse JOSM imagery data: <{path}> - {err}");
			return;
		}
	};

	message_info!(
		messages,
		"Parsed {} imagery sources. Took: {:?}",
		parsed.entries.len(),
		start.elapsed()
	);
	*imageries.lock().unwrap() = parsed;
}

pub fn esri_imagery() -> Imagery {
	Imagery {
		id: "esri".into(),
		name: vec!["ESRI World Imagery".into()],
		description: None,
		imagery_type: ImageryType::Tms,
		url: "https://server.arcgisonline.com/arcgis/rest/services/World_Imagery/MapServer/tile/{zoom}/{y}/{x}"
			.to_string(),
		country: None,
		max_zoom: 22,
		min_zoom: 0,
		tile_size: Some(256),
		bounds: None,
	}
}

pub fn osm_imagery() -> Imagery {
	Imagery {
		id: "osm".into(),
		name: vec!["OpenStreetMap Carto".into()],
		description: None,
		imagery_type: ImageryType::Tms,
		url: "https://tile.openstreetmap.org/{zoom}/{x}/{y}.png".to_string(),
		country: None,
		max_zoom: 19,
		min_zoom: 0,
		tile_size: Some(256),
		bounds: None,
	}
}

impl From<&str> for Name {
	fn from(text: &str) -> Self {
		Name {
			lang: None,
			text: text.to_string(),
		}
	}
}

#[serde_inline_default]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Imagery {
	pub id: ImageryId,
	pub name: Vec<Name>,
	pub description: Option<Vec<Description>>,
	#[serde(rename = "type")]
	pub imagery_type: ImageryType,
	pub url: String,
	pub country: Option<Country>,
	#[serde_inline_default(20)]
	pub max_zoom: u32,
	#[serde_inline_default(0)]
	pub min_zoom: u32,
	pub tile_size: Option<u32>,
	pub bounds: Option<ImageryBounds>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ImageryBounds {
	#[serde(rename = "@max-lat")]
	pub max_lat: f64,
	#[serde(rename = "@max-lon")]
	pub max_lon: f64,
	#[serde(rename = "@min-lat")]
	pub min_lat: f64,
	#[serde(rename = "@min-lon")]
	pub min_lon: f64,
	pub shape: Option<Vec<Shape>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Shape {
	pub point: Vec<LatLon>,
}

/* #[derive(Serialize, Deserialize)]
pub struct Point {
	#[serde(rename = "@lat")]
	pub lat: String,
	#[serde(rename = "@lon")]
	pub lon: String,
}*/

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImageryType {
	Tms,
	Wmts,
	Wms,
	WmsEndpoint,
	Mvt,
	Bing,
	Scanex,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Imageries {
	#[serde(
		rename = "entry",
		serialize_with = "serialize_map",
		deserialize_with = "deserialize_vec_to_map"
	)]
	pub entries: HashMap<ImageryId, Imagery>,
}

impl Imageries {
	pub fn new() -> Self {
		Self {
			entries: HashMap::new(),
		}
	}
}

impl Default for Imageries {
	fn default() -> Self {
		Self::new()
	}
}

fn serialize_map<S>(map: &HashMap<ImageryId, Imagery>, serializer: S) -> Result<S::Ok, S::Error>
where
	S: serde::Serializer,
{
	let vec: Vec<&Imagery> = map.values().collect();
	vec.serialize(serializer)
}

fn deserialize_vec_to_map<'de, D>(deserializer: D) -> Result<HashMap<ImageryId, Imagery>, D::Error>
where
	D: serde::Deserializer<'de>,
{
	let vec: Vec<Imagery> = Vec::deserialize(deserializer)?;
	Ok(vec.into_iter().map(|imagery| (imagery.id.clone(), imagery)).collect())
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ImageryId(String);

impl From<&str> for ImageryId {
	fn from(value: &str) -> Self {
		Self(value.to_string())
	}
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Name {
	#[serde(rename = "$text")]
	pub text: String,
	#[serde(rename = "@lang")]
	pub lang: Option<String>,
}

impl Display for Name {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match &self.lang {
			None => write!(f, "{}", self.text),
			Some(lang) => write!(f, "{}:{}", lang, self.text),
		}
	}
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Description {
	#[serde(rename = "@lang")]
	pub lang: Option<String>,
	#[serde(rename = "$text")]
	pub text: Option<String>,
}

impl Display for Description {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.text.as_ref().unwrap_or(&"".to_string()))
	}
}

impl Imagery {
	pub fn tile_size(&self) -> Option<u32> {
		self.tile_size
	}

	pub fn max_zoom(&self) -> u32 {
		self.max_zoom
	}

	pub fn min_zoom(&self) -> u32 {
		self.min_zoom
	}

	pub fn tile_url(&self, x: u32, y: u32, zoom: u32) -> String {
		self.url
			.replace("{x}", &x.to_string())
			.replace("{y}", &y.to_string())
			.replace("{zoom}", &zoom.to_string())
	}

	pub fn name(&self) -> &str {
		&self.name[0].text
	}
}

#[cfg(test)]
mod tests {
	use log::{error, info};

	use super::*;

	#[test]
	pub fn xml_parsing() {
		crate::init_test_logger();
		let xml = include_str!("../../josm_imagery_test.xml");
		// let xml = include_str!("../../imagery_small.xml");
		let mut de = Deserializer::from_str(xml);
		let result: Result<Imageries, _> = serde_path_to_error::deserialize(&mut de);

		match result {
			Ok(imageries) => {
				info!("Success: {} sources parsed.", imageries.entries.len())
			}
			Err(err) => {
				let path = err.path().to_string();
				error!("Unable to parse JOSM imagery data: <{path}> - {err}");
			}
		};
	}
}
