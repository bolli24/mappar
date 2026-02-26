pub mod css;
mod draw_list;
pub mod map_memory;
pub mod map_state;
pub mod map_view;
mod polygon;
mod preview;
pub mod render;
mod style;
mod view;

use super::messages::Messages;
use crate::dbgtxt;
use crate::imagery::imagery_fetcher::{ImageryFetcher, Tile};
use crate::imagery::{Imagery, ImageryRef};
use crate::map::latlon::{CIRCUMFERENCE_EARTH, LatLon, latlon};
use crate::map::map_data::{EditHistory, MapData};
use crate::ui::debug_text::DebugText;
use crate::ui::input::KeyBinds;
use crate::util::convert::{ToDVec2, ToLyonPoint, ToPos2};
use crate::util::quick_set::QuickSet;
use css::Styles;
use derive_new::new;
use draw_list::DrawList;
use egui::emath::Rot2;
use egui::{
	Color32, Galley, Id, Mesh, Painter, Pos2, Rect, Response, StrokeKind, TextureId, TopBottomPanel, Vec2, Widget,
	pos2, vec2,
};
use epaint::mutex::Mutex;
use epaint::{CornerRadius, FontsView, PathShape, PathStroke, Shape, TextShape, Vertex};
use glam::{DVec2, UVec2, dvec2, uvec2};
use itertools::Itertools;
use lyon_tessellation::path::Path;
use lyon_tessellation::{
	BuffersBuilder, FillOptions, FillTessellator, FillVertex, FillVertexConstructor, VertexBuffers,
};
use map_memory::MapMemory;
use map_state::{EditMode, MapStateMachine, State as MapState};
use map_view::MapView;
use render::{DetailedElement, RenderElement};
use statig::prelude::StateMachine;
use std::cmp::Ordering;
use std::collections::HashSet;
use std::f64::consts::PI;
use std::ops::RangeInclusive;
use std::sync::Arc;
use style::TextPosition;
use view::View;

const ZOOM_SPEED: f64 = 5.0;

#[derive(new)]
pub struct MapWidget<'a> {
	map_data: &'a mut MapData,
	history: &'a mut EditHistory,
	styles: &'a mut Option<Styles>,
	imagery_fetcher: Arc<Mutex<ImageryFetcher>>,
	id: Id,
	debug_text: Option<&'a mut DebugText>,
	mem: &'a mut MapMemory,
	active_imagery_ref: Option<ImageryRef>,
	messages: Messages,
	map_state: &'a mut StateMachine<MapStateMachine>,
	update_edit_mode: Option<EditMode>,
	key_binds: &'a KeyBinds,
}

const UV: Rect = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));

