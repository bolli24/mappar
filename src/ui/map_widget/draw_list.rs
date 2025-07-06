use egui::{Painter, Shape};

#[derive(Default)]
pub struct DrawList {
	pub paths: Vec<Shape>,
	pub areas: Vec<Shape>,
	pub nodes: Vec<Shape>,
	pub images: Vec<Shape>,
	pub text: Vec<Shape>,
}

impl DrawList {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn draw(self, painter: &Painter) {
		puffin::profile_function!();
		painter.extend(self.areas);
		painter.extend(self.paths);
		painter.extend(self.nodes);
		painter.extend(self.images);
		painter.extend(self.text);
	}

	pub const fn count(&self) -> usize {
		self.paths.len() + self.areas.len() + self.nodes.len() + self.images.len() + self.text.len()
	}
}
