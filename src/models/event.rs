use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Full database row for the events table.
#[derive(Debug, Clone, FromRow)]
pub struct EventRow {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub event_type: String,
    pub start_date: DateTime<Utc>,
    pub end_date: Option<DateTime<Utc>>,
    pub timezone: String,
    pub venue_name: Option<String>,
    pub address: String,
    pub city: String,
    pub state: Option<String>,
    pub country: String,
    pub latitude: f64,
    pub longitude: f64,
    pub cost: Option<String>,
    pub url: Option<String>,
    pub submitted_by: String,
    pub status: String,
    pub reviewed_by: Option<String>,
    pub reviewed_at: Option<DateTime<Utc>>,
    pub rejection_reason: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// API response for a single event.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EventResponse {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub event_type: String,
    pub start_date: DateTime<Utc>,
    pub end_date: Option<DateTime<Utc>>,
    pub timezone: String,
    pub venue_name: Option<String>,
    pub address: String,
    pub city: String,
    pub state: Option<String>,
    pub country: String,
    pub latitude: f64,
    pub longitude: f64,
    pub cost: Option<String>,
    pub url: Option<String>,
    pub submitted_by: String,
    pub status: String,
    pub reviewed_by: Option<String>,
    pub reviewed_at: Option<DateTime<Utc>>,
    pub rejection_reason: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<EventRow> for EventResponse {
    fn from(e: EventRow) -> Self {
        Self {
            id: e.id,
            name: e.name,
            description: e.description,
            event_type: e.event_type,
            start_date: e.start_date,
            end_date: e.end_date,
            timezone: e.timezone,
            venue_name: e.venue_name,
            address: e.address,
            city: e.city,
            state: e.state,
            country: e.country,
            latitude: e.latitude,
            longitude: e.longitude,
            cost: e.cost,
            url: e.url,
            submitted_by: e.submitted_by,
            status: e.status,
            reviewed_by: e.reviewed_by,
            reviewed_at: e.reviewed_at,
            rejection_reason: e.rejection_reason,
            created_at: e.created_at,
            updated_at: e.updated_at,
        }
    }
}

/// List item returned from proximity and admin list queries.
#[derive(Debug, Serialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct EventListItem {
    pub id: Uuid,
    pub name: String,
    pub event_type: String,
    pub start_date: DateTime<Utc>,
    pub end_date: Option<DateTime<Utc>>,
    pub timezone: String,
    pub venue_name: Option<String>,
    pub city: String,
    pub state: Option<String>,
    pub country: String,
    pub latitude: f64,
    pub longitude: f64,
    pub cost: Option<String>,
    pub submitted_by: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub distance_meters: Option<f64>,
}

/// Request body for creating an event.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateEventRequest {
    pub name: String,
    pub description: Option<String>,
    pub event_type: String,
    pub start_date: DateTime<Utc>,
    pub end_date: Option<DateTime<Utc>>,
    pub timezone: String,
    pub venue_name: Option<String>,
    pub address: String,
    pub city: String,
    pub state: Option<String>,
    pub country: String,
    pub latitude: f64,
    pub longitude: f64,
    pub cost: Option<String>,
    pub url: Option<String>,
}

/// Request body for updating an event (user editing own event).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateEventRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub event_type: Option<String>,
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub timezone: Option<String>,
    pub venue_name: Option<String>,
    pub address: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub country: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub cost: Option<String>,
    pub url: Option<String>,
}

/// Request body for admin reviewing an event.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewEventRequest {
    pub action: String,
    pub reason: Option<String>,
}

/// Query params for listing events near a location.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListEventsQuery {
    pub lat: f64,
    pub lon: f64,
    pub radius_km: f64,
    pub event_type: Option<String>,
    pub from_date: Option<DateTime<Utc>>,
    pub to_date: Option<DateTime<Utc>>,
    pub include_past: Option<bool>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// Query params for admin event listing.
#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AdminListEventsQuery {
    pub status: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// Query params for "my events" listing.
#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct MyEventsQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// Submitter history stats for admin review.
#[derive(Debug, Serialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct SubmitterStats {
    pub total_submitted: i64,
    pub total_approved: i64,
    pub total_rejected: i64,
    pub total_pending: i64,
}
