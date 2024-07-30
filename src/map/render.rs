use crate::map::map_data;
use crate::map::map_data::{Element, MapData, MapElement};
use crate::map::polygon::{Orientation, Polygon};
use crate::ui::map_widget::dvec2_to_pos;
use egui::{Color32, Pos2, Rect, Vec2};
use glam::DVec2;
use itertools::Itertools;

pub struct DetailedElement {
	pub element: RenderElement,
	pub style: Style,
	// TODO MAYBE differentiate between bounding box and distance to pointer
	pub bounding_box: Rect,
	pub is_area: bool,
}

pub enum RenderElement {
	Node(DVec2),
	Way(Vec<DVec2>, bool),
	Collection(Vec<DetailedElement>),
}

impl RenderElement {
	pub fn bounding_box(&self) -> Rect {
		match self {
			Self::Node(pos) => Rect::from_center_size(dvec2_to_pos(*pos), Vec2::splat(2.0)),
			Self::Way(nodes, _) => {
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
	size: 1.5,
	fill: false,
};

pub const DEFAULT_NODE_STYLE: Style = Style::Colored {
	color: Color32::BLACK,
	size: 4.0,
	fill: true,
};

impl DetailedElement {
	pub fn from_map_element(map_element: &MapElement, map_data: &MapData) -> Self {
		let element = match &map_element.element {
			Element::Node { pos } => RenderElement::Node(pos.to_web_mercator_render()),
			Element::Way { nodes: node_indices } => {
				// TODO: order nodes clockwise for filled rendering
				// but keep order counterclockwise for convex partitioning
				let nodes: Vec<DVec2> = node_indices[0..node_indices.len() - 1]
					.iter()
					.map(|n| {
						map_data
							.get_node_pos(*n)
							.unwrap_or_else(|| panic!("Couldn't get pos of node {n}"))
							.to_web_mercator_render()
					})
					.collect();

				let mut polygon = Polygon::new(nodes.clone(), false);
				let is_area = map_element.is_area();
				let is_closed = map_data::is_closed(node_indices.as_slice());
				let convex = if is_closed {
					polygon.set_orientation(Orientation::CCW);
					// polygon
					// 	.triangulate()
					// 	.map(|t| t.into_iter().map_into().collect::<Vec<Polygon>>())
					polygon.convex_partition()
				} else {
					None
				};

				if let Some(partitions) = convex {
					let mut partitions: Vec<_> = partitions
						.into_iter()
						.map(|p| {
							let element = RenderElement::Way(p.points, true);

							Self {
								bounding_box: element.bounding_box(),
								element,
								style: Style::Colored {
									color: Color32::LIGHT_RED,
									fill: false,
									size: 0.5,
								},
								is_area: false,
							}
						})
						.collect();

					let element = RenderElement::Way(nodes, is_closed);

					// partitions.push(Self {
					// 	bounding_box: element.bounding_box(),
					// 	element,
					// 	style: Style::Colored {
					// 		color: Color32::DARK_RED,
					// 		fill: false,
					// 		size: 1.5,
					// 	},
					// 	is_area,
					// });

					RenderElement::Collection(partitions)
				} else {
					RenderElement::Way(nodes, is_closed)
				}
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
			is_area: map_element.is_area(),
		}
	}

	pub fn hovered(&self, pointer_pos: DVec2) -> bool {
		match &self.element {
			RenderElement::Node(pos) => (pointer_pos - *pos).length() < 5.0,
			RenderElement::Way(nodes, _) => {
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
	result
}