impl Widget for MapWidget<'_> {
	#[allow(clippy::too_many_lines)]
	fn ui(mut self, ui: &mut egui::Ui) -> Response {
		// 1. Build MapView with reborrows and call show()
		let output = {
			MapView {
				map_data: Some(&*self.map_data),
				styles: &mut *self.styles,
				imagery_fetcher: self.imagery_fetcher.clone(),
				id: self.id,
				debug_text: self.debug_text.as_deref_mut(),
				mem: &mut *self.mem,
				active_imagery_ref: self.active_imagery_ref,
				messages: self.messages.clone(),
			}
			.show(ui)
		};
		// After this block, self.debug_text/mem/map_data/styles are available again

		// 2. Hover detection
		let mut hover_elements = {
			puffin::profile_scope!("Get hover elements");
			let single = match output.view.pointer_world_pos {
				Some(_) => closest_element(&output.render_data, output.view.scale).map(|e| e.id),
				_ => None,
			};
			QuickSet::from(single)
		};

		// 3. State machine dispatch
		let mut render_data = output.render_data;
		let mut context = map_state::Context {
			ui_context: ui.ctx(),
			response: output.response.clone(),
			map_data: self.map_data,
			history: self.history,
			view: &output.view,
			mem: self.mem,
			world_delta: output.world_delta,
			render_data: &mut render_data,
			hover_elements: &mut hover_elements,
			messages: self.messages.clone(),
			update_edit_mode: self.update_edit_mode,
			debug_text: self.debug_text.as_deref_mut(),
			key_binds: self.key_binds,
		};

		self.map_state.handle_with_context(&(), &mut context);
		dbgtxt!(self.debug_text, "MapState: {:?}", self.map_state.state());
		dbgtxt!(self.debug_text, "Selection Box: {:?}", self.map_state.selection_box);

		dbgtxt!(self.debug_text, "Hovered elements: {:?}", &hover_elements.len());
		let select_threshold = 5.0 * output.view.scale;
		dbgtxt!(self.debug_text, "Select threshold: {select_threshold:.3}");

		// 4. Element drawing with highlights
		let mut shapes = DrawList::new();
		let show_hover = matches!(
			self.map_state.state(),
			MapState::Select { .. } | MapState::BoxSelect { .. } | MapState::Draw { .. }
		);

		self.mem.highlighted_elements.extend(
			self.map_data
				.reference_index
				.relation_descendants(self.mem.highlighted_elements.iter().copied()),
		);

		let selected_children = self
			.map_data
			.reference_index
			.relation_descendants(self.mem.selection.elements.iter().copied());

		ui.ctx().fonts_mut(|fonts| {
			for element in &render_data {
				let hovered =
					hover_elements.contains(&element.id) || self.mem.highlighted_elements.contains(&element.id);
				let selected =
					self.mem.selection.elements.contains(&element.id) || selected_children.contains(&element.id);
				if let Some(hightlighted) = element.maybe_highlighted(hovered && show_hover, selected) {
					draw_element(&hightlighted, &mut shapes, &output.view, fonts);
				}
				draw_element(element, &mut shapes, &output.view, fonts);
			}

			for element in &self.map_state.preview.render_elements(self.map_data) {
				draw_element(element, &mut shapes, &output.view, fonts);
			}
		});

		dbgtxt!(self.debug_text, "Drawn Elements: {}", shapes.count());

		shapes.draw(&output.painter);

		// 5. Selection box
		if let Some((start, end)) = self.map_state.selection_box {
			let rect = Rect::from_two_pos(output.view.render_pos(&start), output.view.render_pos(&end));
			output.painter.rect(
				rect,
				0,
				Color32::WHITE.gamma_multiply(0.3),
				(2.0, Color32::DARK_GRAY),
				StrokeKind::Inside,
			);
		}

		// 6. Info bar
		TopBottomPanel::bottom("map_info_bar").show(ui.ctx(), |ui| {
			ui.monospace(format!(
				"{} nodes, {} ways, {} relations",
				self.mem.selection.node_count(),
				self.mem.selection.way_count(),
				self.mem.selection.relation_count()
			))
		});

		output.frame_response
	}
}

// TODO: maybe use egui's native svg rendering.
fn icon_shape(texture_id: TextureId, rect: Rect, angle: f32) -> Shape {
	let mut mesh = Mesh::with_texture(texture_id);
	mesh.add_rect_with_uv(rect, UV, Color32::WHITE);
	if angle != 0.0 {
		mesh.rotate(Rot2::from_angle(angle), rect.center());
	}
	Shape::mesh(mesh)
}

const MINIMUM_TRIANGLE_SIZE: f32 = 10.0; // Don't render very small triangles

