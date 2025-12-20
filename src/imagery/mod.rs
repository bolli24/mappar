use crate::map::map_data::LatLon;
use crate::ui::messages::Messages;
use crate::{message_error, message_info};
use celes::Country;
use chumsky::Parser;
use indexmap::IndexMap;
use lasso::{Rodeo, Spur};
use log::error;
use quick_xml::de::Deserializer;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_inline_default::serde_inline_default;
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

#[cfg(not(target_arch = "wasm32"))]
pub const JOSM_URL: &str = "https://josm.openstreetmap.de/maps";
// josm.openstreetmap.de/maps doesn't support CORS, which is required for requests in the browser,
// so we host our own hourly-updated copy
#[cfg(target_arch = "wasm32")]
pub const JOSM_URL: &str = "https://pub-a872030d163249caaf83979e9d4fe79a.r2.dev/josm.xml";

pub const JOSM_FILE: &str = "josm_imagery.xml";

#[derive(PartialEq, Eq, Hash, Debug, Copy, Clone, serde::Serialize, serde::Deserialize)]
pub struct ImageryRef(Spur);

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
			let _ = imageries_from_str(&xml, messages, imageries);
		} else {
			message_info!(messages, "Could not find local imagery info. Fetching...");
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
	#[cfg(not(target_arch = "wasm32"))]
	let request = client
		.get(JOSM_URL)
		.header("User-Agent", crate::map::api::USER_AGENT)
		.build()?;
	#[cfg(target_arch = "wasm32")]
	let request = client.get(JOSM_URL).build()?;
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

		// TODO: Wasm32: Cache iamgery locally
		#[cfg(not(target_arch = "wasm32"))]
		if imageries_from_str(&xml, messages.clone(), imageries).is_ok() {
			if let Err(err) = std::fs::write(josm_xml_path, &xml) {
				message_error!(messages, "Unable to write JOSM imagery data to file: {err}");
			}
		}

		#[cfg(target_arch = "wasm32")]
		let _ = imageries_from_str(&xml, messages, imageries);
	});
}

fn imageries_from_str(xml: &str, mut messages: Messages, imageries: Arc<Mutex<Imageries>>) -> Result<(), ()> {
	let start = Instant::now();

	let mut deserializer = Deserializer::from_str(xml);
	deserializer.event_buffer_size(Some(NonZeroUsize::new(10000).unwrap()));
	let result = serde_path_to_error::deserialize(&mut deserializer);

	let mut parsed: Imageries = match result {
		Ok(imageries) => imageries,
		Err(err) => {
			let path = err.path().to_string();
			message_error!(messages, "Unable to parse JOSM imagery data: <{path}> - {err}");
			return Err(());
		}
	};

	let mut to_remove = Vec::new();

	for (id, imagery) in &parsed.entries {
		if imagery.imagery_type != ImageryType::Tms || imagery.tile_url(0, 0, 0).is_none() {
			to_remove.push(id.clone());
		}
	}

	for id in to_remove {
		parsed.entries.shift_remove(&id);
	}

	message_info!(
		messages,
		"Parsed {} imagery sources. Took: {:?}",
		parsed.entries.len(),
		start.elapsed()
	);
	*imageries.lock().unwrap() = parsed;
	Ok(())
}

