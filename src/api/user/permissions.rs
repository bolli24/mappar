use crate::api::client::{ApiResult, OsmClient};
use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Permission {
	ReadPrefs,
	WritePrefs,
	WriteDiary,
	WriteApi,
	WriteChangesetComments,
	WriteRedactions,
	ReadGpx,
	WriteGpx,
	WriteNotes,
	WriteBlocks,
	ConsumeMessages,
	SendMessages,
}

impl OsmClient {
	pub async fn get_user_permissions(&self) -> ApiResult<Vec<Permission>> {
		let response = self.get("/api/0.6/permissions").await?.text().await?;
		let osm: Osm = quick_xml::de::from_str(&response)?;

		let permissions = osm.permissions.permission.into_iter().map(|p| p.name).collect();

		Ok(permissions)
	}
}

impl<'de> Deserialize<'de> for Permission {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		let s = String::deserialize(deserializer)?;
		Ok(match s.as_str() {
			"allow_read_prefs" => Self::ReadPrefs,
			"allow_write_prefs" => Self::WritePrefs,
			"allow_write_diary" => Self::WriteDiary,
			"allow_write_api" => Self::WriteApi,
			"allow_write_changeset_comments" => Self::WriteChangesetComments,
			"allow_write_redactions" => Self::WriteRedactions,
			"allow_read_gpx" => Self::ReadGpx,
			"allow_write_gpx" => Self::WriteGpx,
			"allow_write_notes" => Self::WriteNotes,
			"allow_write_blocks" => Self::WriteBlocks,
			"allow_consume_messages" => Self::ConsumeMessages,
			"allow_send_messages" => Self::SendMessages,
			_ => return Err(serde::de::Error::custom(format!("unknown permission: {s}"))),
		})
	}
}

impl std::fmt::Display for Permission {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::ReadPrefs => write!(f, "allow_read_prefs"),
			Self::WritePrefs => write!(f, "allow_write_prefs"),
			Self::WriteDiary => write!(f, "allow_write_diary"),
			Self::WriteApi => write!(f, "allow_write_api"),
			Self::WriteChangesetComments => write!(f, "allow_write_changeset_comments"),
			Self::WriteRedactions => write!(f, "allow_write_redactions"),
			Self::ReadGpx => write!(f, "allow_read_gpx"),
			Self::WriteGpx => write!(f, "allow_write_gpx"),
			Self::WriteNotes => write!(f, "allow_write_notes"),
			Self::WriteBlocks => write!(f, "allow_write_blocks"),
			Self::ConsumeMessages => write!(f, "allow_consume_messages"),
			Self::SendMessages => write!(f, "allow_send_messages"),
		}
	}
}

#[derive(Deserialize)]
struct Osm {
	#[serde(rename = "@version")]
	version: String,
	#[serde(rename = "@generator")]
	generator: String,
	permissions: OsmPermissions,
}

#[derive(Deserialize)]
struct OsmPermissions {
	#[serde(default)]
	permission: Vec<PermissionField>,
}

#[derive(Deserialize)]
struct PermissionField {
	#[serde(rename = "@name")]
	name: Permission,
}

#[cfg(test)]
mod tests {
	use super::*;

	#[tokio::test]
	async fn get_permissions() {
		let client = OsmClient::new(None, None);
		let permissions = client.get_user_permissions().await.unwrap();
		println!("{}, {permissions:?}", permissions.len());
	}
}
