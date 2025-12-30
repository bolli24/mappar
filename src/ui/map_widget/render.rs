use super::View;
use super::css::Styles;
use super::css::context::StyleContext;
use super::style::Style;
use crate::map::map_data::{self, ElementId};
use crate::map::map_data::{Element, MapData, MapElement};
use crate::util::convert::ToPos2;
use egui::{Color32, Pos2, Rect, Vec2};
use glam::DVec2;
use itertools::Itertools;

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
	Area(Vec<DVec2>),
	Collection(Vec<DetailedElement>),
}

pub type RenderPolygon = Vec<DVec2>;

impl RenderElement {
	pub fn bounding_box(&self) -> Rect {
		match self {
			Self::Node(pos) => Rect::from_center_size(pos.to_pos2(), Vec2::splat(2.0)),
			Self::Path(nodes, _) => {
				assert!(!nodes.is_empty(), "Empty way");
				bounding_box_from_points(nodes)
			}
			Self::Area(perimeter) => bounding_box_from_points(perimeter),
			Self::Collection(elements) => elements.iter().map(|p| p.bounding_box).fold(Rect::NOTHING, Rect::union),
		}
	}

	pub fn distance(&self, pointer_pos: Option<DVec2>) -> Option<f64> {
		pointer_pos.map(|pointer_pos| match &self {
			Self::Node(pos) => (pointer_pos - *pos).length(),
			Self::Path(nodes, false) => nodes
				.iter()
				.tuple_windows()
				.map(|(a, b)| distance_point_to_segment(a, b, &pointer_pos))
				.fold(f64::INFINITY, f64::min),
			Self::Path(nodes, true) => Self::distance_to_loop(&pointer_pos, nodes),
			Self::Area(perimeter) => Self::distance_to_loop(&pointer_pos, perimeter),
			Self::Collection(elements) => elements.iter().fold(f64::INFINITY, |a, b| {
				a.min(b.pointer_distance.expect("No pointer distance"))
			}),
		})
	}

	pub fn in_box_selection(&self, rect: impl Into<Option<Rect>>) -> bool {
		let Some(rect) = rect.into() else {
			return false;
		};
		match &self {
			Self::Node(pos) => rect.contains(pos.to_pos2()),
			Self::Path(nodes, _) | Self::Area(nodes) => nodes.iter().all(|n| rect.contains(n.to_pos2())),
			Self::Collection(elements) => elements.iter().all(|e| e.element.in_box_selection(rect)),
		}
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
		styles: &mut Option<Styles>,
		view: &View,
		style_context: &StyleContext,
	) {
		// puffin::profile_function!(map_element.id.to_string());

		let style = styles
			.as_mut()
			.map(|styles| styles.get_style(map_element, style_context))
			.unwrap_or_default();

		let element = match &map_element.element {
			Element::Node { pos } => RenderElement::Node(pos.get().to_web_mercator()),
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

				// TODO: Area rendering
				let is_closed = map_data::is_closed(node_indices.as_slice());
				if style.fill_color != Color32::TRANSPARENT && is_closed {
					RenderElement::Area(nodes)
				} else {
					RenderElement::Path(nodes, is_closed)
				}
			}
			Element::Relation { .. } => {
				todo!()
			}
		};

		let bounding_box = element.bounding_box();
		if !bounding_box.intersects(view.world_view_rect) {
			return;
		}

		render_data.push(Self {
			id: map_element.id,
			bounding_box,
			pointer_distance: element.distance(view.pointer_world_pos),
			pointer_threshold: f64::from(style.size * 1.2),
			element,
			style,
		});
	}

	pub fn hovered(&self) -> Self {
		self.highlight(Color32::YELLOW)
	}

	pub fn selected(&self) -> Self {
		self.highlight(Color32::RED)
	}

	pub fn hovered_and_selected(&self) -> Self {
		self.highlight(Color32::ORANGE)
	}

	pub fn maybe_highlighted(&self, hovered: bool, selected: bool) -> Option<Self> {
		Some(match (hovered, selected) {
			(true, false) => self.hovered(),
			(false, true) => self.selected(),
			(true, true) => self.hovered_and_selected(),
			(false, false) => return None,
		})
	}

	fn highlight(&self, color: Color32) -> Self {
		let mut element = self.clone();
		element.style.color = color.gamma_multiply(0.75);
		element.style.size += 2.0;
		element.style.text.text = String::new();

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

// FIXME: make sure the added point is sufficiently far from the segment points
//
/// Return: the closest point that lies one of the segments of a polyline and that segment.
pub fn closest_point_on_polyline(
	points: &[DVec2],
	nodes: &[ElementId],
	p: DVec2,
) -> Option<(DVec2, ElementId, ElementId)> {
	debug_assert!(points.len() > 1, "Path must contain at least 2 points");

	let mut min_dist_sq = f64::MAX;

	let mut closest = (points[0], nodes[0], nodes[1]);

	for (i, (a, b)) in points.iter().tuple_windows().enumerate() {
		let current = closest_point_on_segment(*a, *b, p);
		let dist_sq = current.distance_squared(p);

		if dist_sq < min_dist_sq {
			min_dist_sq = dist_sq;
			closest = (current, nodes[i], nodes[i + 1]);
		}
	}

	Some(closest)
}

fn closest_point_on_segment(a: DVec2, b: DVec2, p: DVec2) -> DVec2 {
	let ab = b - a;
	let len_sq = ab.length_squared();

	if len_sq < f64::EPSILON {
		return a;
	}

	let ap = p - a;
	let t = (ap.dot(ab) / len_sq).clamp(0.0, 1.0);

	a + ab * t
}