impl From<&str> for Name {
	fn from(text: &str) -> Self {
		Self {
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
	pub url: ImageryUrl,
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
	pub entries: IndexMap<ImageryId, Imagery>,

	#[serde(skip)]
	pub ids: Rodeo,
}

impl Imageries {
	#[must_use]
	pub fn new() -> Self {
		Self {
			entries: IndexMap::new(),
			ids: Rodeo::new(),
		}
	}

	pub fn get_ref_unchecked(&mut self, id: &ImageryId) -> ImageryRef {
		ImageryRef(self.ids.get_or_intern(&id.0))
	}

	pub fn get_ref(&mut self, id: &ImageryId) -> Option<ImageryRef> {
		self.entries.contains_key(id).then(|| self.get_ref_unchecked(id))
	}

	pub fn get(&self, reference: ImageryRef) -> Option<&Imagery> {
		self.entries.get(&ImageryId(
			self.ids
				.try_resolve(&reference.0)
				.expect("Imagery reference must be valid.")
				.to_string(),
		))
	}

	/* pub fn get_ref(&self, id: &ImageryId) -> Option<ImageryRef> {
		self.entries.get(id).and_then(|_| self.ids.get(&id.0)).map(ImageryRef)
	} */
}

impl Default for Imageries {
	fn default() -> Self {
		Self::new()
	}
}

fn serialize_map<S>(map: &IndexMap<ImageryId, Imagery>, serializer: S) -> Result<S::Ok, S::Error>
where
	S: serde::Serializer,
{
	let vec: Vec<&Imagery> = map.values().collect();
	vec.serialize(serializer)
}

fn deserialize_vec_to_map<'de, D>(deserializer: D) -> Result<IndexMap<ImageryId, Imagery>, D::Error>
where
	D: serde::Deserializer<'de>,
{
	let vec: Vec<Imagery> = Vec::deserialize(deserializer)?;
	Ok(vec.into_iter().map(|imagery| (imagery.id.clone(), imagery)).collect())
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ImageryId(pub String);

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
		write!(f, "{}", self.text.as_ref().unwrap_or(&String::new()))
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

	pub fn tile_url(&self, x: u32, y: u32, zoom: u32) -> Option<String> {
		let mut url = String::new();
		for segment in &self.url.0 {
			match segment {
				UrlSegment::String(s) => url.push_str(s),
				UrlSegment::Switch(options) => {
					let selected = &options[(x + y + zoom) as usize % options.len()];
					url.push_str(selected);
				}
				UrlSegment::Placeholder(placeholder) => match placeholder {
					Placeholder::X => url.push_str(&x.to_string()),
					Placeholder::Y => url.push_str(&y.to_string()),
					Placeholder::MinusY => url.push_str(&(2i32.pow(zoom) - 1 - (y as i32)).to_string()),
					Placeholder::Zoom(offset) => url.push_str(&(zoom as i8 + offset).to_string()),
					_ => return None, // TODO: Support all url placeholders
				},
			}
		}
		Some(url)
	}

	pub fn name(&self) -> &str {
		&self.name[0].text
	}
}

#[derive(Debug, Clone)]
enum Placeholder {
	X,
	Y,
	BangY,
	MinusY,
	Zoom(i8),
	Projection,
	Width,
	Height,
	BoundingBox,
	ApiKey,
	Time,
	/*W,
	S,
	N,
	E,*/
	WkId,
}

#[derive(Debug, Clone)]
enum UrlSegment {
	String(String),
	Switch(Vec<String>),
	Placeholder(Placeholder),
}

#[derive(Debug, Clone)]
pub struct ImageryUrl(Vec<UrlSegment>);

fn imagery_url_parser<'a>() -> impl Parser<'a, &'a str, ImageryUrl, chumsky::extra::Err<chumsky::error::Rich<'a, char>>>
{
	use chumsky::prelude::*;

	let url_string = any()
		.filter(|c| *c != '{' && *c != '}')
		.repeated()
		.at_least(1)
		.collect::<String>();

	let string = any()
		.filter(|c| *c != '{' && *c != '}' && *c != ':' && *c != ',')
		.repeated()
		.at_least(1)
		.collect::<String>();

	let number = text::digits(10)
		.at_most(2)
		.collect::<String>()
		.map(|n| n.parse::<i8>().unwrap());

	let zoom_offset = (just('+').or(just('-')).then(number)).or(empty().map(|()| ('+', 0)));
	let zoom = just("zoom").ignore_then(zoom_offset).map(|(sign, mut number)| {
		if sign == '-' {
			number = -number;
		}
		Placeholder::Zoom(number)
	});

	let placeholder = string
		.try_map(|placeholder, span| {
			let p = match placeholder.as_str() {
				"x" => Placeholder::X,
				"y" => Placeholder::Y,
				"!y" => Placeholder::BangY,
				"-y" => Placeholder::MinusY,
				"proj" => Placeholder::Projection,
				"width" => Placeholder::Width,
				"height" => Placeholder::Height,
				"bbox" => Placeholder::BoundingBox,
				"apikey" => Placeholder::ApiKey,
				"wkid" => Placeholder::WkId,
				"time" => Placeholder::Time,
				_ => return Err(Rich::custom(span, format!("Invalid placeholder: {placeholder}"))),
			};
			Ok(UrlSegment::Placeholder(p))
		})
		.or(zoom.map(UrlSegment::Placeholder))
		.delimited_by(just('{'), just('}'));

	let switch_options = string // Can be replaced with 'text::ascii::ident()' once chumsky updates past
		.map(String::from)
		.or(empty().map(|()| String::new()))
		.separated_by(just(','))
		.at_least(1)
		.collect::<Vec<String>>();
	let switch = just("switch:")
		.ignore_then(switch_options)
		.delimited_by(just('{'), just('}'))
		.map(UrlSegment::Switch);

	let segment = choice((switch, placeholder, url_string.map(UrlSegment::String)));

	segment.repeated().collect().map(ImageryUrl)
}

impl Serialize for ImageryUrl {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		let url_str = self.to_string();
		serializer.serialize_str(&url_str)
	}
}

impl<'de> Deserialize<'de> for ImageryUrl {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		let url_str = String::deserialize(deserializer)?;
		let parse_result = imagery_url_parser().parse(&url_str);

