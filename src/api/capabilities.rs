use crate::api::client::{ApiResult, OsmClient};
use regex::Regex;

use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct Capabilities {
	pub version_minimum: String,
	pub version_maximum: String,
	pub area_maximum: f64,
	pub note_area_maximum: f64,
	pub tracepoints_per_page: u32,
	pub way_nodes_maximum: u32,
	pub relation_members_maximum: u32,
	pub changesets_maximum_elements: u32,
	pub changesets_default_query_limit: u32,
	pub changesets_maximum_query_limit: u32,
	pub notes_default_query_limit: u32,
	pub notes_maximum_query_limit: u32,
	pub timeout_seconds: u32,
	pub status: Status,
	pub imagery_blacklist: Vec<Result<Regex, regex::Error>>,
}

impl OsmClient {
	pub async fn get_capabilities(&self) -> ApiResult<Capabilities> {
		let response = self.get("/api/capabilities").await?.text().await?;
		let osm: Osm = quick_xml::de::from_str(&response)?;

		let mut capabilities = Capabilities {
			version_minimum: osm.api.version.minimum,
			version_maximum: osm.api.version.maximum,
			area_maximum: osm.api.area.maximum,
			note_area_maximum: osm.api.note_area.maximum,
			tracepoints_per_page: osm.api.tracepoints.per_page,
			way_nodes_maximum: osm.api.waynodes.maximum,
			relation_members_maximum: osm.api.relationmembers.maximum,
			changesets_maximum_elements: osm.api.changesets.maximum_elements,
			changesets_default_query_limit: osm.api.changesets.default_query_limit,
			changesets_maximum_query_limit: osm.api.changesets.maximum_query_limit,
			notes_default_query_limit: osm.api.notes.default_query_limit,
			notes_maximum_query_limit: osm.api.notes.maximum_query_limit,
			timeout_seconds: osm.api.timeout.seconds,
			status: osm.api.status,
			imagery_blacklist: vec![],
		};

		if let Some(policy) = osm.policy {
			if let Some(imagery) = policy.imagery {
				capabilities.imagery_blacklist = imagery.blacklist.into_iter().map(|b| Regex::new(&b.regex)).collect();
			}
		}

		Ok(capabilities)
	}
}

#[derive(Deserialize)]
struct Osm {
	#[serde(rename = "api")]
	api: ApiCapabilities,
	#[serde(default)]
	policy: Option<Policy>,
}

#[derive(Debug, Deserialize)]
struct ApiCapabilities {
	version: VersionRange,
	area: MaxAttrF64,
	note_area: MaxAttrF64,
	tracepoints: PerPageAttr,
	waynodes: MaxAttrU32,
	relationmembers: MaxAttrU32,
	changesets: ChangesetsAttr,
	notes: QueryLimitAttr,
	timeout: TimeoutAttr,
	status: Status,
}

#[derive(Debug, Deserialize)]
struct VersionRange {
	#[serde(rename = "@minimum")]
	minimum: String,
	#[serde(rename = "@maximum")]
	maximum: String,
}

#[derive(Debug, Deserialize)]
struct MaxAttrF64 {
	#[serde(rename = "@maximum")]
	maximum: f64,
}

#[derive(Debug, Deserialize)]
struct MaxAttrU32 {
	#[serde(rename = "@maximum")]
	maximum: u32,
}

#[derive(Debug, Deserialize)]
struct PerPageAttr {
	#[serde(rename = "@per_page")]
	per_page: u32,
}

#[derive(Debug, Deserialize)]
struct QueryLimitAttr {
	#[serde(rename = "@default_query_limit")]
	default_query_limit: u32,
	#[serde(rename = "@maximum_query_limit")]
	maximum_query_limit: u32,
}

#[derive(Debug, Deserialize)]
struct ChangesetsAttr {
	#[serde(rename = "@maximum_elements")]
	maximum_elements: u32,
	#[serde(rename = "@default_query_limit")]
	default_query_limit: u32,
	#[serde(rename = "@maximum_query_limit")]
	maximum_query_limit: u32,
}

#[derive(Debug, Deserialize)]
struct TimeoutAttr {
	#[serde(rename = "@seconds")]
	seconds: u32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Status {
	#[serde(rename = "@database")]
	database: String,
	#[serde(rename = "@api")]
	api: String,
	#[serde(rename = "@gpx")]
	gpx: String,
}

#[derive(Debug, Deserialize)]
struct Policy {
	#[serde(default)]
	imagery: Option<Imagery>,
}

#[derive(Debug, Deserialize)]
struct Imagery {
	#[serde(default)]
	blacklist: Vec<Blacklist>,
}

#[derive(Debug, Deserialize)]
struct Blacklist {
	#[serde(rename = "@regex")]
	regex: String,
}

#[cfg(test)]
mod tests {
	use super::*;

	#[tokio::test]
	async fn get_capabilities() {
		let client = OsmClient::new(None, None);
		let capabilities = client.get_capabilities().await.unwrap();
		println!("{capabilities:?}");
	}
}
