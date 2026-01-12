use crate::LatLon;
use chrono::{DateTime, Utc};
use indexmap::IndexMap;
use mappar_macros::Id;
use std::cell::Cell;
use std::fmt::Display;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct ElementId(pub i64, pub ElementType);

impl Display for ElementId {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}:{}", self.0, self.1)
	}
}

#[derive(Id, Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Version(pub u64);

#[derive(Id, Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct UserId(pub u32);

#[derive(Debug, Clone)]
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

#[must_use]
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

#[derive(Debug, Clone)]
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
			if let Some(value) = self.get_tag_with_new(tag)
				&& !value.is_empty()
			{
				return value.clone();
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

#[derive(Debug, Clone)]
pub enum Element {
	Node { pos: Cell<LatLon> },
	Way { nodes: Vec<ElementId> }, // TODO: change this to Vec<i64>
	Relation { members: Vec<Member> },
}