#[inline]
fn draw_element(element: &DetailedElement, shapes: &mut DrawList, view: &View, fonts: &mut FontsView) {
	let color = element.style.color;
	let size = element.style.size;
	let text = &element.style.text;

	match &element.element {
		RenderElement::Node(pos) => {
			let render_pos = view.render_pos(pos);
			let rect = Rect::from_center_size(render_pos, Vec2::splat(size * view.ui_scale));
			if let Some(icon) = element.style.icon {
				shapes
					.images
					.push(icon_shape(icon.0, rect, element.style.icon_rotation));
			} else {
				shapes.nodes.push(Shape::rect_filled(rect, CornerRadius::ZERO, color));
			}

			if !text.text.is_empty() {
				shapes.text.push(Shape::text(
					fonts,
					render_pos + text.offset,
					text.anchor,
					&text.text,
					text.font_id(),
					text.color,
				));
			}
		}
		RenderElement::Path(nodes, is_closed) => {
			let vertices: Vec<Pos2> = nodes.iter().map(|pos| view.render_pos(pos)).collect();

			if !is_closed && text.position == TextPosition::Line && !text.text.is_empty() {
				draw_text_on_line(shapes, view, fonts, text, &vertices);
			} else if *is_closed && text.position == TextPosition::Center && !text.text.is_empty() {
				draw_text_in_area(shapes, fonts, text, &vertices);
			}

			let stroke = PathStroke::new(size * view.ui_scale, color);
			shapes.paths.push(Shape::Path(PathShape {
				points: vertices,
				stroke,
				fill: Color32::TRANSPARENT,
				closed: *is_closed,
			}));
		}
		RenderElement::Area(nodes) => {
			let mut buffers: VertexBuffers<Pos2, u16> = VertexBuffers::new();
			let mut vertex_builder = BuffersBuilder::new(&mut buffers, Ctor);
			let mut tessellator = FillTessellator::new();

			let mut path_builder = Path::builder();
			let (first, rest) = nodes.split_first().expect("There must be at least one node.");

			path_builder.begin(view.render_pos(first).to_lyon_point());

			for pos in rest.iter().map(|p| view.render_pos(p).to_lyon_point()) {
				path_builder.line_to(pos);
			}

			path_builder.end(true);

			let path = path_builder.build();

			{
				puffin::profile_scope!("Tessellate path.");
				tessellator
					.tessellate_path(&path, &FillOptions::default(), &mut vertex_builder)
					.expect("valid path geometry");
			}

			let mesh = Mesh {
				vertices: buffers
					.vertices
					.into_iter()
					.map(|pos| Vertex {
						pos,
						uv: epaint::WHITE_UV,
						color: element.style.fill_color,
					})
					.collect(),
				indices: buffers.indices.iter().map(|&i| u32::from(i)).collect(),
				texture_id: TextureId::default(),
			};

			if !mesh.is_empty() {
				shapes.areas.push(Shape::mesh(mesh));
			}

			let path_element = DetailedElement {
				element: RenderElement::Path(nodes.clone(), true),
				style: element.style.clone(),
				..*element
			};
			draw_element(&path_element, shapes, view, fonts);
		}
		RenderElement::Collection(elements) => {
			for e in elements {
				draw_element(e, shapes, view, fonts);
			}
		}
	}
}

#[inline]
fn triangle_area(triangle: &[Pos2]) -> f32 {
	debug_assert!(triangle.len() == 3);

	let a = triangle[0];
	let b = triangle[1];
	let c = triangle[2];

	0.5 * (c.x.mul_add(a.y - b.y, a.x.mul_add(b.y - c.y, b.x * (c.y - a.y))).abs())
}

fn draw_text_in_area(shapes: &mut DrawList, fonts: &mut FontsView, text: &style::Text, vertices: &[Pos2]) {
	let galley = fonts.layout_no_wrap(text.text.clone(), text.font_id(), text.color);
	let bounds = Rect::from_points(vertices);
	let pos = find_label_pos(bounds, &galley, vertices);

	if let Some(pos) = pos {
		let pos = pos - galley.size() * 0.5;
		shapes.text.push(Shape::Text(TextShape {
			pos,
			galley,
			underline: egui::Stroke::NONE,
			fallback_color: text.color,
			override_text_color: None,
			opacity_factor: 1.0,
			angle: 0.0,
		}));
	}
}

