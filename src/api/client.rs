use crate::api::auth::OsmAuthToken;
use crate::api::user::UserDetails;
use crate::api::{API_URL, OAUTH_URL, USER_AGENT};
use log::warn;
use reqwest::{Client, Method, Response, StatusCode};
use serde::Deserialize;

pub struct OsmClient {
	client: Client,
	pub auth: Option<OsmAuthToken>,
	pub user: Option<UserDetails>,
	base_url: String,
	#[cfg(not(target_arch = "wasm32"))]
	pub credential_entry: keyring::Entry,
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
}

pub type ApiResult<T> = Result<T, Error>;

#[derive(Debug)]
pub struct Versions(Vec<String>);

impl OsmClient {
	pub fn new(auth: Option<OsmAuthToken>, user: Option<UserDetails>) -> Self {
		Self {
			client: Client::new(),
			auth,
			user,
			base_url: API_URL.to_owned(),
			#[cfg(not(target_arch = "wasm32"))]
			credential_entry: keyring::Entry::new_with_target(OAUTH_URL, "mappar", "user")
				.expect("Error creating keyring entry."),
		}
	}

	pub async fn call(&self, path: &str, body: Option<String>, method: Method) -> ApiResult<Response> {
		let url = if path.starts_with('/') {
			format!("{}{}", self.base_url, path)
		} else {
			format!("{}/{}", self.base_url, path)
		};
		let mut request_builder = self.client.request(method, url).header("User-Agent", USER_AGENT);

		if let Some(auth) = &self.auth {
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
