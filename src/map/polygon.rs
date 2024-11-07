use std::cell::{Cell, RefCell};
use std::fmt::Debug;

use egui::{Pos2, Rect};
use glam::DVec2;
use itertools::Itertools;

/// **Adapted From** <https://github.com/ivanfratric/polypartition/> (MIT Licence)
// TODO: Maybe make code more rusty

#[derive(Clone, Debug)]
pub struct Polygon {
	pub is_hole: bool,
	pub points: Vec<DVec2>,
}

#[derive(Clone)]
pub struct Multipolygon(Vec<Polygon>);

impl Multipolygon {
	pub fn bounding_box(&self) -> Rect {
		self.0
			.iter()
			.filter(|p| !p.is_hole)
			.map(|p| p.bounding_box())
			.fold(Rect::NOTHING, |acc, r| acc.union(r))
	}
}

#[derive(Clone, Copy, Debug)]
pub struct Triangle(pub DVec2, pub DVec2, pub DVec2);

#[derive(Clone)]
struct PartitionVertex<'a> {
	is_active: Cell<bool>,
	is_convex: Cell<bool>,
	is_ear: Cell<bool>,
	p: DVec2,
	angle: Cell<f64>,
	previous: RefCell<Option<&'a PartitionVertex<'a>>>,
	next: RefCell<Option<&'a PartitionVertex<'a>>>,
}

impl Debug for PartitionVertex<'_> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("PartitionVertex")
			.field("is_active", &self.is_active.get())
			.field("is_convex", &self.is_convex.get())
			.field("is_ear", &self.is_ear.get())
			.field("p", &self.p)
			.field("angle", &self.angle.get())
			.field("previous", &self.previous.borrow().map(|v| v.p))
			.field("previous", &self.previous.borrow().map(|v| v.p))
			.finish()
	}
}

impl Default for PartitionVertex<'_> {
	fn default() -> Self {
		Self {
			is_active: Cell::new(false),
			is_convex: Cell::new(false),
			is_ear: Cell::new(false),
			p: DVec2::ZERO,
			angle: Cell::new(0.0),
			previous: RefCell::new(None),
			next: RefCell::new(None),
		}
	}
}
impl From<Triangle> for Polygon {
	fn from(t: Triangle) -> Self {
		Polygon::new(vec![t.0, t.1, t.2], false)
	}
}

#[derive(Eq, PartialEq, Copy, Clone, Debug)]
pub enum Orientation {
	CW,
	CCW,
	None,
}

impl Polygon {
	pub const fn new(points: Vec<DVec2>, is_hole: bool) -> Self {
		Self { points, is_hole }
	}

	pub fn bounding_box(&self) -> Rect {
		let min_max_x = self.points.iter().map(|n| n.x as f32).minmax().into_option().unwrap();
		let min_max_y = self.points.iter().map(|n| n.y as f32).minmax().into_option().unwrap();

		Rect::from_min_max(Pos2::new(min_max_x.0, min_max_y.0), Pos2::new(min_max_x.1, min_max_y.1))
	}

	pub fn get_orientation(&self) -> Orientation {
		let mut area = 0.0;
		for (i, _) in self.points.iter().enumerate() {
			let j = if self.num_points() == i + 1 { 0 } else { i + 1 };
			area += self.points[i]
				.x
				.mul_add(self.points[j].y, -(self.points[i].y * self.points[j].x));
		}

		if area > 0.0 {
			Orientation::CCW
		} else if area < 0.0 {
			Orientation::CW
		} else {
			Orientation::None
		}
	}

	pub fn set_orientation(&mut self, orientation: Orientation) {
		let current = self.get_orientation();
		if current != Orientation::None && current != orientation {
			self.invert();
		}
	}

	pub fn invert(&mut self) {
		self.points.reverse()
	}

	pub fn is_valid(&self) -> bool {
		self.num_points() > 3
	}

	pub fn num_points(&self) -> usize {
		self.points.len()
	}

