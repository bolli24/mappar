use super::edit::{Action, Edit, EditError};
use chrono::{DateTime, Utc};
use glam::{DVec2, dvec2};
use indexmap::IndexMap;
use mappar_macros::Id;
use std::cell::Cell;
use std::collections::{HashMap, HashSet};
use std::f64::consts::{FRAC_PI_2, FRAC_PI_4, PI};
use std::fmt::{Debug, Display};
use std::ops::Add;
use undo::Record;

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

pub struct MapData {
	elements: HashMap<ElementId, MapElement>,
	history: Record<Edit>,
}

impl MapData {
	pub fn new() -> Self {
		Self {
			elements: HashMap::new(),
			history: Record::new(),
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
		if let Element::Node { pos } = &e.element {
			return Some(pos.get());
		}
		None
	}

	pub fn iter(&self) -> impl Iterator<Item = &MapElement> {
		self.elements.values()
	}

	pub fn len(&self) -> usize {
		self.elements.len()
	}

	pub fn history_len(&self) -> usize {
		self.history.len()
	}

	pub fn edit(&mut self, edit: Edit) -> Result<ElementId, EditError> {
		self.history.edit(&mut self.elements, edit)
	}

	pub fn undo(&mut self) -> Option<Result<ElementId, EditError>> {
		self.history.undo(&mut self.elements)
	}

	pub fn redo(&mut self) -> Option<Result<ElementId, EditError>> {
		self.history.undo(&mut self.elements)
	}

	pub fn add_tag(&self, id: ElementId, key: String, value: String) -> Edit {
		Edit(id, Action::AddTag { key, value })
	}

	pub fn add_new_tag(&self, id: ElementId) -> Edit {
		let (key, value) = self.get(&id).unwrap().new_tag.clone().unwrap();
		Edit(id, Action::AddNewTag { key, value })
	}

	pub fn override_tag(&self, id: ElementId, key: String, value: String) -> Edit {
		Edit(
			id,
			Action::OverrideTag {
				previous_value: self.get_value(&id, &key),
				key,
				value,
			},
		)
	}

	pub fn update_value(&self, id: ElementId, key: String, value: String) -> Edit {
		Edit(
			id,
			Action::UpdateValue {
				previous_value: self.get_value(&id, &key),
				key,
				value,
			},
		)
	}
	pub fn remove_tag(&self, id: ElementId, key: String) -> Edit {
		let Some(element) = self.get(&id) else {
			unreachable!("Element {id} must exist.");
		};
		let Some(index) = element.tags.get_index_of(&key) else {
			unreachable!("Element {id} must have key '{key}' to be overwritten.");
		};
		let value = element.tags.get_index(index).unwrap().1.clone();
		Edit(id, Action::RemoveTag { key, value, index })
	}

	pub fn update_new_tag(&self, id: ElementId, key: String, value: String) -> Edit {
		let Some(element) = self.get(&id) else {
			unreachable!("Element {id} must exist.");
		};
		Edit(
			id,
			Action::UpdateNewTag {
				current: (key, value),
				previous: element.new_tag.clone(),
			},
		)
	}

	// TODO: use Edit
	pub fn move_by<'a>(&mut self, elements: impl IntoIterator<Item = &'a ElementId>, delta: &DVec2) {
		let mut moved = HashSet::with_capacity(100);

		for element_id in elements.into_iter() {
			self.move_element(&mut moved, element_id, delta);
		}
	}

	fn move_element(&mut self, moved: &mut HashSet<ElementId>, element_id: &ElementId, delta: &DVec2) {
		if !moved.insert(*element_id) {
			return;
		}
		let element = self.get(element_id).unwrap();
		match &element.element {
			Element::Node { pos } => {
				pos.set(pos.get() + *delta);
			}
			Element::Way { nodes } => {
				debug_assert!(
					nodes.len() >= 2 && !is_closed(nodes) || nodes.len() >= 3,
					"Way must be proper shape."
				);
				for node in nodes.clone() {
					self.move_element(moved, &node, delta);
				}
			}
			Element::Relation { .. } => {
				todo!("Moving relations.")
				// TODO: Do I even have to do this?
			}
		}
	}

	/// # Safety:
	/// Panics if element or key does not exist.
	fn get_value(&self, id: &ElementId, key: &String) -> String {
		let Some(element) = self.get(id) else {
			unreachable!("Element {id} must exist.");
		};
		let Some(previous_value) = element.tags.get(key) else {
			unreachable!("Element {id} must have key '{key}' to be overwritten.");
		};
		previous_value.clone()
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

#[derive(Copy, Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LatLon {
	#[serde(rename = "@lat")]
	pub lat: f64,
	#[serde(rename = "@lon")]
	pub lon: f64,
}
pub const RADIUS_EARTH: f64 = 6_378_137.0;
pub const CIRCUMFERENCE_EARTH: f64 = 2.0 * PI * RADIUS_EARTH;

pub const WEB_MERCATOR_CUTOFF: f64 = 85.051_129; // 2.0 * f64::atan(E.powf(PI)) - PI / 2.0

impl LatLon {
	#[inline]
	pub fn to_web_mercator(self) -> DVec2 {
		DVec2 {
			x: RADIUS_EARTH * self.lon.to_radians(),
			y: f64::ln(f64::tan(self.lat.to_radians().mul_add(0.5, FRAC_PI_4))) * RADIUS_EARTH,
		}
	}

