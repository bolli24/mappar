use egui::{Color32, Pos2, Rect, Vec2};
use glam::DVec2;
use itertools::Itertools;

use crate::css::Styles;
use crate::map::map_data;
use crate::map::map_data::{Element, MapData, MapElement};
use crate::map::polygon::{Orientation, Polygon};
use crate::map::style::Style;
use crate::ui::map_widget::dvec2_to_pos;

pub type ElementId = i64;

#[derive(Debug, Clone)]
pub struct DetailedElement {
	pub id: ElementId,
	pub pointer_distance: Option<f64>,
	pub pointer_threshold: f64,
	pub element: RenderElement,
	pub style: Style,
	pub bounding_box: Rect,
}

// TODO: maybe use SmallVec
#[derive(Debug, Clone)]
pub enum RenderElement {
	Node(DVec2),
	Path(Vec<DVec2>, bool),
	Area(Vec<DVec2>, Vec<RenderPolygon>),
	Collection(Vec<DetailedElement>),
}

pub type RenderPolygon = Vec<DVec2>;

impl RenderElement {
	pub fn bounding_box(&self) -> Rect {
		match self {
			Self::Node(pos) => Rect::from_center_size(dvec2_to_pos(*pos), Vec2::splat(2.0)),
			Self::Path(nodes, _) => {
				assert!(!nodes.is_empty(), "Empty way");
				bounding_box_from_points(nodes)
			}
			Self::Area(perimeter, _) => bounding_box_from_points(perimeter),
			Self::Collection(elements) => elements.iter().map(|p| p.bounding_box).fold(Rect::NOTHING, Rect::union),
		}
	}

	pub fn distance(&self, pointer_pos: Option<DVec2>) -> Option<f64> {
		puffin::profile_function!();
		pointer_pos.map(|pointer_pos| match &self {
			Self::Node(pos) => (pointer_pos - *pos).length(),
			Self::Path(nodes, false) => nodes
				.iter()
				.tuple_windows()
				.map(|(a, b)| distance_point_to_segment(a, b, &pointer_pos))
				.fold(f64::INFINITY, f64::min),
			Self::Path(nodes, true) => Self::distance_to_loop(&pointer_pos, nodes),
			Self::Area(_perimeter, _) => todo!(), // Self::distance_to_loop(&pointer_pos, perimeter),
			Self::Collection(elements) => elements.iter().fold(f64::INFINITY, |a, b| {
				a.min(b.pointer_distance.expect("No pointer distance"))
			}),
		})
	}

	fn distance_to_loop(pointer_pos: &DVec2, points: &[DVec2]) -> f64 {
		points
			.iter()
			.circular_tuple_windows()
			.map(|(a, b)| distance_point_to_segment(a, b, pointer_pos))
			.fold(f64::INFINITY, f64::min)
	}
}

impl DetailedElement {
	pub fn add_elements(
		render_data: &mut Vec<Self>,
		map_element: &MapElement,
		map_data: &MapData,
		styles: &mut Styles,
		pointer_pos: Option<DVec2>,
	) {
		puffin::profile_function!(map_element.id.to_string());

		let style = styles.get_style(map_element, map_data);

		let element = match &map_element.element {
			Element::Node { pos } => RenderElement::Node(pos.to_web_mercator()),
			Element::Way { nodes: node_indices } => {
				// TODO: order nodes clockwise for filled rendering
				// but keep order counterclockwise for convex partitioning
				let nodes: Vec<DVec2> = node_indices
					.iter()
					.map(|n| {
						map_data
							.get_node_pos(*n)
							.unwrap_or_else(|| panic!("Couldn't get pos of node {n}"))
							.to_web_mercator()
					})
					.collect();

				let mut polygon = Polygon::new(nodes.clone(), false);
				let is_area = map_element.is_area();

				let convex = if is_area {
					polygon.set_orientation(Orientation::CCW);
					// polygon.convex_partition()
					// TODO
					Some(())
				} else {
					None
				};

				if let Some(_partitions) = convex {
					let path = RenderElement::Path(nodes, true);
					// TODO improve area rendering
					// let area = RenderElement::Area(partitions.into_iter().map(|p| p.points).collect());

					RenderElement::Collection(vec![
						Self {
							id: map_element.id,
							bounding_box: path.bounding_box(),
							pointer_distance: path.distance(pointer_pos),
							pointer_threshold: f64::from(style.size),
							element: path,
							style,
						},
						// DetailedElement {
						// 	id: map_element.id,
						// 	bounding_box: area.bounding_box(),
						// 	element: area,
						// 	style: Style::Colored {
						// 		color: Color32::RED.gamma_multiply(0.5),
						// 		size: 1.5,
						// 		fill: true,
						// 	},
						// },
					])
				} else {
					let is_closed = map_data::is_closed(node_indices.as_slice());
					RenderElement::Path(nodes, is_closed)
				}
			}
			Element::Relation { .. } => {
				todo!()
				// let members = members
				// 	.iter()
				// 	.filter_map(|m| map_data.get_element(m.ref_id))
				// 	.map(|m| Self::from_map_element(m, map_data, pointer_pos))
				// 	.collect();
				// RenderElement::Collection(members)
			}
		};
		render_data.push(Self {
			id: map_element.id,
			bounding_box: element.bounding_box(),
			pointer_distance: element.distance(pointer_pos),
			pointer_threshold: f64::from(style.size),
			element,
			style,
		});
	}

