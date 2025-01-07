use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Imagery {
    pub entry: Vec<Entry>,
}

#[derive(Serialize, Deserialize)]
pub struct Entry {
    #[serde(rename = "@overlay")]
    pub overlay: Option<String>,
    #[serde(rename = "@eli-best")]
    pub eli_best: Option<String>,
    pub eula: Option<Eula>,
    #[serde(rename = "minimum-tile-expire")]
    pub minimum_tile_expire: Option<String>,
    pub mirror: Vec<Mirror>,
    #[serde(rename = "logo-url")]
    pub logo_url: Option<String>,
    #[serde(rename = "logo-image")]
    pub logo_image: Option<String>,
    #[serde(rename = "country-code")]
    pub country_code: Vec<String>,
    #[serde(rename = "default-layers")]
    pub default_layers: Option<DefaultLayers>,
    pub date: Vec<String>,
    #[serde(rename = "metadata-header")]
    pub metadata_header: Option<MetadataHeader>,
    #[serde(rename = "no-tile-checksum")]
    pub no_tile_checksum: Option<NoTileChecksum>,
    #[serde(rename = "no-tile-header")]
    pub no_tile_header: Option<NoTileHeader>,
    #[serde(rename = "terms-of-use-url")]
    pub terms_of_use_url: Option<String>,
    #[serde(rename = "terms-of-use-text")]
    pub terms_of_use_text: Option<String>,
    pub default: Option<String>,
    pub oldid: Vec<Oldid>,
    #[serde(rename = "mod-tile-features")]
    pub mod_tile_features: Option<String>,
    pub bounds: Option<Bounds>,
    #[serde(rename = "tile-size")]
    pub tile_size: Option<String>,
    #[serde(rename = "custom-http-header")]
    pub custom_http_header: Option<CustomHttpHeader>,
    #[serde(rename = "privacy-policy-url")]
    pub privacy_policy_url: Vec<String>,
    pub icon: Vec<String>,
    #[serde(rename = "attribution-url")]
    pub attribution_url: Option<String>,
    #[serde(rename = "attribution-text")]
    pub attribution_text: Vec<AttributionText>,
    pub name: Vec<Name>,
    pub id: String,
    pub category: Vec<String>,
    #[serde(rename = "type")]
    pub entry_type: String,
    pub url: String,
    pub description: Vec<Description>,
    #[serde(rename = "permission-ref")]
    pub permission_ref: Vec<String>,
    #[serde(rename = "valid-georeference")]
    pub valid_georeference: Option<String>,
    pub transparent: Option<String>,
    pub projections: Option<EntryProjections>,
    #[serde(rename = "max-zoom")]
    pub max_zoom: Option<String>,
    #[serde(rename = "min-zoom")]
    pub min_zoom: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct Eula {
    #[serde(rename = "@mandatory")]
    pub mandatory: Option<String>,
    #[serde(rename = "$text")]
    pub text: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct Mirror {
    #[serde(rename = "privacy-policy-url")]
    pub privacy_policy_url: Option<String>,
    pub id: Option<String>,
    pub projections: Option<MirrorProjections>,
    #[serde(rename = "type")]
    pub mirror_type: String,
    pub url: String,
    #[serde(rename = "max-zoom")]
    pub max_zoom: Option<String>,
    #[serde(rename = "min-zoom")]
    pub min_zoom: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct MirrorProjections {
    pub code: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct DefaultLayers {
    pub layer: Layer,
}

#[derive(Serialize, Deserialize)]
pub struct Layer {
    #[serde(rename = "@name")]
    pub name: String,
    #[serde(rename = "@style")]
    pub style: Option<String>,
    #[serde(rename = "@tile-matrix-set")]
    pub tile_matrix_set: String,
}

#[derive(Serialize, Deserialize)]
pub struct MetadataHeader {
    #[serde(rename = "@header-name")]
    pub header_name: String,
    #[serde(rename = "@metadata-key")]
    pub metadata_key: String,
}

#[derive(Serialize, Deserialize)]
pub struct NoTileChecksum {
    #[serde(rename = "@type")]
    pub no_tile_checksum_type: String,
    #[serde(rename = "@value")]
    pub value: String,
}

#[derive(Serialize, Deserialize)]
pub struct NoTileHeader {
    #[serde(rename = "@name")]
    pub name: String,
    #[serde(rename = "@value")]
    pub value: String,
}

#[derive(Serialize, Deserialize)]
pub struct Oldid {
    #[serde(rename = "@date")]
    pub date: String,
    #[serde(rename = "$text")]
    pub text: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct Bounds {
    #[serde(rename = "@max-lat")]
    pub max_lat: String,
    #[serde(rename = "@max-lon")]
    pub max_lon: String,
    #[serde(rename = "@min-lat")]
    pub min_lat: String,
    #[serde(rename = "@min-lon")]
    pub min_lon: String,
    pub shape: Vec<Shape>,
}

#[derive(Serialize, Deserialize)]
pub struct Shape {
    pub point: Vec<Point>,
}

#[derive(Serialize, Deserialize)]
pub struct Point {
    #[serde(rename = "@lat")]
    pub lat: String,
    #[serde(rename = "@lon")]
    pub lon: String,
}

#[derive(Serialize, Deserialize)]
pub struct CustomHttpHeader {
    #[serde(rename = "@header-name")]
    pub header_name: String,
    #[serde(rename = "@header-value")]
    pub header_value: String,
}

#[derive(Serialize, Deserialize)]
pub struct AttributionText {
    #[serde(rename = "@mandatory")]
    pub mandatory: Option<String>,
    #[serde(rename = "$text")]
    pub text: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct Name {
    #[serde(rename = "@lang")]
    pub lang: Option<String>,
    #[serde(rename = "$text")]
    pub text: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct Description {
    #[serde(rename = "@lang")]
    pub lang: String,
    #[serde(rename = "$text")]
    pub text: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct EntryProjections {
    pub code: Vec<String>,
}


