use celes::Country;
use egui::{Pos2, Rect};
use glam::{DVec2, dvec2};

pub trait ToDVec2 {
	fn to_dvec2(self) -> DVec2;
}

pub trait ToPos2 {
	fn to_pos2(self) -> Pos2;
}

pub trait ToLyonPoint {
	fn to_lyon_point(self) -> lyon_tessellation::math::Point;
}

impl ToDVec2 for Pos2 {
	fn to_dvec2(self) -> DVec2 {
		dvec2(self.x as f64, self.y as f64)
	}
}

impl ToDVec2 for egui::Vec2 {
	fn to_dvec2(self) -> DVec2 {
		dvec2(self.x as f64, self.y as f64)
	}
}

impl ToPos2 for DVec2 {
	fn to_pos2(self) -> Pos2 {
		Pos2::new(self.x as f32, self.y as f32)
	}
}

impl ToLyonPoint for Pos2 {
	fn to_lyon_point(self) -> lyon_tessellation::math::Point {
		lyon_tessellation::math::Point::new(self.x, self.y)
	}
}

pub trait CountryFlag {
	fn flag(&self) -> Option<String>;
}

impl CountryFlag for Country {
	fn flag(&self) -> Option<String> {
		const DIFF: u32 = 127397;
		let mut code = self.alpha2.chars();
		let one = char::from_u32((code.next()? as u32) + DIFF)?;
		let two = char::from_u32((code.next()? as u32) + DIFF)?;
		Some(format!("{one}{two}"))
	}
}

pub trait ToRect {
	fn to_rect(self) -> Rect;
}

impl ToRect for (DVec2, DVec2) {
	fn to_rect(self) -> Rect {
		Rect::from_two_pos(self.0.to_pos2(), self.1.to_pos2())
	}
}

#[cfg(test)]
mod tests {
	use super::CountryFlag;
	use crate::init_test_logger;
	use celes::Country;
	use log::info;

	#[test]
	pub fn country() {
		init_test_logger();

		let us = Country::the_united_states_of_america();
		let flag = us.flag();

		info!("Flag: {flag:?}");
	}
}