	#[allow(clippy::too_many_lines)]
	pub fn convex_partition(&self) -> Option<Vec<Self>> {
		puffin::profile_function!();
		if !self.is_valid() {
			return None;
		}

		let mut parts = Vec::new();

		if self.is_convex() {
			parts.push(self.clone());
			return Some(parts);
		}

		let mut triangles = self
			.triangulate()?
			.iter()
			.map(|t| Self::new(vec![t.0, t.1, t.2], false))
			.collect::<Vec<_>>();

		let mut iter1_outer = 0;
		while iter1_outer < triangles.len() {
			puffin::profile_scope!("main loop");
			let iter1 = iter1_outer;
			iter1_outer += 1;
			let mut iter2 = 0;
			let mut i11_outer = 0;
			while i11_outer < triangles[iter1].num_points() {
				i11_outer += 1;
				let i11 = i11_outer - 1;
				let i12 = (i11 + 1) % triangles[iter1].num_points();
				let d1 = triangles[iter1].points[i11];
				let d2 = triangles[iter1].points[i12];

				let mut is_diagonal = false;
				let mut i22 = 0;
				let mut i21 = 0;

				for iter_two in iter1..triangles.len() {
					if iter1 == iter_two {
						continue;
					}

					iter2 = iter_two;

					i21 = 0;
					while i21 < 3 {
						debug_assert!(triangles[iter2].num_points() == 3);
						let p21 = triangles[iter2].points[i21];

						if d2 != p21 {
							i21 += 1;
							continue;
						}

						if i21 == 2 {
							debug_assert!(i21 == 2);
							i22 = 0;
						} else {
							i22 = i21 + 1;
						}

						let p22 = triangles[iter2].points[i22];
						if d1 != p22 {
							i21 += 1;
							continue;
						}
						is_diagonal = true;
						break;
					}
					if is_diagonal {
						break;
					}
				}

				if !is_diagonal {
					continue;
				}

				let p2 = triangles[iter1].points[i11];
				let i13 = if i11 == 0 {
					triangles[iter1].num_points() - 1
				} else {
					i11 - 1
				};
				let p1 = triangles[iter1].points[i13];
				let i23 = if i22 == triangles[iter2].num_points() - 1 {
					0
				} else {
					i22 + 1
				};
				let p3 = triangles[iter2].points[i23];

				if !is_convex(p1, p2, p3) {
					continue;
				}

				let p2 = triangles[iter1].points[i12];
				let i13 = if i12 == triangles[iter1].num_points() - 1 {
					0
				} else {
					i12 + 1
				};
				let p3 = triangles[iter1].points[i13];
				let i23 = if i21 == 0 {
					triangles[iter2].num_points() - 1
				} else {
					i21 - 1
				};
				let p1 = triangles[iter2].points[i23];

				if !is_convex(p1, p2, p3) {
					continue;
				}

				{
					puffin::profile_scope!("New Polygon");
					let mut new_points =
						Vec::with_capacity(triangles[iter1].num_points() + triangles[iter2].num_points() - 2);
					let mut j = i12;
					while j != i11 {
						new_points.push(triangles[iter1].points[j]);
						j = (j + 1) % (triangles[iter1].num_points());
					}

					let mut j = i22;
					while j != i21 {
						new_points.push(triangles[iter2].points[j]);
						j = (j + 1) % (triangles[iter2].num_points());
					}

					triangles[iter1].points = new_points;
					triangles.remove(iter2);
				}

				i11_outer -= 1;
				continue;
			}
		}
		Some(triangles)
	}

