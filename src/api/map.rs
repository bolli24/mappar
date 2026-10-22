use crate::api::client::{ApiResult, Error, OsmClient};
use crate::map::bounds::Bounds;
use crate::map::element::{Element, ElementId, ElementInfo, ElementType, MapElement, Member};
use crate::map::latlon::{LatLon, latlon};
use crate::map::map_data::MapData;
use indexmap::IndexMap;
use log::{error, info, warn};
use quick_xml::events::attributes::Attributes;
use quick_xml::events::{BytesStart, Event};
use quick_xml::{Reader, XmlVersion};
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
					if !matches!(e.name().as_ref(), "node" | "way" | "relation") {
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
								"tag" => {
									parse_tag(&mut tags, e).ok()?;
								}
								"nd" => {
									let node_ref = e.attributes().next()?.ok()?;
									if node_ref.key.as_ref() == "ref" {
										nodes.push(ElementId::new(node_ref.value.parse().ok()?, ElementType::Node));
									}
								}
								"member" => {
									let (ref_id, role) = parse_member(e)?;
									members.push(Member { ref_id, role });
								}
								name => {
									warn!("Unknown tag: {:?}", name);
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
						"node" => (Element::Node { pos: lat_lon?.into() }, ElementType::Node),
						"way" => (Element::Way { nodes }, ElementType::Way),
						"relation" => (
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
						id: ElementId::new(id, element_type),
						info,
						element,
						tags,
						new_tag: None,
						generation: 0,
					});
				}
				Event::Empty(ref e) => {
					if e.name().as_ref() != "node" {
						warn!("Unknown element type: {e:?}");
						continue;
					}
					let (id, lat_lon, info) = parse_attributes(e.attributes())?;

					data.add(MapElement {
						id: ElementId::new(id, ElementType::Node),
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

fn parse_member(e: &BytesStart) -> Option<(ElementId, String)> {
	let mut ref_id: i64 = -1;
	let mut ref_type = ElementType::Node;
	let mut role = String::new();
	for attr in e.attributes() {
		let attr = attr.ok()?;
		match attr.key.as_ref() {
			"ref" => {
				ref_id = attr.value.parse().ok()?;
			}
			"role" => {
				role = attr.normalized_value(XmlVersion::Implicit1_0).ok()?.to_string();
			}
			"type" => ref_type = ElementType::try_from(attr.value.as_ref()).ok()?,
			_ => {
				error!(
					"Unknown attribute on relation member: {:?}=\"{:?}\"",
					attr.key, attr.value
				);
			}
		}
	}
	Some((ElementId::new(ref_id, ref_type), role))
}

fn parse_tag(tags: &mut IndexMap<String, String>, e: &BytesStart) -> Result<(), quick_xml::Error> {
	let mut key = String::new();
	let mut value = String::new();

	for a in e.attributes() {
		let a = a?;
		match a.key.as_ref() {
			"k" => key = a.normalized_value(XmlVersion::Implicit1_0)?.to_string(),
			"v" => value = a.normalized_value(XmlVersion::Implicit1_0)?.to_string(),
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
			"lat" => {
				lat = attr.value.parse().ok()?;
			}
			"lon" => {
				lon = attr.value.parse().ok()?;
			}
			"id" => {
				id = attr.value.parse().ok()?;
			}
			"version" => {
				info.version = attr.value.parse().ok()?;
			}
			"changeset" => {
				info.changeset = attr.value.parse().ok()?;
			}
			"uid" => {
				info.uid = attr.value.parse().ok();
			}
			"user" => {
				info.user = Some(attr.value.to_string());
			}
			"visible" => {
				info.visible = attr.value == "true";
			}
			"timestamp" => {
				info.timestamp = attr.value.parse().ok()?;
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
