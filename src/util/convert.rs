use egui::Pos2;
use glam::{dvec2, DVec2};

pub trait ToDVec2 {
	fn to_dvec2(self) -> DVec2;
}

pub trait ToPos2 {
	fn to_pos2(self) -> Pos2;
}

impl ToDVec2 for Pos2 {
	fn to_dvec2(self) -> DVec2 {
		dvec2(self.x as f64, self.y as f64)
	}
}

impl ToPos2 for DVec2 {
	fn to_pos2(self) -> Pos2 {
		Pos2::new(self.x as f32, self.y as f32)
	}
}
