use crate::api::client::{ApiResult, Error, OsmClient};
use crate::map::bounds::Bounds;
use crate::map::element::{Element, ElementId, ElementInfo, ElementType, MapElement, Member};
use crate::map::latlon::{LatLon, latlon};
use crate::map::map_data::MapData;
use indexmap::IndexMap;
use log::{error, info, warn};
use quick_xml::Reader;
use quick_xml::events::attributes::Attributes;
use quick_xml::events::{BytesStart, Event};
use std::str::from_utf8;
use web_time::Instant;

impl OsmClient {
	pub async fn get_data_from_bounds(&self, bounds: Bounds) -> ApiResult<MapData> {
		let start = Instant::now();
		info!("Getting data from the API");
		let xml = self.get(&format!("/api/0.6/map?bbox={bounds}")).await?.text().await?;
		info!("Time taken: {:?}", start.elapsed());
		let start = Instant::now();

		let data = parse_osm_xml(&xml).ok_or(Error::Xml)?;

		#[cfg(all(not(target_arch = "wasm32"), debug_assertions))]
		std::fs::write("../api_data.xml", xml).expect("Error writing debug xml file");

		info!("Parsed data successfully. Took: {:?}", start.elapsed());

		Ok(data)
	}
}

// TODO: better Error handling, maybe use anyhow
pub(crate) fn parse_osm_xml(xml: &str) -> Option<MapData> {
	let mut data = MapData::new();
	let mut reader = Reader::from_str(xml);

	loop {
		match reader.read_event() {
			Ok(event) => match event {
				Event::Start(ref e) => {
					if !matches!(e.name().as_ref(), b"node" | b"way" | b"relation") {
						warn!("Unknown element: {e:?}");
						continue;
					}
					let attributes = e.attributes();

					let mut tags = IndexMap::new();
					let mut nodes = Vec::new();
					let mut members = Vec::new();
					loop {
						match reader.read_event() {
							Ok(Event::Empty(ref e)) => match e.name().as_ref() {
								b"tag" => {
									parse_tag(&mut tags, e).ok()?;
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
						b"node" => (Element::Node { pos: lat_lon?.into() }, ElementType::Node),
						b"way" => (Element::Way { nodes }, ElementType::Way),
						b"relation" => (
							Element::Relation {
								members,
								fully_downloaded: false,
							},
							ElementType::Relation,
						),
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
						new_tag: None,
						generation: 0,
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
						element: Element::Node { pos: lat_lon?.into() },
						tags: IndexMap::new(),
						new_tag: None,
						generation: 0,
					});
				}
				Event::Eof => {
					break;
				}
				_ => {}
			},
			Err(e) => {
				error!("Error: {e:?}");
				break;
			}
		}
	}

	data.iter()
		.filter_map(|element| {
			if let Element::Relation { members, .. } = &element.element {
				let all_present = members.iter().all(|m| data.get(&m.ref_id).is_some());
				if all_present { Some(element.id) } else { None }
			} else {
				None
			}
		})
		.collect::<Vec<_>>()
		.iter()
		.for_each(|id| {
			if let Some(MapElement {
				element: Element::Relation { fully_downloaded, .. },
				..
			}) = data.get_mut(id)
			{
				*fully_downloaded = true;
			} else {
				unreachable!("Element must exist.")
			}
		});

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
				role = attr.unescape_value().unwrap().to_string();
			}
			b"type" => ref_id.1 = ElementType::try_from(from_utf8(attr.value.as_ref()).unwrap()).unwrap(),
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

fn parse_tag(tags: &mut IndexMap<String, String>, e: &BytesStart) -> Result<(), quick_xml::Error> {
	let mut key = String::new();
	let mut value = String::new();

	for a in e.attributes() {
		let a = a?;
		match a.key.as_ref() {
			b"k" => key = a.unescape_value()?.to_string(),
			b"v" => value = a.unescape_value()?.to_string(),
			_ => {}
		}
	}
	if !key.is_empty() && !value.is_empty() {
		tags.insert(key, value);
	}

	Ok(())
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
				warn!("Unknown attribute: {attr:?}");
			}
		}
	}
	let lat_lon = if !lat.is_nan() && !lon.is_nan() {
		Some(latlon(lat, lon))
	} else {
		None
	};
	Some((id, lat_lon, info))
}

#[cfg(test)]
mod tests {
	use crate::map::latlon::latlon;

	use super::*;

	#[tokio::test]
	async fn test_get_data_from_bounds() {
		crate::init_test_logger();
		let client = OsmClient::new(None, None);

		let bounds = Bounds {
			min: latlon(51.82887, 12.19731),
			max: latlon(51.82344, 12.21268),
		};
		client.get_data_from_bounds(bounds).await.unwrap();
	}
}
