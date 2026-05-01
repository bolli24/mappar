#[cfg(target_os = "windows")]
use crate::api::OAUTH_URL;
use crate::api::auth::OsmAuthToken;
use crate::api::user::UserDetails;
use crate::api::{API_URL, USER_AGENT};
use egui::mutex::RwLock;
use log::warn;
use reqwest::{Client, Method, Response, StatusCode};
use serde::Deserialize;
use std::sync::Arc;

#[derive(Debug, Default, Clone)]
pub struct AuthState {
	pub auth: Option<OsmAuthToken>,
	pub user: Option<UserDetails>,
}

#[derive(Clone)]
pub struct OsmClient {
	client: Client,
	auth_state: Arc<RwLock<AuthState>>,
	base_url: String,
	#[cfg(any(target_os = "windows", target_os = "linux"))]
	pub credential_entry: Arc<keyring_core::Entry>,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error("Reqwest error: {0}")]
	Reqwest(#[from] reqwest::Error),
	#[error("API error: {0} - {1}")]
	Api(StatusCode, String),
	#[error("XML error")]
	Xml,
	#[error("Deserialization error: {0}")]
	Deserialization(#[from] quick_xml::DeError),
	#[error("Can't get full element data for node.")]
	GetFullNode,
}

pub type ApiResult<T> = Result<T, Error>;

#[derive(Debug)]
pub struct Versions(Vec<String>);

impl OsmClient {
	pub fn new(auth: Option<OsmAuthToken>, user: Option<UserDetails>) -> Self {
		Self {
			client: Client::new(),
			auth_state: Arc::new(RwLock::new(AuthState { auth, user })),
			base_url: API_URL.to_owned(),
			#[cfg(any(target_os = "windows", target_os = "linux"))]
			credential_entry: Arc::new(credential_entry().expect("Error creating keyring entry.")),
		}
	}

	// Helper methods for auth state access
	pub fn auth(&self) -> Option<OsmAuthToken> {
		self.auth_state.read().auth.clone()
	}

	pub fn set_auth(&self, auth: Option<OsmAuthToken>) {
		self.auth_state.write().auth = auth;
	}

	pub fn user(&self) -> Option<UserDetails> {
		self.auth_state.read().user.clone()
	}

	pub fn set_user(&self, user: Option<UserDetails>) {
		self.auth_state.write().user = user;
	}

	pub fn logout(&self) {
		let mut state = self.auth_state.write();
		state.auth = None;
		state.user = None;
	}

	pub fn is_authenticated(&self) -> bool {
		self.auth_state.read().auth.is_some()
	}

	pub fn has_user(&self) -> bool {
		self.auth_state.read().user.is_some()
	}

	pub async fn call(&self, path: &str, body: Option<String>, method: Method) -> ApiResult<Response> {
		let url = if path.starts_with('/') {
			format!("{}{}", self.base_url, path)
		} else {
			format!("{}/{}", self.base_url, path)
		};

		// Clone auth token while holding lock briefly
		let auth_token = self.auth_state.read().auth.clone();

		// Build and execute request WITHOUT holding lock
		let mut request_builder = self.client.request(method, url).header("User-Agent", USER_AGENT);

		if let Some(auth) = &auth_token {
			request_builder = request_builder.header(reqwest::header::AUTHORIZATION, format!("Bearer {}", auth.0));
		}
		if let Some(body) = body {
			request_builder = request_builder.body(body)
		}

		let request = request_builder.build()?;
		let response = self.client.execute(request).await?;

		if response.status().is_success() {
			Ok(response)
		} else {
			let status = response.status();
			let text = response.text().await?;
			warn!("Failed to get data from the API: [{status}] {text}");
			Err(Error::Api(status, text))
		}
	}

	pub async fn get(&self, path: &str) -> ApiResult<Response> {
		self.call(path, None, Method::GET).await
	}

	pub async fn post(&self, path: &str, body: String) -> ApiResult<Response> {
		self.call(path, Some(body), Method::POST).await
	}

	pub async fn put(&self, path: &str, body: String) -> ApiResult<Response> {
		self.call(path, Some(body), Method::PUT).await
	}

	pub async fn get_versions(&self) -> ApiResult<Versions> {
		#[derive(Deserialize)]
		struct Osm {
			api: Api,
		}

		#[derive(Deserialize)]
		struct Api {
			version: Vec<String>,
		}

		let response = self.get("/api/versions").await?.text().await?;
		let osm: Osm = quick_xml::de::from_str(&response)?;
		Ok(Versions(osm.api.version))
	}
}

#[cfg(any(target_os = "windows", target_os = "linux"))]
fn credential_entry() -> keyring_core::Result<keyring_core::Entry> {
	init_credential_store()?;

	#[cfg(target_os = "windows")]
	{
		let modifiers = std::collections::HashMap::from([("target", OAUTH_URL)]);
		keyring_core::Entry::new_with_modifiers("mappar", "user", &modifiers)
	}

	#[cfg(target_os = "linux")]
	{
		keyring_core::Entry::new("mappar", "user")
	}
}

#[cfg(any(target_os = "windows", target_os = "linux"))]
fn init_credential_store() -> keyring_core::Result<()> {
	#[cfg(target_os = "windows")]
	keyring_core::set_default_store(windows_native_keyring_store::Store::new()?);

	#[cfg(target_os = "linux")]
	keyring_core::set_default_store(dbus_secret_service_keyring_store::Store::new()?);

	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;

	#[tokio::test]
	async fn get_versions() {
		let client = OsmClient::new(None, None);
		let versions = client.get_versions().await.unwrap();
		assert_eq!(versions.0[0], "0.6".to_owned());
		assert_eq!(versions.0.len(), 1);
		println!("{versions:?}");
	}
}
