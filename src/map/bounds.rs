use crate::LatLon;
use crate::map::latlon::latlon;
use std::fmt::Display;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Bounds {
	pub min: LatLon,
	pub max: LatLon,
}

impl Bounds {
	#[must_use]
	pub const fn center(&self) -> LatLon {
		latlon(self.top().midpoint(self.bottom()), self.left().midpoint(self.right()))
	}
	#[must_use]
	pub const fn left(&self) -> f64 {
		f64::min(self.min.lon, self.max.lon)
	}
	#[must_use]
	pub const fn right(&self) -> f64 {
		f64::max(self.min.lon, self.max.lon)
	}
	#[must_use]
	pub const fn top(&self) -> f64 {
		f64::max(self.min.lat, self.max.lat)
	}
	#[must_use]
	pub const fn bottom(&self) -> f64 {
		f64::min(self.min.lat, self.max.lat)
	}
	#[must_use]
	pub const fn top_left(&self) -> LatLon {
		latlon(self.top(), self.left())
	}
	#[must_use]
	pub const fn top_right(&self) -> LatLon {
		latlon(self.top(), self.right())
	}
	#[must_use]
	pub const fn bottom_left(&self) -> LatLon {
		latlon(self.bottom(), self.left())
	}
	#[must_use]
	pub const fn bottom_right(&self) -> LatLon {
		latlon(self.bottom(), self.right())
	}
}

impl Display for Bounds {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{},{},{},{}", self.left(), self.bottom(), self.right(), self.top())
	}
}
