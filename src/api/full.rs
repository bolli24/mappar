use crate::api::client::{ApiResult, Error, OsmClient};
use crate::api::map::parse_osm_xml;
use crate::map::map_data::MapData;
use crate::{ElementId, ElementType};
use log::info;
use web_time::Instant;

impl OsmClient {
	pub async fn get_full(&self, element_id: ElementId) -> ApiResult<MapData> {
		if element_id.1 == ElementType::Node {
			return Err(Error::GetFullNode);
		}

		let start = Instant::now();
		info!("Getting full element data for {}", element_id);
		let xml = self
			.get(&format!("/api/0.6/{}/{}/full", element_id.1, element_id.0))
			.await?
			.text()
			.await?;
		info!("Time taken: {:?}", start.elapsed());
		let start = Instant::now();

		let data = parse_osm_xml(&xml).ok_or(Error::Xml)?;

		#[cfg(all(not(target_arch = "wasm32"), debug_assertions))]
		std::fs::write("../api_data_full.xml", xml).expect("Error writing debug xml file");

		info!("Parsed data successfully. Took: {:?}", start.elapsed());

		Ok(data)
	}
}