fn draw_text_on_line(shapes: &mut DrawList, view: &View, fonts: &mut FontsView, text: &style::Text, vertices: &[Pos2]) {
	use std::f32::consts::{FRAC_PI_2, PI};

	let galley = fonts.layout_no_wrap(text.text.clone(), text.font_id(), text.color);
	let len_sq = galley.size().x.powi(2);

	for (a, b) in vertices.iter().tuple_windows() {
		if a.distance_sq(*b) > len_sq {
			let vec = b.to_vec2() - a.to_vec2();
			let midpoint = *a + (vec * 0.5);

			if !view.viewport.contains(midpoint) {
				continue;
			}

			let normalized = vec.normalized();
			let mut angle = vec.angle();
			let mut flip = 0.5;

			if angle > FRAC_PI_2 {
				angle -= PI;
			} else if angle < -FRAC_PI_2 {
				angle += PI;
			} else {
				flip = -0.5;
			}
			let pos = midpoint + flip * (galley.size().x * normalized - galley.size().y * normalized.rot90());

			shapes.text.push(Shape::Text(TextShape {
				pos,
				galley: galley.clone(),
				underline: egui::Stroke::NONE,
				fallback_color: text.color,
				override_text_color: None,
				opacity_factor: 1.0,
				angle,
			}));
		}
	}
}

fn draw_tiles(
	painter: &Painter,
	imagery_ref: ImageryRef,
	imagery_fetcher: &Arc<Mutex<ImageryFetcher>>,
	view: &View,
	debug_text: &mut Option<&mut DebugText>,
) {
	#[cfg(not(target_arch = "wasm32"))]
	const ZOOM_ADJUST: f64 = 0.0;
	#[cfg(target_arch = "wasm32")]
	const ZOOM_ADJUST: f64 = -0.5;

	puffin::profile_function!();
	let mut imagery_fetcher = imagery_fetcher.lock();
	dbgtxt!(
		debug_text,
		"Local cached tiles: {}",
		imagery_fetcher.persistent_cached_count()
	);
	dbgtxt!(
		debug_text,
		"Memory cached tiles: {} (≈{}MiB)",
		imagery_fetcher.memory_cached_count(),
		imagery_fetcher.vram_used()
	);
	dbgtxt!(debug_text, "Pending tiles: {}", imagery_fetcher.pending_count());

	let corners = get_world_corners(view.viewport, view.camera_pos, view.scale);
	let imagery = imagery_fetcher
		.imageries()
		.write()
		.get(imagery_ref)
		.expect("Imagery reference should be valid.")
		.clone();

	let tile_zoom_level = u32::min(
		(view.zoom_level + (f64::from(view.ui_scale) - 1.0) + ZOOM_ADJUST).round() as u32,
		imagery.max_zoom(),
	);

	let tile_size = ((CIRCUMFERENCE_EARTH) / f64::exp2(f64::from(tile_zoom_level)) / 256.0)
		* f64::from(imagery.tile_size().unwrap_or(256));
	let (x_tiles, y_tiles) = tiles_in_rect(corners, tile_zoom_level);

	dbgtxt!(
		debug_text,
		"Tiles in View: {}",
		x_tiles.clone().count() * y_tiles.clone().count()
	);

	let mut drawn_tiles = HashSet::with_capacity(100);

	// TODO: don't fetch new tiles when moving fast

	for (x, y) in x_tiles.cartesian_product(y_tiles) {
		draw_tile(
			Tile::new(imagery_ref, (x, y, tile_zoom_level)),
			MIN_STEP,
			view,
			tile_size,
			&imagery,
			painter,
			&mut drawn_tiles,
			&mut imagery_fetcher,
		);
	}

	dbgtxt!(debug_text, "Drawn tiles: {}", drawn_tiles.len());
}

const MAX_STEP: i32 = 2;
const MIN_STEP: i32 = -MAX_STEP;
const RENDER_EXTRA_TILE: bool = true;

