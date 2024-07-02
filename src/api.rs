use std::fmt::{Debug, Display};
use std::str::from_utf8;
use std::time::Instant;

use log::{error, info, warn};
use quick_xml::events::attributes::Attributes;
use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;
use reqwest::{Client, StatusCode};

use crate::map_data::{Bounds, Element, ElementInfo, ElementType, LatLon, MapData, MapElement, Member, Tag};

#[cfg(not(debug_assertions))]
const API_URL: &str = "https://api.openstreetmap.org";

#[cfg(debug_assertions)]
const API_URL: &str = "https://master.apis.dev.openstreetmap.org";

const API_BOUNDING_BOX: &str = "/api/0.6/map?bbox=";

const USER_AGENT: &str = "mappar/0.1-alpha";

pub enum Error {
	Reqwest(reqwest::Error),
	ApiError(StatusCode, String),
}

impl Display for Error {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Reqwest(error) => write!(f, "Reqwest error: {error}"),
			Self::ApiError(status, body) => write!(f, "API error: {status} - {body}"),
		}
	}
}

impl Debug for Error {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		Display::fmt(self, f)
	}
}

impl From<reqwest::Error> for Error {
	fn from(error: reqwest::Error) -> Self {
		Self::Reqwest(error)
	}
}

pub async fn get_data(bounds: Bounds) -> Result<MapData, Error> {
	info!("Getting data from the API");
	let start = Instant::now();
	let url = format!("{}{}{}", API_URL, API_BOUNDING_BOX, bounds.as_string());
	let client = Client::new();
	let request = client.get(url).header("User-Agent", USER_AGENT).build()?;
	let response = client.execute(request).await?;

	if !response.status().is_success() {
		error!("Failed to get data from the API");
		return Err(Error::ApiError(response.status(), response.text().await?));
	}

	info!("Status: {:?}", response.status());
	info!("Response: {:?}", response);
	let xml = response.text().await?;

	info!("Time taken: {:?}", start.elapsed());
	let start = Instant::now();

	let data = parse_osm_xml(&xml);

	if cfg!(debug_assertions) {
		std::fs::write("api_data.xml", xml).expect("Error writing debug xml file");
	}

	info!("Parsed data successfully. Took: {:?}", start.elapsed());

	Ok(data)
}

fn parse_osm_xml(xml: &str) -> MapData {
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

					let mut tags: Option<Vec<Tag>> = None;
					let mut nodes = Vec::new();
					let mut members = Vec::new();
					loop {
						match reader.read_event() {
							Ok(Event::Empty(ref e)) => match e.name().as_ref() {
								b"tag" => {
									parse_tag(&mut tags, e);
								}
								b"nd" => {
									let node_ref = e.attributes().next().unwrap().unwrap();
									if node_ref.key.as_ref() == b"ref" {
										nodes.push(from_utf8(&node_ref.value).unwrap().parse().unwrap());
									}
								}
								b"member" => {
									let (ref_id, role, member_type) = parse_member(e);
									members.push(Member {
										ref_id,
										member_type,
										role,
									});
								}
								name => {
									warn!("Unknown tag: {:?}", from_utf8(name).unwrap());
								}
							},
							Ok(Event::End(_)) => {
								break;
							}
							_ => {}
						}
					}

					let (id, lat_lon, info) = parse_attributes(attributes);

					let element = match e.name().as_ref() {
						b"node" => Element::Node { pos: lat_lon.unwrap() },
						b"way" => Element::Way { nodes },
						b"relation" => Element::Relation { members },
						_ => {
							warn!("Unknown element type: {e:?}");
							continue;
						}
					};
					data.add(MapElement {
						id,
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
					let (id, lat_lon, info) = parse_attributes(e.attributes());

					data.add(MapElement {
						info,
						element: Element::Node { pos: lat_lon.unwrap() },
						tags: None,
						id,
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
	data
}

fn parse_member(e: &BytesStart) -> (i64, String, ElementType) {
	let mut ref_id: i64 = -1;
	let mut role = String::new();
	let mut member_type = ElementType::Node;
	for attr in e.attributes() {
		let attr = attr.unwrap();
		match attr.key.as_ref() {
			b"ref" => {
				ref_id = from_utf8(attr.value.as_ref()).unwrap().parse().unwrap();
			}
			b"role" => {
				role = from_utf8(attr.value.as_ref()).unwrap().to_string();
			}
			b"type" => member_type = ElementType::from_str(from_utf8(attr.value.as_ref()).unwrap()).unwrap(),
			_ => {
				error!(
					"Unknown attribute on relation member: {:?}=\"{:?}\"",
					attr.key, attr.value
				);
			}
		}
	}
	(ref_id, role, member_type)
}

fn parse_tag(tags: &mut Option<Vec<Tag>>, e: &BytesStart) {
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
		tags.get_or_insert(Vec::new()).push(Tag { key, value });
	}
}

fn parse_attributes(attributes: Attributes) -> (i64, Option<LatLon>, ElementInfo) {
	let mut lat = f64::NAN;
	let mut lon = f64::NAN;
	let mut id = 0;
	let mut info = ElementInfo::default();

	for attr in attributes {
		let attr = attr.unwrap();
		match attr.key.as_ref() {
			b"lat" => {
				lat = from_utf8(&attr.value).unwrap().parse().unwrap();
			}
			b"lon" => {
				lon = from_utf8(&attr.value).unwrap().parse().unwrap();
			}
			b"id" => {
				id = from_utf8(&attr.value).unwrap().parse().unwrap();
			}
			b"version" => {
				info.version = from_utf8(&attr.value).unwrap().parse().unwrap();
			}
			b"changeset" => {
				info.changeset = from_utf8(&attr.value).unwrap().parse().unwrap();
			}
			b"uid" => {
				info.uid = from_utf8(&attr.value).unwrap().parse().ok();
			}
			b"user" => {
				info.user = Some(from_utf8(&attr.value).unwrap().to_string());
			}
			b"visible" => {
				info.visible = from_utf8(&attr.value).unwrap() == "true";
			}
			b"timestamp" => {
				info.timestamp = from_utf8(&attr.value).unwrap().parse().unwrap();
			}
			_ => {
				warn!("Unknown attribute: {:?}", attr);
			}
		}
	}
	let lat_lon = if lat != f64::MAX && lon != f64::MAX {
		Some(LatLon(lat, lon))
	} else {
		None
	};
	(id, lat_lon, info)
}

#[cfg(test)]
mod tests {
	use crate::map_data::LatLon;

	use super::*;

	#[tokio::test]
	async fn test_get_data() {
		crate::init_test_logger();

		let bounds = Bounds {
			min: LatLon(51.82887, 12.19731),
			max: LatLon(51.82344, 12.21268),
		};
		let result = get_data(bounds).await;
		println!("Result: {result:?}");
		assert!(result.is_ok());
	}
}
