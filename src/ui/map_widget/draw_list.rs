use egui::{Painter, Shape};

pub struct DrawList {
	pub paths: Vec<Shape>,
	pub areas: Vec<Shape>,
	pub nodes: Vec<Shape>,
	pub images: Vec<Shape>,
}

impl DrawList {
	pub const fn new() -> Self {
		Self {
			paths: Vec::new(),
			areas: Vec::new(),
			nodes: Vec::new(),
			images: Vec::new(),
		}
	}

	pub fn draw(self, painter: &Painter) {
		puffin::profile_function!();
		painter.extend(self.areas);
		painter.extend(self.paths);
		painter.extend(self.nodes);
		painter.extend(self.images);
	}

	pub fn count(&self) -> usize {
		self.paths.len() + self.areas.len() + self.nodes.len() + self.images.len()
	}
}
