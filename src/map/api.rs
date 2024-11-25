use super::map_data::ElementId;
use crate::map::map_data::{Bounds, Element, ElementInfo, ElementType, LatLon, MapData, MapElement, Member, Tag};
use log::{error, info, warn};
use quick_xml::events::attributes::Attributes;
use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;
use reqwest::{Client, StatusCode};
use std::fmt::Debug;
use std::str::from_utf8;

#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;
#[cfg(target_arch = "wasm32")]
use web_time::Instant;

#[cfg(not(debug_assertions))]
const API_URL: &str = "https://api.openstreetmap.org";

#[cfg(debug_assertions)]
const API_URL: &str = "https://master.apis.dev.openstreetmap.org";

const API_BOUNDING_BOX: &str = "/api/0.6/map?bbox=";

const USER_AGENT: &str = concat!("mappar/", env!("CARGO_PKG_VERSION"));

#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error("Reqwest error: {0}")]
	Reqwest(#[from] reqwest::Error),
	#[error("API error: {0} - {1}")]
	Api(StatusCode, String),
	#[error("XML error")]
	XML,
}

pub async fn get_data(bounds: Bounds) -> Result<MapData, Error> {
	info!("Getting data from the API");
	let start = Instant::now();
	let url = format!("{}{}{}", API_URL, API_BOUNDING_BOX, bounds.as_string());
	let client = Client::new();
	info!("User-Agent: {USER_AGENT}");
	let request = client.get(url).header("User-Agent", USER_AGENT).build()?;
	let response = client.execute(request).await?;

	if !response.status().is_success() {
		error!("Failed to get data from the API");
		return Err(Error::Api(response.status(), response.text().await?));
	}

	info!("Status: {:?}", response.status());
	info!("Response: {:?}", response);
	let xml = response.text().await?;

	info!("Time taken: {:?}", start.elapsed());
	let start = Instant::now();

	let data = parse_osm_xml(&xml).ok_or(Error::XML)?;

	#[cfg(not(target_arch = "wasm32"))]
	if cfg!(debug_assertions) {
		std::fs::write("api_data.xml", xml).expect("Error writing debug xml file");
	}

	info!("Parsed data successfully. Took: {:?}", start.elapsed());

	Ok(data)
}

fn parse_osm_xml(xml: &str) -> Option<MapData> {
	let mut data = MapData::new();
	let mut reader = Reader::from_str(xml);

	loop {
		match reader.read_event() {
			Ok(event) => match event {
				Event::Start(ref e) => {
					if !matches!(e.name().as_ref(), b"node" | b"way" | b"relation") {
						warn!("Unknown element: {:?}", e);
						continue;
					}
					let attributes = e.attributes();

					let mut tags: Vec<Tag> = Vec::new();
					let mut nodes = Vec::new();
					let mut members = Vec::new();
					loop {
						match reader.read_event() {
							Ok(Event::Empty(ref e)) => match e.name().as_ref() {
								b"tag" => {
									parse_tag(&mut tags, e);
								}
								b"nd" => {
									let node_ref = e.attributes().next()?.ok()?;
									if node_ref.key.as_ref() == b"ref" {
										nodes.push(ElementId(
											from_utf8(&node_ref.value).ok()?.parse().ok()?,
											ElementType::Node,
										));
									}
								}
								b"member" => {
									let (ref_id, role) = parse_member(e);
									members.push(Member { ref_id, role });
								}
								name => {
									warn!("Unknown tag: {:?}", from_utf8(name).ok()?);
								}
							},
							Ok(Event::End(_)) => {
								break;
							}
							_ => {}
						}
					}

					let (id, lat_lon, info) = parse_attributes(attributes)?;

					let (element, element_type) = match e.name().as_ref() {
						b"node" => (Element::Node { pos: lat_lon? }, ElementType::Node),
						b"way" => (Element::Way { nodes }, ElementType::Way),
						b"relation" => (Element::Relation { members }, ElementType::Relation),
						_ => {
							warn!("Unknown element type: {e:?}");
							continue;
						}
					};
					data.add(MapElement {
						id: ElementId(id, element_type),
						info,
						element,
						tags,
					});
				}
				Event::Empty(ref e) => {
					if e.name().as_ref() != b"node" {
						warn!("Unknown element type: {e:?}");
						continue;
					}
					let (id, lat_lon, info) = parse_attributes(e.attributes())?;

					data.add(MapElement {
						id: ElementId(id, ElementType::Node),
						info,
						element: Element::Node { pos: lat_lon? },
						tags: Vec::new(),
					});
				}
				Event::Eof => {
					break;
				}
				_ => {}
			},
			Err(e) => {
				error!("Error: {:?}", e);
				break;
			}
		}
	}
	Some(data)
}

