use egui::{Color32, Pos2, Rect, Vec2};
use glam::DVec2;
use itertools::Itertools;

use crate::map_data::{Element, MapData, MapElement};
use crate::map_widget::dvec2_to_pos;

pub struct DetailedElement {
	pub element: RenderElement,
	pub style: Style,
	pub bounding_box: Rect,
}

pub enum RenderElement {
	Node(DVec2),
	Way(Vec<DVec2>),
	Collection(Vec<DetailedElement>),
}

impl RenderElement {
	pub fn bounding_box(&self) -> Rect {
		match self {
			Self::Node(pos) => Rect::from_center_size(dvec2_to_pos(*pos), Vec2::splat(2.0)),
			Self::Way(nodes) => {
				assert!(!nodes.is_empty(), "Empty way");
				let min_max_x = nodes.iter().map(|n| n.x as f32).minmax().into_option().unwrap();
				let min_max_y = nodes.iter().map(|n| n.y as f32).minmax().into_option().unwrap();

				Rect::from_min_max(Pos2::new(min_max_x.0, min_max_y.0), Pos2::new(min_max_x.1, min_max_y.1))
			}
			Self::Collection(elements) => {
				let points: Vec<_> = elements
					.iter()
					.flat_map(|e| [e.bounding_box.min, e.bounding_box.max])
					.collect();

				Rect::from_points(&points)
			}
		}
	}
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Style {
	Colored { color: Color32, size: f32, fill: bool },
	Invisible,
}

pub const DEFAULT_WAY_STYLE: Style = Style::Colored {
	color: Color32::BLACK,
	size: 1.0,
	fill: false,
};

pub const DEFAULT_NODE_STYLE: Style = Style::Colored {
	color: Color32::BLACK,
	size: 5.0,
	fill: true,
};

impl DetailedElement {
	pub fn from_map_element(map_element: &MapElement, map_data: &MapData) -> Self {
		let element = match &map_element.element {
			Element::Node { pos } => RenderElement::Node(pos.to_web_mercator_render()),
			Element::Way { nodes } => {
				let nodes: Vec<DVec2> = nodes
					.iter()
					.map(|n| {
						map_data
							.get_node_pos(*n)
							.unwrap_or_else(|| panic!("Couldn't get pos of node {n}"))
							.to_web_mercator_render()
					})
					.collect();

				RenderElement::Way(nodes)
			}
			Element::Relation { members } => {
				let members = members
					.iter()
					.filter_map(|m| map_data.get_element(m.ref_id))
					.map(|m| Self::from_map_element(m, map_data))
					.collect();
				RenderElement::Collection(members)
			}
		};

		let style = match &map_element.element {
			Element::Node { .. } => DEFAULT_NODE_STYLE,
			Element::Way { .. } => DEFAULT_WAY_STYLE,
			Element::Relation { .. } => Style::Invisible,
		};

		Self {
			bounding_box: element.bounding_box(),
			element,
			style,
		}
	}

	pub fn hovered(&self, pointer_pos: DVec2) -> bool {
		match &self.element {
			RenderElement::Node(pos) => (pointer_pos - *pos).length() < 5.0,
			RenderElement::Way(nodes) => {
				self.bounding_box.contains(dvec2_to_pos(pointer_pos)) && point_in_polygon(pointer_pos, nodes)
			}
			RenderElement::Collection(elements) => {
				for element in elements {
					if element.hovered(pointer_pos) {
						return true;
					}
				}
				false
			}
		}
	}
}

/// **Adapted from**: <https://wrfranklin.org/Research/Short_Notes/pnpoly.html>
pub fn point_in_polygon(test: DVec2, verts: &[DVec2]) -> bool {
	let mut result = false;
	let mut i = 0;
	let mut j = 0;

	while i < verts.len() {
		if ((verts[i].y > test.y) != (verts[j].y > test.y))
			&& (test.x < (verts[j].x - verts[i].x) * (test.y - verts[i].y) / (verts[j].y - verts[i].y) + verts[i].x)
		{
			result = !result;
		}

		i += 1;
		j = 1;
	}

	return result;
}
