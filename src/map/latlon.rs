use glam::DVec2;
use std::f64::consts::{FRAC_PI_2, FRAC_PI_4, PI};
use std::hash::{Hash, Hasher};
use std::ops::Add;

pub const RADIUS_EARTH: f64 = 6_378_137.0;
pub const CIRCUMFERENCE_EARTH: f64 = 2.0 * PI * RADIUS_EARTH;

#[derive(Copy, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct LatLon {
	#[serde(rename = "@lat")]
	pub lat: f64,
	#[serde(rename = "@lon")]
	pub lon: f64,
}

impl PartialEq for LatLon {
	fn eq(&self, other: &Self) -> bool {
		self.lat.to_bits() == other.lat.to_bits() && self.lon.to_bits() == other.lon.to_bits()
	}
}

impl Eq for LatLon {}

impl Hash for LatLon {
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.lat.to_bits().hash(state);
		self.lon.to_bits().hash(state);
	}
}

impl LatLon {
	#[inline]
	#[must_use]
	pub fn to_web_mercator(self) -> DVec2 {
		DVec2 {
			x: RADIUS_EARTH * self.lon.to_radians(),
			y: f64::ln(f64::tan(self.lat.to_radians().mul_add(0.5, FRAC_PI_4))) * RADIUS_EARTH,
		}
	}

	#[inline]
	#[must_use]
	pub fn from_web_mercator(pos: DVec2) -> Self {
		Self {
			lat: f64::atan(f64::exp(pos.y / RADIUS_EARTH))
				.mul_add(2.0, -FRAC_PI_2)
				.to_degrees(),
			lon: (pos.x / RADIUS_EARTH).to_degrees(),
		}
	}
}

#[must_use]
pub const fn latlon(lat: f64, lon: f64) -> LatLon {
	LatLon { lat, lon }
}

impl Add<DVec2> for LatLon {
	type Output = Self;

	fn add(self, rhs: DVec2) -> Self::Output {
		Self::from_web_mercator(self.to_web_mercator() + rhs)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use log::info;

	#[test]
	fn test_lat_lon() {
		crate::init_test_logger();
		let lat_lon = LatLon {
			lat: 51.82887,
			lon: 12.19731,
		};
		let web_mercator = lat_lon.to_web_mercator();
		info!("Web Mercator: {web_mercator:?}");
		let lat_lon2 = LatLon::from_web_mercator(web_mercator);
		let diff = (lat_lon.lat - lat_lon2.lat).abs() + (lat_lon.lon - lat_lon2.lon).abs();
		assert!(diff < 0.0001);
	}
}
