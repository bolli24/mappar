use skia_safe::{Color, Paint, Path, Point};

use crate::map_data::{Element, MapData, MapElement};

pub enum RenderElement {
	Node(Point, Style),
	Way(Path, Style),
	Collection(Vec<RenderElement>, Style),
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Style {
	Colored { color: Color, size: f32, fill: bool },
	Invisible,
}

impl Style {
	pub fn get_paint(&self) -> Option<Paint> {
		match self {
			Style::Colored { color, size, fill } => {
				let mut paint = Paint::default();
				paint.set_color(*color);
				paint.set_stroke_width(*size);
				paint.set_stroke(!fill);
				paint.set_anti_alias(true);
				Some(paint)
			}
			Style::Invisible => None,
		}
	}
}

pub const DEFAULT_STYLE: Style = Style::Colored {
	color: Color::BLACK,
	size: 1.0,
	fill: false,
};

impl RenderElement {
	pub fn from_map_element(element: &MapElement, map_data: &MapData) -> Self {
		match &element.element {
			Element::Node { pos } => Self::Node(pos.to_web_mercator(), DEFAULT_STYLE),
			Element::Way { nodes } => {
				let nodes: Vec<Point> = nodes
					.iter()
					.map(|n| {
						map_data
							.get_node_pos(*n)
							.unwrap_or_else(|| panic!("Couldn't get pos of node {n}"))
							.to_web_mercator()
					})
					.collect();

				let mut path = Path::new();
				path.move_to(nodes[0]);

				for point in nodes.iter().skip(1) {
					path.line_to(*point);
				}

				Self::Way(path, DEFAULT_STYLE)
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

	pub fn style(&self) -> &Style {
		match self {
			RenderElement::Node(_, style) => style,
			RenderElement::Way(_, style) => style,
			RenderElement::Collection(_, style) => style,
		}
	}
}
