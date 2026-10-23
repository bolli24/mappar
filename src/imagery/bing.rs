use reqwest::Client;
use serde::Deserialize;
use thiserror::Error;

const API_KEY: &str = env!("BING_API_KEY");

const METADATA_URL: &str =
	"https://dev.virtualearth.net/REST/v1/Imagery/Metadata/AerialOSM?include=ImageryProviders&uriScheme=https&key=";

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BingMetadataResponse {
	pub resource_sets: Vec<ResourceSet>,
	pub status_code: u16,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceSet {
	pub resources: Vec<ImageryResource>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageryResource {
	pub image_height: u32,
	pub image_url: String,
	pub image_url_subdomains: Vec<String>,
	pub image_width: u32,
	pub zoom_max: u32,
	pub zoom_min: u32,
}

pub struct BingMetadata {
	pub tile_url: String,
	pub zoom_min: u32,
	pub zoom_max: u32,
	pub tile_size: u32,
}

#[derive(Debug, Error)]
pub enum BingError {
	#[error("Bing metadata request failed: {0}")]
	Request(#[from] reqwest::Error),
	#[error("Bing metadata parse error: {0}")]
	Parse(#[from] serde_json::Error),
	#[error("Bing metadata response contained no resources")]
	EmptyResponse,
}

fn to_mappar_url(image_url: &str, subdomains: &[String]) -> String {
	let switch = format!("{{switch:{}}}", subdomains.join(","));
	image_url.replace("{subdomain}", &switch)
}

/// Fetches the Bing imagery metadata and returns the resolved tile URL template.
pub async fn bing_metadata(client: &Client) -> Result<BingMetadata, BingError> {
	let url = format!("{METADATA_URL}{API_KEY}");

	let text = client.get(&url).send().await?.error_for_status()?.text().await?;

	let metadata: BingMetadataResponse = serde_json::from_str(&text)?;

	let resource = metadata
		.resource_sets
		.first()
		.and_then(|rs| rs.resources.first())
		.ok_or(BingError::EmptyResponse)?;

	let tile_url = to_mappar_url(&resource.image_url, &resource.image_url_subdomains);

	Ok(BingMetadata {
		tile_url,
		zoom_min: resource.zoom_min,
		zoom_max: resource.zoom_max,
		tile_size: resource.image_height,
	})
}

pub fn tile_to_quadkey(x: u32, y: u32, zoom: u32) -> String {
	(0..zoom)
		.rev()
		.map(|i| {
			let mask = 1 << i;
			let digit = u8::from(y & mask != 0) << 1 | u8::from(x & mask != 0);
			(b'0' + digit) as char
		})
		.collect()
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn deserialize_metadata_response() {
		let json = r#"{
            "authenticationResultCode": "ValidCredentials",
            "brandLogoUri": "https://dev.virtualearth.net/Branding/logo_powered_by.png",
            "copyright": "Copyright",
            "resourceSets": [
                {
                    "estimatedTotal": 1,
                    "resources": [
                        {
                            "__type": "ImageryMetadata:http://schemas.microsoft.com/search/local/ws/rest/v1",
                            "imageHeight": 256,
                            "imageUrl": "https://ecn.{subdomain}.tiles.virtualearth.net/tiles/a{quadkey}.jpeg?g=1&pr=odbl",
                            "imageUrlSubdomains": ["t0", "t1", "t2", "t3"],
                            "imageWidth": 256,
                            "imageryProviders": [],
                            "vintageEnd": null,
                            "vintageStart": null,
                            "zoomMax": 21,
                            "zoomMin": 1
                        }
                    ]
                }
            ],
            "statusCode": 200,
            "statusDescription": "OK",
            "traceId": "8c199342b639b6e4806ca67edde2c005"
        }"#;

		let response: BingMetadataResponse = serde_json::from_str(json).unwrap();
		assert_eq!(response.status_code, 200);
		assert_eq!(response.resource_sets.len(), 1);

		let resource = &response.resource_sets[0].resources[0];
		assert_eq!(resource.image_height, 256);
		assert_eq!(resource.image_width, 256);
		assert_eq!(resource.zoom_min, 1);
		assert_eq!(resource.zoom_max, 21);
		assert_eq!(resource.image_url_subdomains, vec!["t0", "t1", "t2", "t3"]);
		assert!(resource.image_url.contains("{quadkey}"));
	}

	#[test]
	fn quadkey_conversion() {
		assert_eq!(tile_to_quadkey(3, 5, 3), "213");
		assert_eq!(tile_to_quadkey(0, 0, 1), "0");
		assert_eq!(tile_to_quadkey(1, 0, 1), "1");
		assert_eq!(tile_to_quadkey(0, 1, 1), "2");
		assert_eq!(tile_to_quadkey(1, 1, 1), "3");
		assert_eq!(tile_to_quadkey(0, 0, 0), "");
	}

	#[test]
	fn url_conversion() {
		let image_url = "https://ecn.{subdomain}.tiles.virtualearth.net/tiles/a{quadkey}.jpeg?g=1&pr=odbl";
		let subdomains = vec!["t0".into(), "t1".into(), "t2".into(), "t3".into()];
		let result = to_mappar_url(image_url, &subdomains);
		assert_eq!(
			result,
			"https://ecn.{switch:t0,t1,t2,t3}.tiles.virtualearth.net/tiles/a{quadkey}.jpeg?g=1&pr=odbl"
		);
	}
}