// TODO: use UV to render extra tiles so that they don't overlap
fn draw_tile(
	tile: Tile,
	extra_step: i32,
	view: &View,
	tile_size: f64,
	imagery: &Imagery,
	painter: &Painter,
	drawn_tiles: &mut HashSet<Tile>,
	imagery_fetcher: &mut ImageryFetcher,
) {
	puffin::profile_scope!("Draw Tile", format!("{:?}", tile.coords));
	if !drawn_tiles.insert(tile) {
		return;
	}
	let tile_texture = imagery_fetcher.get_tile(tile, extra_step == MIN_STEP);
	if let Some(image) = tile_texture {
		let coords = get_tile_pos(tile);
		let coords = (coords - (view.camera_pos * dvec2(1.0, -1.0))).to_pos2();
		let rect = Rect::from_min_size(coords, vec2(tile_size as f32, tile_size as f32)) / view.scale as f32;
		let rect = rect.translate(view.viewport.size() * 0.5);

		let tint = Color32::WHITE;

		#[cfg(any())]
		let tint = if extra_step == -1 {
			Color32::LIGHT_BLUE
		} else if extra_step == 0 {
			Color32::BLUE
		} else if extra_step == 2 {
			Color32::LIGHT_RED
		} else if extra_step == 3 {
			Color32::RED
		} else {
			Color32::WHITE
		};

		painter.image(image, rect, UV, tint);
		return;
	}

	if !RENDER_EXTRA_TILE {
		return;
	}

	if (MIN_STEP..0).contains(&extra_step) {
		let extra_step = extra_step + 1;
		if let Some(larger) = tile.get_larger(imagery.min_zoom) {
			draw_tile(
				larger,
				extra_step,
				view,
				tile_size * 2.0,
				imagery,
				painter,
				drawn_tiles,
				imagery_fetcher,
			);
		}
	} else if (0..MAX_STEP).contains(&extra_step) {
		// FIXME:
		let extra_step = extra_step + 1;
		let Some(smaller) = tile.get_smaller(imagery.max_zoom) else {
			return;
		};

		for tile in smaller {
			draw_tile(
				tile,
				extra_step,
				view,
				tile_size / 2.0,
				imagery,
				painter,
				drawn_tiles,
				imagery_fetcher,
			);
		}
	}
}

fn closest_element(render_data: &[DetailedElement], scale: f64) -> Option<&DetailedElement> {
	render_data
		.iter()
		.filter(|e| matches!(e.pointer_distance, Some(dist) if dist < e.pointer_threshold * scale))
		.sorted_by(|a, b| match (a, b) {
			(
				DetailedElement {
					pointer_distance: Some(_),
					element: RenderElement::Node(_),
					..
				},
				DetailedElement {
					pointer_distance: Some(_),
					element: RenderElement::Node(_),
					..
				},
			) => a.pointer_distance.partial_cmp(&b.pointer_distance).expect("NaN"),
			(
				DetailedElement {
					pointer_distance: Some(_),
					element: RenderElement::Node(_),
					..
				},
				_,
			) => Ordering::Less,
			(
				_,
				DetailedElement {
					pointer_distance: Some(_),
					element: RenderElement::Node(_),
					..
				},
			) => Ordering::Greater,
			_ => a.pointer_distance.partial_cmp(&b.pointer_distance).expect("NaN"),
		})
		.next()
}

fn find_label_pos(bounds: Rect, galley: &Galley, points: &[Pos2]) -> Option<Pos2> {
	let galley_size = galley.size();

	if galley_size.x > bounds.width() || galley_size.y > bounds.height() {
		return None;
	}

	let centered = Rect::from_center_size(bounds.center(), galley.size());

	if rect_inside_polygon(centered, points) {
		return Some(centered.center());
	}

	None
	// TODO: find better label position
}

fn get_tile(coords: DVec2, zoom_level: u32) -> UVec2 {
	const TILE_SIZE: f64 = 256.0;
	let lat_lon = LatLon::from_web_mercator(coords);
	let scale = 2.0f64.powi(zoom_level as i32);

	let mut siny = lat_lon.lat.to_radians().sin();

	siny = siny.clamp(-0.9999, 0.9999);
	let world_coords = dvec2(
		0.5 + lat_lon.lon / 360.0,
		0.5 - f64::ln((1.0 + siny) / (1.0 - siny)) / (4.0 * PI),
	);

	(world_coords * scale).floor().as_uvec2()
}

fn tiles_in_rect(corners: (DVec2, DVec2), zoom_level: u32) -> (RangeInclusive<u32>, RangeInclusive<u32>) {
	let tiles = (get_tile(corners.0, zoom_level), get_tile(corners.1, zoom_level));
	let min = uvec2(tiles.0.x.min(tiles.1.x), tiles.0.y.min(tiles.1.y));
	let max = uvec2(tiles.0.x.max(tiles.1.x), tiles.0.y.max(tiles.1.y));
	(min.x..=max.x, min.y..=max.y)
}

