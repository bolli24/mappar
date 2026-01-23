use crate::api::client::{ApiResult, OsmClient};
use crate::api::user::UserDetails;
use serde::Deserialize;

impl OsmClient {
	pub async fn get_user_details(&self) -> ApiResult<UserDetails> {
		let response = self.get("/api/0.6/user/details").await?.text().await?;
		let osm: Osm = quick_xml::de::from_str(&response)?;

		Ok(UserDetails {
			id: osm.user.id,
			display_name: osm.user.display_name,
			terms_agreed: osm.user.contributor_terms.agreed,
		})
	}
}

#[derive(Deserialize)]
struct Osm {
	user: UserField,
}

#[derive(Deserialize)]
struct UserField {
	#[serde(rename = "@id")]
	id: u32,
	#[serde(rename = "@display_name")]
	display_name: String,
	#[serde(rename = "contributor-terms")]
	contributor_terms: ContributorTerms,
}

#[derive(Deserialize)]
struct ContributorTerms {
	#[serde(rename = "@agreed")]
	agreed: bool,
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::api::client::Error;
	use reqwest::StatusCode;

	#[tokio::test]
	async fn get_user_details() {
		let client = OsmClient::new(None, None);
		let auth_error = client.get_user_details().await.unwrap_err();

		assert!(match auth_error {
			Error::Api(StatusCode::UNAUTHORIZED, message) => {
				message == "Couldn't authenticate you"
			}
			_ => false,
		});
	}
}