		match parse_result.into_result() {
			Ok(url) => Ok(url),
			Err(err) => {
				error!("Failed to parse imagery URL '{url_str}': \n{err:?}");
				Ok(Self(vec![UrlSegment::String(url_str.to_string())]))
			}
		}
	}
}
impl Display for ImageryUrl {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		for segment in &self.0 {
			match segment {
				UrlSegment::String(s) => write!(f, "{s}")?,
				UrlSegment::Placeholder(p) => {
					let p_str = match p {
						Placeholder::X => "x",
						Placeholder::Y => "y",
						Placeholder::BangY => "!y",
						Placeholder::MinusY => "-y",
						Placeholder::Zoom(number) if *number == 0 => "zoom",
						Placeholder::Zoom(number) if *number < 0 => &format!("zoom{number}"),
						Placeholder::Zoom(number) => &format!("zoom+{number}"),
						Placeholder::Projection => "proj",
						Placeholder::Width => "width",
						Placeholder::Height => "height",
						Placeholder::BoundingBox => "bbox",
						Placeholder::ApiKey => "apikey",
						Placeholder::WkId => "wkid",
						Placeholder::Time => "time",
					};
					write!(f, "{{{p_str}}}")?;
				}
				UrlSegment::Switch(options) => write!(f, "{{switch:{}}}", options.join(","))?,
			}
		}
		Ok(())
	}
}

impl<'a> TryFrom<&'a str> for ImageryUrl {
	type Error = Vec<chumsky::error::Rich<'a, char>>;

	fn try_from(value: &'a str) -> Result<Self, Self::Error> {
		imagery_url_parser().parse(value).into_result()
	}
}

#[cfg(test)]
mod tests {
	use log::{error, info};

	use super::*;

	#[test]
	pub fn xml_parsing() {
		crate::init_test_logger();
		let xml = include_str!("../../test/josm_imagery_test.xml");
		// let xml = include_str!("../../imagery_small.xml");
		let mut de = Deserializer::from_str(xml);
		let result: Result<Imageries, _> = serde_path_to_error::deserialize(&mut de);

		match result {
			Ok(imageries) => {
				info!("Success: {} sources parsed.", imageries.entries.len());
			}
			Err(err) => {
				let path = err.path().to_string();
				error!("Unable to parse JOSM imagery data: <{path}> - {err}");
			}
		}
	}

	#[test]
	pub fn url_parsing() {
		crate::init_test_logger();

		let url = "https://{switch:,ab1,b}.tile-cyclosm.openstreetmap.fr/cyclosm/{zoom-16}{x}/{y}.png";
		let result = imagery_url_parser().parse(url);

		info!("{result:?}");

		assert!(result.into_result().is_ok());
	}
}
