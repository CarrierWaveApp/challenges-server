use serde::{Deserialize, Serialize};

// --- Query params ---

#[derive(Debug, Deserialize)]
pub struct BoundariesQuery {
    pub refs: Option<String>,
    pub bbox: Option<String>,
    pub limit: Option<i64>,
    pub simplify: Option<f64>,
}

// --- DB row types ---

#[derive(Debug, sqlx::FromRow)]
pub struct ParkBoundaryRow {
    pub pota_reference: String,
    pub park_name: String,
    pub designation: Option<String>,
    pub manager: Option<String>,
    pub acreage: Option<f64>,
    pub match_quality: String,
    pub source: String,
    pub geometry_json: Option<String>,
}

// --- API response types ---

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BoundaryFeature {
    #[serde(rename = "type")]
    pub feature_type: &'static str,
    pub geometry: serde_json::Value,
    pub properties: BoundaryProperties,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BoundaryProperties {
    pub reference: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub designation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manager: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub acreage: Option<f64>,
    pub match_quality: String,
    pub source: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BoundariesResponse {
    #[serde(rename = "type")]
    pub collection_type: &'static str,
    pub features: Vec<BoundaryFeature>,
    pub meta: BoundariesMeta,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BoundariesMeta {
    pub matched: usize,
    pub unmatched_refs: Vec<String>,
}

// --- ArcGIS API types ---

#[derive(Debug, Deserialize)]
pub struct ArcGisResponse {
    pub features: Option<Vec<ArcGisFeature>>,
}

#[derive(Debug, Deserialize)]
pub struct ArcGisFeature {
    pub attributes: Option<ArcGisAttributes>,
    pub geometry: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ArcGisAttributes {
    #[serde(alias = "Loc_Nm")]
    pub loc_nm: Option<String>,
    #[serde(alias = "Unit_Nm")]
    pub unit_nm: Option<String>,
    #[serde(alias = "Mang_Name")]
    pub mang_name: Option<String>,
    #[serde(alias = "Des_Tp")]
    pub des_tp: Option<String>,
    #[serde(alias = "GIS_Acres")]
    pub gis_acres: Option<f64>,
    #[serde(alias = "FeatClass")]
    pub feat_class: Option<String>,
}
