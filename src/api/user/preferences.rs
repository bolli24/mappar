use crate::api::client::{ApiResult, OsmClient};
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserPreferences(HashMap<String, String>);

impl OsmClient {
	/*
	<osm version="0.6" generator="OpenStreetMap server">
		<preferences>
			<preference k="somekey" v="somevalue" />
			<preference k="someotherkey" v="someothervalue" />
			...
		</preferences>
	</osm>
	 */
	pub async fn get_user_preferences(&self) -> ApiResult<UserPreferences> {
		let response = self.get("/api/0.6/user/preferences").await?.text().await?;
		let osm: Osm = quick_xml::de::from_str(&response)?;
		let preferences = osm
			.preferences
			.preference
			.into_iter()
			.map(|preference| (preference.key, preference.value))
			.collect();

		Ok(UserPreferences(preferences))
	}
}

#[derive(Deserialize)]
struct Osm {
	preferences: PreferencesField,
}

#[derive(Deserialize)]
struct PreferencesField {
	#[serde(rename = "preference")]
	#[serde(default)]
	preference: Vec<PreferenceEntry>,
}

#[derive(Deserialize)]
struct PreferenceEntry {
	#[serde(rename = "@k")]
	key: String,
	#[serde(rename = "@v")]
	value: String,
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::api::auth::OsmAuthToken;
	use dotenvy_macro::dotenv;

	#[tokio::test]
	async fn get_user_preferences() {
		let client = OsmClient::new(Some(OsmAuthToken(dotenv!("TEST_ACCOUNT_TOKEN").to_owned())), None);
		let user_preferences = client.get_user_preferences().await.unwrap();

		println!("{user_preferences:?}");
	}
}