	#[inline]
	pub fn to_web_mercator_render(self) -> DVec2 {
		let pos = self.to_web_mercator();
		dvec2(pos.x, -pos.y)
	}

	#[inline]
	pub fn from_web_mercator(pos: DVec2) -> Self {
		Self {
			lat: f64::atan(f64::exp(pos.y / RADIUS_EARTH))
				.mul_add(2.0, -FRAC_PI_2)
				.to_degrees(),
			lon: (pos.x / RADIUS_EARTH).to_degrees(),
		}
	}
}

impl Add<DVec2> for LatLon {
	type Output = LatLon;

	fn add(self, rhs: DVec2) -> Self::Output {
		LatLon::from_web_mercator(self.to_web_mercator() + rhs)
	}
}

#[derive(Copy, Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Bounds {
	pub min: LatLon,
	pub max: LatLon,
}

pub fn latlon(lat: f64, lon: f64) -> LatLon {
	LatLon { lat, lon }
}

impl Bounds {
	pub fn center(&self) -> LatLon {
		latlon((self.top() + self.bottom()) / 2.0, (self.left() + self.right()) / 2.0)
	}
	pub fn left(&self) -> f64 {
		f64::min(self.min.lon, self.max.lon)
	}

	pub fn right(&self) -> f64 {
		f64::max(self.min.lon, self.max.lon)
	}

	pub fn top(&self) -> f64 {
		f64::max(self.min.lat, self.max.lat)
	}

	pub fn bottom(&self) -> f64 {
		f64::min(self.min.lat, self.max.lat)
	}

	pub fn top_left(&self) -> LatLon {
		latlon(self.top(), self.left())
	}

	pub fn top_right(&self) -> LatLon {
		latlon(self.top(), self.right())
	}

	pub fn bottom_left(&self) -> LatLon {
		latlon(self.bottom(), self.left())
	}

	pub fn bottom_right(&self) -> LatLon {
		latlon(self.bottom(), self.right())
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
			.any(|t| (t.0 == "area" && t.1 == "yes") || is_area_tag(t.0, t.1))
	}

	pub fn get_tag_with_new(&self, key: &str) -> Option<&String> {
		self.tags.get(key).or_else(|| {
			self.new_tag.as_ref().and_then(
				|(new_key, new_value)| {
					if new_key == key { Some(new_value) } else { None }
				},
			)
		})
	}

	pub fn is_closed(&self) -> bool {
		match &self.element {
			Element::Way { nodes } => is_closed(nodes),
			_ => false,
		}
	}
}
pub fn is_closed(nodes: &[ElementId]) -> bool {
	debug_assert!(nodes.len() >= 2);
	nodes[0] == nodes[nodes.len() - 1]
}

pub fn unique_nodes(nodes: &[ElementId]) -> impl Iterator<Item = &ElementId> + '_ {
	let closed = is_closed(nodes);
	nodes.iter().skip(if closed { 1 } else { 0 })
}

pub fn unique_nodes_mut(nodes: &mut [ElementId]) -> impl Iterator<Item = &mut ElementId> + '_ {
	let closed = is_closed(nodes);
	nodes.iter_mut().skip(if closed { 1 } else { 0 })
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
	pub new_tag: Option<(String, String)>,
}

pub const AUTO_TEXT_TAGS: [&str; 6] = ["name", "int_name", "ref", "operator", "brand", "addr:housenumber"];

impl MapElement {
	// TODO: also consider "name:" + <LANG>
	pub fn get_auto_text(&self) -> String {
		for tag in AUTO_TEXT_TAGS {
			if let Some(value) = self.get_tag_with_new(tag) {
				if !value.is_empty() {
					return value.to_string();
				}
			}
		}
		String::new()
	}
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

impl TryFrom<&str> for ElementType {
	type Error = ();

	fn try_from(value: &str) -> Result<Self, Self::Error> {
		match value {
			"node" => Ok(Self::Node),
			"way" => Ok(Self::Way),
			"relation" => Ok(Self::Relation),
			_ => Err(()),
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
	Node { pos: Cell<LatLon> },
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
		let lat_lon = LatLon {
			lat: 51.82887,
			lon: 12.19731,
		};
		let web_mercator = lat_lon.to_web_mercator();
		info!("Web Mercator: {:?}", web_mercator);
		let lat_lon2 = LatLon::from_web_mercator(web_mercator);
		let diff = (lat_lon.lat - lat_lon2.lat).abs() + (lat_lon.lon - lat_lon2.lon).abs();
		assert!(diff < 0.0001);
	}
}
