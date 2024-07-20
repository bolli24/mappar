use egui::Color32;
use glam::DVec2;

use crate::map_data::{Element, MapData, MapElement};

pub type RenderPos = DVec2;

pub enum RenderElement {
	Node(RenderPos, Style),
	Way(Vec<RenderPos>, Style),
	Collection(Vec<RenderElement>, Style),
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

impl RenderElement {
	pub fn from_map_element(element: &MapElement, map_data: &MapData) -> Self {
		match &element.element {
			Element::Node { pos } => Self::Node(pos.to_web_mercator_render(), DEFAULT_NODE_STYLE),
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

				Self::Way(nodes, DEFAULT_WAY_STYLE)
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

	pub const fn style(&self) -> &Style {
		match self {
			Self::Way(_, style) | Self::Collection(_, style) | Self::Node(_, style) => style,
		}
	}
}
