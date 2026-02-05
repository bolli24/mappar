pub mod auth;
mod capabilities;
pub mod client;
mod full;
pub mod map;
pub mod user;

#[cfg(not(debug_assertions))]
pub const API_URL: &str = "https://api.openstreetmap.org";

#[cfg(debug_assertions)]
pub const API_URL: &str = "https://master.apis.dev.openstreetmap.org";

#[cfg(not(debug_assertions))]
pub const OAUTH_URL: &str = "https://www.openstreetmap.org";

#[cfg(debug_assertions)]
pub const OAUTH_URL: &str = API_URL;

pub const USER_AGENT: &str = concat!("mappar/", env!("CARGO_PKG_VERSION"));
