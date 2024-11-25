use chrono::{DateTime, Utc};
use glam::{dvec2, DVec2};
use mappar_macros::Id;
use std::collections::HashMap;
use std::f64::consts::{FRAC_PI_2, FRAC_PI_4, PI};
use std::fmt::Debug;

#[derive(Id, Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct ElementId(pub i64);

#[derive(Id, Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Version(pub u64);

#[derive(Id, Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct UserId(pub u32);

#[derive(Debug)]
pub struct MapData {
	nodes: HashMap<ElementId, MapElement>,
	ways: HashMap<ElementId, MapElement>,
	relations: HashMap<ElementId, MapElement>,
}

impl MapData {
	pub fn new() -> Self {
		Self {
			nodes: HashMap::new(),
			ways: HashMap::new(),
			relations: HashMap::new(),
		}
	}

	pub fn add(&mut self, element: MapElement) {
		match &element.element {
			Element::Node { .. } => self.nodes.insert(element.id, element),
			Element::Way { .. } => self.ways.insert(element.id, element),
			Element::Relation { .. } => self.relations.insert(element.id, element),
		};
	}

	pub fn add_node(&mut self, element: MapElement) {
		self.nodes.insert(element.id, element);
	}
	pub fn add_way(&mut self, element: MapElement) {
		self.ways.insert(element.id, element);
	}
	pub fn add_relation(&mut self, element: MapElement) {
		self.relations.insert(element.id, element);
	}

	pub fn get_node(&self, id: &ElementId) -> Option<&MapElement> {
		self.nodes.get(id)
	}
	pub fn get_way(&self, id: &ElementId) -> Option<&MapElement> {
		self.ways.get(id)
	}
	pub fn get_relation(&self, id: &ElementId) -> Option<&MapElement> {
		self.relations.get(id)
	}

	pub fn get_node_pos(&self, id: ElementId) -> Option<LatLon> {
		let e = self.nodes.get(&id)?;
		if let Element::Node { pos } = e.element {
			return Some(pos);
		}
		None
	}

	pub fn iter(&self) -> impl Iterator<Item = &MapElement> {
		self.relations
			.values()
			.chain(self.ways.values())
			.chain(self.nodes.values())
	}

	pub fn len(&self) -> usize {
		self.nodes.len() + self.ways.len() + self.relations.len()
	}
}

#[derive(Debug)]
pub struct Tag {
	pub key: String,
	pub value: String,
}

#[derive(Debug)]
pub struct ElementInfo {
	pub version: Version,
	pub timestamp: DateTime<Utc>,
	pub changeset: Version,
	pub uid: Option<UserId>,
	pub user: Option<String>,
	pub visible: bool,
}

impl Default for ElementInfo {
	fn default() -> Self {
		Self {
			version: Version(0),
			timestamp: Utc::now(),
			changeset: Version(0),
			uid: None,
			user: None,
			visible: true,
		}
	}
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct LatLon(pub f64, pub f64);

pub const RADIUS_EARTH: f64 = 6_378_137.0;
pub const CIRCUMFERENCE_EARTH: f64 = 2.0 * PI * RADIUS_EARTH;

pub const WEB_MERCATOR_CUTOFF: f64 = 85.051_129; // 2.0 * f64::atan(E.powf(PI)) - PI / 2.0

impl LatLon {
	#[inline]
	pub fn to_web_mercator(self) -> DVec2 {
		DVec2 {
			x: RADIUS_EARTH * self.1.to_radians(),
			y: f64::ln(f64::tan(self.0.to_radians().mul_add(0.5, FRAC_PI_4))) * RADIUS_EARTH,
		}
	}

	#[inline]
	pub fn to_web_mercator_render(self) -> DVec2 {
		let pos = self.to_web_mercator();
		dvec2(pos.x, -pos.y)
	}

