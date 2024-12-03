use chrono::{DateTime, Utc};
use glam::{dvec2, DVec2};
use indexmap::IndexMap;
use mappar_macros::Id;
use std::collections::HashMap;
use std::f64::consts::{FRAC_PI_2, FRAC_PI_4, PI};
use std::fmt::{Debug, Display};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct ElementId(pub i64, pub ElementType);

impl std::fmt::Display for ElementId {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}:{}", self.0, self.1)
	}
}

#[derive(Id, Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Version(pub u64);

#[derive(Id, Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct UserId(pub u32);

#[derive(Debug)]
pub struct MapData {
	elements: HashMap<ElementId, MapElement>,
}

impl MapData {
	pub fn new() -> Self {
		Self {
			elements: HashMap::new(),
		}
	}

	pub fn add(&mut self, element: MapElement) {
		self.elements.insert(element.id, element);
	}

	pub fn get(&self, id: &ElementId) -> Option<&MapElement> {
		self.elements.get(id)
	}

	pub fn get_mut(&mut self, id: &ElementId) -> Option<&mut MapElement> {
		self.elements.get_mut(id)
	}

	pub fn get_node_pos(&self, id: ElementId) -> Option<LatLon> {
		let e = self.get(&id)?;
		if let Element::Node { pos } = e.element {
			return Some(pos);
		}
		None
	}

	pub fn iter(&self) -> impl Iterator<Item = &MapElement> {
		self.elements.values()
	}

	pub fn len(&self) -> usize {
		self.elements.len()
	}
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
				self.has_area_tag()
			}
			Element::Relation { .. } => {
				// TODO: check if members form a closed area
				self.has_multipolygon_tag() && self.has_area_tag()
			}
		}
	}
	fn has_multipolygon_tag(&self) -> bool {
		self.tags.iter().any(|t| t.0 == "type" && t.1 == "multipolygon")
	}
	fn has_area_tag(&self) -> bool {
		self.tags
			.iter()
			.any(|t| (t.0 == "area" && t.1 == "yes") || is_area_tag(&t.0, &t.1))
	}
}
pub fn is_closed(nodes: &[ElementId]) -> bool {
	nodes.len() >= 2 && nodes[0] == nodes[nodes.len() - 1]
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
	pub tags: IndexMap<String, String>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum ElementType {
	Node,
	Way,
	Relation,
}

impl Display for ElementType {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Node => write!(f, "node"),
			Self::Way => write!(f, "way"),
			Self::Relation => write!(f, "relation"),
		}
	}
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