fn parse_member(e: &BytesStart) -> (ElementId, String) {
	let mut ref_id = ElementId(-1, ElementType::Node);
	let mut role = String::new();
	for attr in e.attributes() {
		let attr = attr.unwrap();
		match attr.key.as_ref() {
			b"ref" => {
				ref_id.0 = from_utf8(attr.value.as_ref()).unwrap().parse().unwrap();
			}
			b"role" => {
				role = from_utf8(attr.value.as_ref()).unwrap().to_string();
			}
			b"type" => ref_id.1 = ElementType::from_str(from_utf8(attr.value.as_ref()).unwrap()).unwrap(),
			_ => {
				error!(
					"Unknown attribute on relation member: {:?}=\"{:?}\"",
					attr.key, attr.value
				);
			}
		}
	}
	(ref_id, role)
}

fn parse_tag(tags: &mut Vec<Tag>, e: &BytesStart) {
	let mut key = String::new();
	let mut value = String::new();

	for a in e.attributes() {
		let a = a.unwrap();
		match a.key.as_ref() {
			b"k" => {
				key = from_utf8(&a.value).unwrap().to_string();
			}
			b"v" => {
				value = from_utf8(&a.value).unwrap().to_string();
			}
			_ => {}
		}
	}
	if !key.is_empty() && !value.is_empty() {
		tags.push(Tag { key, value });
	}
}

fn parse_attributes(attributes: Attributes) -> Option<(i64, Option<LatLon>, ElementInfo)> {
	let mut lat = f64::NAN;
	let mut lon = f64::NAN;
	let mut id = 0;
	let mut info = ElementInfo::default();

	for attr in attributes {
		let attr = attr.ok()?;
		match attr.key.as_ref() {
			b"lat" => {
				lat = from_utf8(&attr.value).ok()?.parse().ok()?;
			}
			b"lon" => {
				lon = from_utf8(&attr.value).ok()?.parse().ok()?;
			}
			b"id" => {
				id = from_utf8(&attr.value).ok()?.parse().ok()?;
			}
			b"version" => {
				info.version = from_utf8(&attr.value).ok()?.parse().ok()?;
			}
			b"changeset" => {
				info.changeset = from_utf8(&attr.value).ok()?.parse().ok()?;
			}
			b"uid" => {
				info.uid = from_utf8(&attr.value).ok()?.parse().ok();
			}
			b"user" => {
				info.user = Some(from_utf8(&attr.value).ok()?.to_string());
			}
			b"visible" => {
				info.visible = from_utf8(&attr.value).ok()? == "true";
			}
			b"timestamp" => {
				info.timestamp = from_utf8(&attr.value).ok()?.parse().ok()?;
			}
			_ => {
				warn!("Unknown attribute: {:?}", attr);
			}
		}
	}
	let lat_lon = if !lat.is_nan() && !lon.is_nan() {
		Some(LatLon(lat, lon))
	} else {
		None
	};
	Some((id, lat_lon, info))
}

#[cfg(test)]
mod tests {
	use crate::map::map_data::LatLon;

	use super::*;

	#[tokio::test]
	async fn test_get_data() {
		crate::init_test_logger();

		let bounds = Bounds {
			min: LatLon(51.82887, 12.19731),
			max: LatLon(51.82344, 12.21268),
		};
		let result = get_data(bounds).await;
		info!("Result: {result:?}");
		assert!(result.is_ok());
	}
}