	pub fn triangulate(&self) -> Option<Vec<Triangle>> {
		puffin::profile_function!();
		if !self.is_valid() {
			return None;
		}

		if self.num_points() == 3 {
			return Some(vec![Triangle(self.points[0], self.points[1], self.points[2])]);
		}

		let num_vertices = self.num_points();

		let vertices: Vec<PartitionVertex> = self
			.points
			.iter()
			.map(|p| PartitionVertex {
				p: *p,
				is_active: Cell::new(true),
				..Default::default()
			})
			.collect();

		for i in 0..num_vertices {
			if i == (num_vertices - 1) {
				*vertices[i].next.borrow_mut() = Some(&vertices[0]);
			} else {
				*vertices[i].next.borrow_mut() = Some(&vertices[i + 1]);
			}
			if i == 0 {
				*vertices[i].previous.borrow_mut() = Some(&vertices[num_vertices - 1]);
			} else {
				*vertices[i].previous.borrow_mut() = Some(&vertices[i - 1]);
			}
		}

		for i in 0..num_vertices {
			update_vertex(&vertices[i], vertices.as_slice());
		}

		let mut triangles = Vec::new();

		for i in 0..num_vertices - 3 {
			let mut ear: Option<&PartitionVertex> = None;
			for vj in &vertices {
				if !vj.is_active.get() || !vj.is_ear.get() {
					continue;
				}
				if ear.is_none() || vj.angle > ear.unwrap().angle {
					ear = Some(vj);
				}
			}

			let ear = ear?;

			triangles.push(Triangle(
				ear.previous.borrow().unwrap().p,
				ear.p,
				ear.next.borrow().unwrap().p,
			));

			ear.is_active.set(false);
			*ear.previous.borrow().unwrap().next.borrow_mut() = *ear.next.borrow();
			*ear.next.borrow().unwrap().previous.borrow_mut() = *ear.previous.borrow();

			if i == num_vertices - 4 {
				break;
			}

			update_vertex(ear.previous.borrow().unwrap(), vertices.as_slice());
			update_vertex(ear.next.borrow().unwrap(), vertices.as_slice());
		}

		for v in &vertices {
			if v.is_active.get() {
				triangles.push(Triangle(
					v.previous.borrow().unwrap().p,
					v.p,
					v.next.borrow().unwrap().p,
				));
				break;
			}
		}

		Some(triangles)
	}

	pub fn triangulate_multi(in_poly: &Multipolygon) -> Option<Vec<Triangle>> {
		let without_holes = Self::remove_holes(in_poly)?;

		let mut out_polys = Vec::new();

		for poly in without_holes {
			let mut result = poly.triangulate()?;
			out_polys.append(&mut result);
		}

		Some(out_polys)
	}

	pub fn convex_partion_multi(in_poly: &Multipolygon) -> Option<Vec<Self>> {
		let without_holes = Self::remove_holes(in_poly)?;
		let mut out_polys = Vec::new();

		for poly in without_holes {
			let mut result = poly.convex_partition()?;
			out_polys.append(&mut result);
		}

		Some(out_polys)
	}

