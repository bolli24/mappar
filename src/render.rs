use egui::Vec2;
use skia_safe::Color;

use crate::map_data::{Element, MapData, MapElement};

pub enum RenderElement {
	Node(Vec2, Style),
	Way(Vec<Vec2>, Style),
	Collection(Vec<RenderElement>, Style),
}

pub enum Style {
	Colored { color: Color, size: f32 },
	Invisible,
}

pub const DEFAULT_STYLE: Style = Style::Colored {
	color: Color::BLACK,
	size: 1.0,
};

impl RenderElement {
	pub fn from_map_element(element: &MapElement, map_data: &MapData) -> Self {
		match &element.element {
			Element::Node { pos } => Self::Node(pos.to_web_mercator(), DEFAULT_STYLE),
			Element::Way { nodes } => {
				let nodes = nodes
					.iter()
					.map(|n| {
						map_data
							.get_node_pos(*n)
							.unwrap_or_else(|| panic!("Couldn't get pos of node {n}"))
							.to_web_mercator()
					})
					.collect();
				Self::Way(nodes, DEFAULT_STYLE)
			}
			Element::Relation { members } => {
				let members = members
					.iter()
					.filter_map(|m| map_data.get_element(m.ref_id))
					.map(|m| Self::from_map_element(m, map_data))
					.collect();
				Self::Collection(members, Style::Invisible)
			}
		}
	}
}