	pub fn in_box_selection(&self, rect: Rect) -> bool {
		puffin::profile_function!();
		match &self.element {
			RenderElement::Node(pos) => rect.contains(dvec2_to_pos(*pos)),
			RenderElement::Path(nodes, _) | RenderElement::Area(nodes, _) => {
				nodes.iter().all(|n| rect.contains(dvec2_to_pos(*n)))
			}
			RenderElement::Collection(elements) => elements.iter().all(|e| e.in_box_selection(rect)),
		}
	}

	pub fn hovered(&self) -> Self {
		let mut element = self.clone();
		element.style.color = Color32::RED.gamma_multiply(0.75);
		element.style.size += 2.0;

		match &mut element.element {
			RenderElement::Node(_) => {
				element.style.icon = None;
			}
			RenderElement::Collection(elements) => {
				for sub_element in elements {
					*sub_element = sub_element.hovered();
				}
			}
			_ => (),
		}

		element
	}
}

/// **Adapted from**: <https://wrfranklin.org/Research/Short_Notes/pnpoly.html>
///
// int pnpoly(int nvert, float *vertx, float *verty, float testx, float testy)
// {
//   int i, j, c = 0;
//   for (i = 0, j = nvert-1; i < nvert; j = i++) {
//     if ( ((verty[i]>testy) != (verty[j]>testy)) &&
// 	 (testx < (vertx[j]-vertx[i]) * (testy-verty[i]) / (verty[j]-verty[i]) + vertx[i]) )
//        c = !c;
//   }
//   return c;
// }
pub fn point_in_polygon(test: DVec2, verts: &[DVec2]) -> bool {
	let mut result = false;
	let mut j = verts.len() - 1;

	for i in 0..verts.len() {
		if ((verts[i].y > test.y) != (verts[j].y > test.y))
			&& (test.x < (verts[j].x - verts[i].x) * (test.y - verts[i].y) / (verts[j].y - verts[i].y) + verts[i].x)
		{
			result = !result;
		}
		j = i;
	}
	result
}

fn bounding_box_from_points(points: &[DVec2]) -> Rect {
	let min_max_x = points.iter().map(|n| n.x as f32).minmax().into_option().unwrap();
	let min_max_y = points.iter().map(|n| n.y as f32).minmax().into_option().unwrap();

	Rect::from_min_max(Pos2::new(min_max_x.0, min_max_y.0), Pos2::new(min_max_x.1, min_max_y.1))
}

fn distance_point_to_segment(a: &DVec2, b: &DVec2, p: &DVec2) -> f64 {
	let ab = b - a;
	let ap = p - a;
	let ab_len_squared = ab.length_squared();

	if ab_len_squared == 0.0 {
		return (p - a).length();
	}

	let t = ap.dot(ab) / ab_len_squared;
	let t_clamped = t.clamp(0.0, 1.0);

	let closest_point = a + ab * t_clamped;
	(p - closest_point).length()
}