	/// Returns the current polygon with all holes removed
	/// Returns None if the polygon has no holes
	#[allow(clippy::too_many_lines)]
	pub fn remove_holes(in_poly: &Multipolygon) -> Option<Vec<Self>> {
		let in_polys = &in_poly.0;
		if in_polys.is_empty() {
			return None;
		}

		if in_polys.iter().all(|p| !p.is_hole) {
			// Polygon has no holes
			return None;
		}

		let polys = in_polys;
		let mut out_polys = polys.clone();
		let mut hole_iter = polys.first().unwrap(); // Hole with the largest x
		let mut hole_iter_index = 0;
		let mut poly_iter = polys.first().unwrap();
		let mut poly_iter_index = 0;
		let mut hole_point_index = 0;
		let mut poly_point_index = 0;
		let mut best_poly_point = DVec2::ZERO;

		loop {
			let mut has_holes = false;

			for (index, iter) in polys.iter().enumerate() {
				if !iter.is_hole {
					continue;
				}

				if !has_holes {
					has_holes = true;
					hole_iter = iter;
					hole_iter_index = index;
					hole_point_index = 0;
				}

				for (i, (a, b)) in iter.points.iter().zip(hole_iter.points.iter()).enumerate() {
					if a.x > b.x {
						hole_iter = iter;
						hole_iter_index = index;
						hole_point_index = i;
					}
				}
			}

			if !has_holes {
				break;
			}

			let hole_point = hole_iter.points[hole_point_index];
			let mut point_found = false;

			for (index, iter) in polys.iter().enumerate() {
				if iter.is_hole {
					continue;
				}

				for (i, p) in iter.points.iter().enumerate() {
					if p.x <= hole_point.x {
						continue;
					}

					if !in_cone(
						iter.points[(i + iter.num_points() - 1) % (iter.num_points())],
						iter.points[i],
						iter.points[(i + 1) % iter.num_points()],
						hole_point,
					) {
						continue;
					}
					let poly_point = iter.points[i];
					if point_found {
						let v1 = (poly_point - hole_point).normalize();
						let v2 = (best_poly_point - hole_point).normalize();
						if v2.x > v1.x {
							continue;
						}
					}
					let mut point_visible = true;
					for iter2 in polys {
						if iter2.is_hole {
							continue;
						}
						for (i2, p2) in iter2.points.iter().enumerate() {
							let linep1 = *p2;
							let linep2 = iter2.points[(i2 + 1) % iter2.num_points()];
							if intersects((hole_point, poly_point), (linep1, linep2)) {
								point_visible = false;
								break;
							}
						}
						if !point_visible {
							break;
						}
					}
					if point_visible {
						point_found = true;
						best_poly_point = poly_point;
						poly_iter = iter;
						poly_iter_index = index;
						poly_point_index = i;
					}
				}
			}
			if !point_found {
				return None;
			}

			let mut new_poly = Self::new(Vec::new(), false);
			for i in 0..=poly_point_index {
				new_poly.points.push(poly_iter.points[i]);
			}
			for i in 0..=hole_iter.num_points() {
				new_poly
					.points
					.push(hole_iter.points[(i + hole_point_index) % (hole_iter.num_points())]);
			}
			for i in poly_point_index..poly_iter.num_points() {
				new_poly.points.push(poly_iter.points[i]);
			}

			out_polys.remove(hole_iter_index);
			out_polys.remove(poly_iter_index);
			out_polys.push(new_poly);
		}

		Some(out_polys)
	}

	fn is_convex(&self) -> bool {
		for i11 in 0..self.num_points() {
			let i12 = if i11 == 0 { self.num_points() - 1 } else { i11 - 1 };
			let i13 = if i11 == self.num_points() - 1 { 0 } else { i11 + 1 };

			if is_reflex(self.points[i12], self.points[i11], self.points[i13]) {
				return false;
			}
		}
		true
	}
}

fn in_cone(p1: DVec2, p2: DVec2, p3: DVec2, p: DVec2) -> bool {
	!is_convex(p1, p2, p3) || is_convex(p1, p2, p) || is_convex(p2, p3, p)
	// if is_convex(p1, p2, p3) {
	// 	is_convex(p1, p2, p) && is_convex(p2, p3, p)
	// } else {
	// 	is_convex(p1, p2, p) || is_convex(p2, p3, p)
	// }
}

fn is_convex(p1: DVec2, p2: DVec2, p3: DVec2) -> bool {
	(p3.y - p1.y).mul_add(p2.x - p1.x, -((p3.x - p1.x) * (p2.y - p1.y))) > 0.0
}

fn is_inside(p1: DVec2, p2: DVec2, p3: DVec2, p: DVec2) -> bool {
	!is_convex(p1, p, p2) && !is_convex(p2, p, p3) && !is_convex(p3, p, p1)
}