	#[inline]
	pub fn from_web_mercator(pos: DVec2) -> Self {
		Self(
			f64::atan(f64::exp(pos.y / RADIUS_EARTH))
				.mul_add(2.0, -FRAC_PI_2)
				.to_degrees(),
			(pos.x / RADIUS_EARTH).to_degrees(),
		)
	}
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Bounds {
	pub min: LatLon,
	pub max: LatLon,
}

impl Bounds {
	pub fn center(&self) -> LatLon {
		LatLon((self.top() + self.bottom()) / 2.0, (self.left() + self.right()) / 2.0)
	}
	pub fn left(&self) -> f64 {
		f64::min(self.min.1, self.max.1)
	}

	pub fn right(&self) -> f64 {
		f64::max(self.min.1, self.max.1)
	}

	pub fn top(&self) -> f64 {
		f64::max(self.min.0, self.max.0)
	}

	pub fn bottom(&self) -> f64 {
		f64::min(self.min.0, self.max.0)
	}

	pub fn top_left(&self) -> LatLon {
		LatLon(self.top(), self.left())
	}

	pub fn top_right(&self) -> LatLon {
		LatLon(self.top(), self.right())
	}

	pub fn bottom_left(&self) -> LatLon {
		LatLon(self.bottom(), self.left())
	}

	pub fn bottom_right(&self) -> LatLon {
		LatLon(self.bottom(), self.right())
	}

	pub fn as_string(&self) -> String {
		format!("{},{},{},{}", self.left(), self.bottom(), self.right(), self.top())
	}
}

impl MapElement {
	pub fn is_area(&self) -> bool {
		match &self.element {
			Element::Node { .. } => false,
			Element::Way { nodes } => {
				if !is_closed(nodes.as_slice()) {
					return false;
				}
				has_area_tag(&self.tags)
			}
			Element::Relation { .. } => {
				// TODO: check if members form a closed area
				has_multipolygon_tag(&self.tags) && has_area_tag(&self.tags)
			}
		}
	}
}
pub fn is_closed(nodes: &[ElementId]) -> bool {
	nodes.len() >= 2 && nodes[0] == nodes[nodes.len() - 1]
}

fn has_multipolygon_tag(tags: &[Tag]) -> bool {
	tags.iter().any(|t| t.key == "type" && t.value == "multipolygon")
}

fn has_area_tag(tags: &[Tag]) -> bool {
	tags.iter()
		.any(|t| (t.key == "area" && t.value == "yes") || is_area_tag(&t.key, &t.value))
}

fn is_area_tag(key: &str, value: &str) -> bool {
	AREA_TAGS.iter().any(|tag| match tag {
		(t_key, None) => **t_key == *key,
		(t_key, Some(t_value)) => **t_key == *key && **t_value == *value,
	})
}

// TODO: Expand
// TODO: Maybe use perfect hash table or other better data structure
pub const AREA_TAGS: [(&str, Option<&str>); 5] = [
	("building", None),
	("natural", Some("water")),
	("landuse", None),
	("area:highway", None),
	("leisure", Some("park")),
];

#[derive(Debug)]
pub struct MapElement {
	pub id: ElementId,
	pub info: ElementInfo,
	pub element: Element,
	pub tags: Vec<Tag>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ElementType {
	Node,
	Way,
	Relation,
}

impl ElementType {
	pub fn from_str(str: &str) -> Option<Self> {
		match str {
			"node" => Some(Self::Node),
			"way" => Some(Self::Way),
			"relation" => Some(Self::Relation),
			_ => None,
		}
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Member {
	pub ref_id: ElementId,
	pub element_type: ElementType,
	pub role: String,
}

#[derive(Debug)]
pub enum Element {
	Node { pos: LatLon },
	Way { nodes: Vec<ElementId> },
	Relation { members: Vec<Member> },
}

#[cfg(test)]
mod tests {
	use super::*;
	use log::info;

	#[test]
	fn test_lat_lon() {
		crate::init_test_logger();
		let lat_lon = LatLon(51.82887, 12.19731);
		let web_mercator = lat_lon.to_web_mercator();
		info!("Web Mercator: {:?}", web_mercator);
		let lat_lon2 = LatLon::from_web_mercator(web_mercator);
		let diff = (lat_lon.0 - lat_lon2.0).abs() + (lat_lon.1 - lat_lon2.1).abs();
		assert!(diff < 0.0001);
	}
}
