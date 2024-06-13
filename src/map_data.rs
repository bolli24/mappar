use std::collections::hash_map::Values;
use std::collections::HashMap;
use std::f64::consts::PI;
use std::fmt::Debug;

use chrono::{DateTime, Utc};
use egui::Vec2;

#[derive(Debug)]
pub struct MapData {
	elements: HashMap<i64, MapElement>,
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

	pub fn get_element(&self, id: i64) -> Option<&MapElement> {
		self.elements.get(&id)
	}

	pub fn get_node_pos(&self, id: i64) -> Option<LatLon> {
		let e = self.elements.get(&id)?;
		if let Element::Node { pos } = e.element {
			return Some(pos);
		}
		None
	}

	pub fn iter(&self) -> Values<'_, i64, MapElement> {
		self.elements.values()
	}
}

#[derive(Debug)]
pub struct Tag {
	pub key: String,
	pub value: String,
}

#[derive(Debug)]
pub struct ElementInfo {
	pub version: u64,
	pub timestamp: DateTime<Utc>,
	pub changeset: u64,
	pub uid: Option<u32>,
	pub user: Option<String>,
	pub visible: bool,
}

impl Default for ElementInfo {
	fn default() -> Self {
		Self {
			version: 0,
			timestamp: Utc::now(),
			changeset: 0,
			uid: None,
			user: None,
			visible: true,
		}
	}
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct LatLon(pub f64, pub f64);

pub const RADIUS_EARTH: f64 = 6_378_137.0;
const DEG_TO_RAG: f64 = 180.0 / PI;

impl LatLon {
	#[inline]
	pub fn to_web_mercator(&self) -> Vec2 {
		Vec2 {
			x: (RADIUS_EARTH * self.1.to_radians()) as f32,
			y: (f64::ln(f64::tan(PI / 4.0 + self.0.to_radians() / 2.0)) * RADIUS_EARTH) as f32,
		}
	}
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Bounds {
	pub min: LatLon,
	pub max: LatLon,
}

impl Bounds {
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

	pub fn as_string(&self) -> String {
		format!("{},{},{},{}", self.left(), self.bottom(), self.right(), self.top())
	}
}

#[derive(Debug)]
pub struct MapElement {
	pub id: i64,
	pub info: ElementInfo,
	pub element: Element,
	pub tags: Option<Vec<Tag>>,
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
			"node" => Some(ElementType::Node),
			"way" => Some(ElementType::Way),
			"relation" => Some(ElementType::Relation),
			_ => None,
		}
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Member {
	pub ref_id: i64,
	pub member_type: ElementType,
	pub role: String,
}

#[derive(Debug)]
pub enum Element {
	Node { pos: LatLon },
	Way { nodes: Vec<i64> },
	Relation { members: Vec<Member> },
}