fn intersects(line1: (DVec2, DVec2), line2: (DVec2, DVec2)) -> bool {
	puffin::profile_function!();
	let (p11, p12) = line1;
	let (p21, p22) = line2;
	if p11 == p21 || p11 == p22 || p12 == p21 || p12 == p22 {
		return false;
	}

	let v1ort = DVec2::new(p12.y - p11.y, p11.x - p12.x);
	let v2ort = DVec2::new(p22.y - p21.y, p21.x - p22.x);

	let v = p21 - p11;
	let dot21 = v.x.mul_add(v1ort.x, v.y * v1ort.y);
	let v = p22 - p11;
	let dot22 = v.x.mul_add(v1ort.x, v.y * v1ort.y);

	if dot21 * dot22 > 0.0 {
		return false;
	}

	let v = p11 - p21;
	let dot11 = v.x.mul_add(v2ort.x, v.y * v2ort.y);
	let v = p12 - p21;
	let dot12 = v.x.mul_add(v2ort.x, v.y * v2ort.y);

	if dot11 * dot12 > 0.0 {
		return false;
	}

	true
}

fn is_reflex(p1: DVec2, p2: DVec2, p3: DVec2) -> bool {
	(p3.y - p1.y).mul_add(p2.x - p1.x, -((p3.x - p1.x) * (p2.y - p1.y))) < 0.0
}

fn update_vertex(v: &PartitionVertex, vertices: &[PartitionVertex]) {
	// puffin::profile_function!();

	let v1 = v.previous.borrow().unwrap();
	let v3 = v.next.borrow().unwrap();

	let is_convex = is_convex(v1.p, v.p, v3.p);
	v.is_convex.set(is_convex);

	let vec1 = (v1.p - v.p).normalize_or_zero();
	let vec3 = (v3.p - v.p).normalize_or_zero();
	v.angle.set(vec1.dot(vec3));

	v.is_ear.set(
		is_convex
			&& !vertices
				.iter()
				.any(|vi| vi.p != v.p && vi.p != v1.p && vi.p != v3.p && is_inside(v1.p, v.p, v3.p, vi.p)),
	);
}

#[cfg(test)]
mod tests {
	use glam::DVec2;

	use crate::map::polygon::Polygon;

	#[test]
	fn triangulate() {
		crate::init_test_logger();

		let points = vec![
			DVec2::new(9172.6, -9023.5),
			DVec2::new(9200.9, -9027.4),
			DVec2::new(9182.2, -8839.7),
			DVec2::new(9160.6, -8837.3),
			DVec2::new(9154.7, -8888.7),
			DVec2::new(9169.3, -8891.1),
			DVec2::new(9173.4, -8907.5),
			DVec2::new(9179.5, -8973.3),
			DVec2::new(9150.1, -9034.1),
			DVec2::new(9148.7, -9042.0),
			DVec2::new(9165.2, -9045.4),
		];

		let poly = Polygon::new(points, false);
		let triangles = poly.triangulate().unwrap();

		for t in triangles.iter() {
			print!("[x: {}, y: {}]\t\t", t.0.x, t.0.y);
			print!("[x: {}, y: {}]\t\t", t.0.x, t.0.y);
			print!("[x: {}, y: {}]\t\t", t.0.x, t.0.y);
			println!();
		}
	}

	#[test]
	fn convex_partition() {
		crate::init_test_logger();

		let points = vec![
			DVec2::new(9172.6, -9023.5),
			DVec2::new(9200.9, -9027.4),
			DVec2::new(9182.2, -8839.7),
			DVec2::new(9160.6, -8837.3),
			DVec2::new(9154.7, -8888.7),
			DVec2::new(9169.3, -8891.1),
			DVec2::new(9173.4, -8907.5),
			DVec2::new(9179.5, -8973.3),
			DVec2::new(9150.1, -9034.1),
			DVec2::new(9148.7, -9042.0),
			DVec2::new(9165.2, -9045.4),
		];

		let poly = Polygon::new(points, false);
		let partition = poly.convex_partition().unwrap();
	}

	#[test]
	fn orientation() {
		let points = vec![
			DVec2::new(0.0, 0.0),
			DVec2::new(1.0, 0.0),
			DVec2::new(1.0, 1.0),
			DVec2::new(0.0, 1.0),
		];

		let poly = Polygon::new(points, false);

		assert_eq!(poly.get_orientation(), crate::map::polygon::Orientation::CCW);
	}
}
