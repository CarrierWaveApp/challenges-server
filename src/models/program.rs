use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::FromRow;

/// Database row for the programs table.
#[derive(Debug, Clone, FromRow)]
pub struct ProgramRow {
    pub slug: String,
    pub name: String,
    pub short_name: String,
    pub icon: String,
    pub icon_url: Option<String>,
    pub website: Option<String>,
    pub server_base_url: Option<String>,
    pub reference_label: String,
    pub reference_format: Option<String>,
    pub reference_example: Option<String>,
    pub multi_ref_allowed: bool,
    pub activation_threshold: Option<i32>,
    pub supports_rove: bool,
    pub capabilities: Vec<String>,
    pub adif_my_sig: Option<String>,
    pub adif_my_sig_info: Option<String>,
    pub adif_sig_field: Option<String>,
    pub adif_sig_info_field: Option<String>,
    pub data_entry_label: Option<String>,
    pub data_entry_placeholder: Option<String>,
    pub data_entry_format: Option<String>,
    pub sort_order: i32,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// API response for a single program (camelCase, matches iOS ActivityProgram).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgramResponse {
    pub slug: String,
    pub name: String,
    pub short_name: String,
    pub icon: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_url: Option<String>,
    pub website: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_base_url: Option<String>,
    pub reference_label: String,
    pub reference_format: Option<String>,
    pub reference_example: Option<String>,
    pub multi_ref_allowed: bool,
    pub activation_threshold: Option<i32>,
    pub supports_rove: bool,
    pub capabilities: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adif_fields: Option<AdifFieldMapping>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_entry: Option<DataEntryConfig>,
}

/// ADIF field mapping for programs that support ADIF export.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdifFieldMapping {
    pub my_sig: Option<String>,
    pub my_sig_info: Option<String>,
    pub sig_field: Option<String>,
    pub sig_info_field: Option<String>,
}

/// Data entry configuration for programs with the dataEntry capability.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DataEntryConfig {
    pub label: String,
    pub placeholder: Option<String>,
    pub format: Option<String>,
}

/// API response for GET /v1/programs.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgramListResponse {
    pub programs: Vec<ProgramResponse>,
    pub version: i64,
}

impl From<ProgramRow> for ProgramResponse {
    fn from(row: ProgramRow) -> Self {
        let adif_fields = if row.adif_my_sig.is_some()
            || row.adif_my_sig_info.is_some()
            || row.adif_sig_field.is_some()
            || row.adif_sig_info_field.is_some()
        {
            Some(AdifFieldMapping {
                my_sig: row.adif_my_sig,
                my_sig_info: row.adif_my_sig_info,
                sig_field: row.adif_sig_field,
                sig_info_field: row.adif_sig_info_field,
            })
        } else {
            None
        };

        let data_entry = row.data_entry_label.map(|label| DataEntryConfig {
            label,
            placeholder: row.data_entry_placeholder,
            format: row.data_entry_format,
        });

        Self {
            slug: row.slug,
            name: row.name,
            short_name: row.short_name,
            icon: row.icon,
            icon_url: row.icon_url,
            website: row.website,
            server_base_url: row.server_base_url,
            reference_label: row.reference_label,
            reference_format: row.reference_format,
            reference_example: row.reference_example,
            multi_ref_allowed: row.multi_ref_allowed,
            activation_threshold: row.activation_threshold,
            supports_rove: row.supports_rove,
            capabilities: row.capabilities,
            adif_fields,
            data_entry,
        }
    }
}