fn get_tile_pos(tile: Tile) -> DVec2 {
	let n = 2.0f64.powi(tile.coords.2.cast_signed());
	let lon_deg = (f64::from(tile.coords.0) / n).mul_add(360.0, -180.0);
	let lat_rad = (PI - 2.0 * PI * f64::from(tile.coords.1) / n).sinh().atan();

	latlon(-lat_rad.to_degrees(), lon_deg).to_web_mercator()
}

fn get_world_corners(viewport: Rect, camera_pos: DVec2, scale: f64) -> (DVec2, DVec2) {
	let half = ((viewport.min + viewport.size() * 0.5).to_dvec2()) * scale;
	(camera_pos - half, camera_pos + half)
}

fn view_to_world_pos(pos: Pos2, view: Rect, camera_pos: DVec2, scale: f64) -> DVec2 {
	let pos = pos - view.size() * 0.5;
	(dvec2(f64::from(pos.x), f64::from(-pos.y))) * scale + dvec2(camera_pos.x, camera_pos.y)
}

fn pointer_world_pos(camera_pos: DVec2, response: &Response, viewport: Rect, scale: f64) -> Option<DVec2> {
	if !response.contains_pointer() {
		return None;
	}
	response
		.ctx
		.input(|i| i.pointer.latest_pos())
		.map(|pos| view_to_world_pos(pos, viewport, camera_pos, scale))
}

fn get_tile_bound(view: Rect, camera_pos: DVec2, scale: f64) -> (DVec2, DVec2) {
	const EXPAND: f64 = 1.2; // Load some extra tiles
	let half =
		(dvec2(f64::from(view.width()) * 0.5, f64::from(view.height()) * 0.5) + view.min.to_dvec2()) * scale * EXPAND;
	(camera_pos - half, camera_pos + half)
}

/// tile width in meters at specified latitude and zoom level
#[must_use]
pub fn tile_width(zoom_level: u32, lat: f64) -> f64 {
	(CIRCUMFERENCE_EARTH * f64::cos(lat.to_radians())) / f64::from(zoom_level).exp2()
}

/// aka Resolution in meters per pixel
#[must_use]
pub fn scale(zoom_level: f64) -> f64 {
	(CIRCUMFERENCE_EARTH * f64::cos(0.0f64.to_radians())) / f64::exp2(zoom_level) / 256.0
}
#[must_use]
pub fn world_size(zoom_level: u32) -> f64 {
	2f64.powi(zoom_level as i32)
}

/// Compute if a point is inside a polygon
/// Only works with closed polygon i.e. points[0] = points[points.len() - 1]
#[must_use]
pub fn point_inside_polygon(p: Pos2, points: &[Pos2]) -> bool {
	debug_assert_eq!(points[0], points[points.len() - 1]);
	let mut wn = 0;
	for i in 0..points.len() - 1 {
		if points[i].y <= p.y {
			if points[i + 1].y > p.y && is_left(points[i], points[i + 1], p) {
				wn += 1;
			}
		} else if !is_left(points[i], points[i + 1], p) {
			wn -= 1;
		}
	}

	wn != 0
}

/// Compute if a point is inside a polygon
// TODO: Maybe check the other corners as well
#[must_use]
pub fn rect_inside_polygon(rect: Rect, points: &[Pos2]) -> bool {
	point_inside_polygon(rect.min, points) && point_inside_polygon(rect.max, points)
}

#[inline]
fn is_left(p0: Pos2, p1: Pos2, p2: Pos2) -> bool {
	let result = (p1.x - p0.x).mul_add(p2.y - p0.y, -((p2.x - p0.x) * (p1.y - p0.y)));
	debug_assert_ne!(result, 0.0);
	return result > 0.0;
}

struct Ctor;
impl FillVertexConstructor<Pos2> for Ctor {
	fn new_vertex(&mut self, vertex: FillVertex) -> Pos2 {
		let position = vertex.position();
		Pos2 {
			x: position.x,
			y: position.y,
		}
	}
}
