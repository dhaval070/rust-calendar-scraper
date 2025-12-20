use diesel::{Insertable, Queryable, Selectable};
use std::collections::hash_map::HashMap;

#[derive(Debug, Queryable, Selectable)]
#[diesel(table_name=crate::schema::sites_config)]
#[diesel(check_for_backend(diesel::mysql::Mysql))]
pub struct SitesConfig {
    pub id: i32,
    pub site_name: String,
    pub display_name: Option<String>,
    pub base_url: String,
    pub home_team: Option<String>,
    pub parser_type: String,
    pub parser_config: Option<serde_json::Value>,
    pub enabled: Option<bool>,
    pub last_scraped_at: Option<chrono::NaiveDateTime>,
    pub scrape_frequency_hours: Option<i32>,
    pub notes: Option<String>,
    pub created_at: Option<chrono::NaiveDateTime>,
    pub updated_at: Option<chrono::NaiveDateTime>,
}

#[derive(Insertable)]
#[diesel(table_name=crate::schema::sites_config)]
pub struct NewSitesConfig {
    pub site_name: String,
    pub display_name: Option<String>,
    pub base_url: String,
    pub home_team: Option<String>,
    pub parser_type: String,
    pub parser_config: Option<serde_json::Value>,
    pub enabled: Option<bool>,
    pub scrape_frequency_hours: Option<i32>,
    pub notes: Option<String>,
}

#[derive(Debug, Queryable, Selectable)]
#[diesel(table_name=crate::schema::events)]
#[diesel(check_for_backend(diesel::mysql::Mysql))]
pub struct Event {
    pub id: u64,
    pub site: String,
    pub source_type: Option<String>,
    pub datetime: chrono::NaiveDateTime,
    pub home_team: String,
    pub oid_home: Option<String>,
    pub guest_team: String,
    pub oid_guest: Option<String>,
    pub location: Option<String>,
    pub division: Option<String>,
    pub location_id: Option<i32>,
    pub surface_id: i32,
    pub date_created: chrono::NaiveDateTime,
}

#[derive(Debug, Queryable, Selectable)]
#[diesel(table_name=crate::schema::events)]
#[diesel(check_for_backend(diesel::mysql::Mysql))]
pub struct EventSmall {
    pub id: u64,
    pub site: String,
    pub datetime: chrono::NaiveDateTime,
    pub home_team: String,
}

#[derive(Insertable)]
#[diesel(table_name=crate::schema::events)]
pub struct NewEvent {
    pub site: String,
    pub source_type: Option<String>,
    pub datetime: chrono::NaiveDateTime,
    pub home_team: String,
    pub oid_home: Option<String>,
    pub guest_team: String,
    pub oid_guest: Option<String>,
    pub location: Option<String>,
    pub division: Option<String>,
    pub location_id: Option<i32>,
}

#[derive(Insertable)]
#[diesel(table_name=crate::schema::provinces)]
pub struct Provnice {
    pub id: i32,
    pub province_name: String,
    pub country: String,
}

#[derive(Insertable)]
#[diesel(table_name=crate::schema::locations)]
pub struct Location {
    pub id: i32,
    pub address1: String,
    pub address2: String,
    pub city: String,
    pub name: String,
    pub uuid: String,
    pub recording_hours_local: String,
    pub postal_code: String,
    pub all_sheets_count: i32,
    pub longitude: f32,
    pub latitude: f32,
    pub logo_url: String,
    pub province_id: Option<i32>,
    pub venue_status: String,
    pub zone: String,
    pub total_surfaces: i32,
    pub deleted_at: Option<chrono::NaiveDateTime>,
}

#[allow(non_snake_case)]
#[derive(Insertable)]
#[diesel(table_name=crate::schema::surfaces)]
pub struct Surface {
    pub id: i32,
    pub location_id: i32,
    pub name: String,
    pub uuid: String,
    pub orderIndex: i32,
    pub venue_id: i32,
    pub closed_from: u64,
    pub coming_soon: bool,
    pub online: bool,
    pub status: String,
    pub sports: String,
    pub first_media_date: u64,
}

#[allow(non_snake_case)]
#[derive(Debug, serde::Deserialize)]
pub struct LocationJson {
    pub id: i32,
    pub address1: Option<String>,
    pub address2: Option<String>,
    pub city: Option<String>,
    pub name: String,
    pub uuid: String,
    pub recording_hours_local: Option<String>,
    pub postalCode: Option<String>,
    pub all_sheets_count: Option<i32>,
    pub longitude: f32,
    pub latitude: f32,
    pub logo_url: Option<HashMap<String, String>>,
    pub province: ProvniceJson,
    pub venue_status: Option<VenueStatusJson>,
    pub surfaces: Vec<SurfaceJson>,
    pub zoneIds: Option<ZoneIds>,
}

#[derive(Debug, serde::Deserialize)]
pub struct ProvniceJson {
    pub id: i32,
    pub name: String,
    pub country: CountryJson,
}

#[derive(Debug, serde::Deserialize)]
pub struct CountryJson {
    pub id: i32,
    pub name: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct ZoneIds {
    pub name: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct VenueStatusJson {
    pub name: String,
}

#[allow(non_snake_case)]
#[derive(Debug, serde::Deserialize)]
pub struct SurfaceJson {
    pub id: i32,
    pub name: String,
    pub uuid: String,
    pub orderIndex: i32,
    pub venueId: i32,
    pub comingSoon: bool,
    pub closedFrom: Option<u64>,
    pub online: bool,
    pub surfaceStatus: NamedJson,
    pub sports: Vec<NamedJson>,
    pub firstMedia: Option<FirstMediaJson>,
    pub renditions: Vec<Rendition>,
}

#[derive(Debug, serde::Deserialize)]
pub struct NamedJson {
    pub name: String,
}
#[allow(non_snake_case)]
#[derive(Debug, serde::Deserialize)]
pub struct FirstMediaJson {
    pub firstMediaDate: u64,
}

#[derive(Debug, serde::Deserialize)]
pub struct Rendition {
    pub id: i32,
    pub name: String,
    pub width: i32,
    pub height: i32,
    pub ratio: String,
    pub bitrate: i64,
}

#[derive(Insertable)]
#[diesel(table_name=crate::schema::sites_locations)]
pub struct SitesLocation {
    pub site: String,
    pub location: String,
    pub location_id: i32,
    pub loc: Option<String>,
    pub surface: Option<String>,
    pub address: Option<String>,
    pub match_type: Option<String>,
    pub surface_id: i32,
}
