pub mod details;
pub mod permissions;
pub mod preferences;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserDetails {
	pub id: u32,
	pub display_name: String,
	pub terms_agreed: bool,
}
