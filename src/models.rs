use diesel::{HasQuery, Insertable, Queryable, Selectable};
use serde::Deserialize;
use std::collections::hash_map::HashMap;
use std::fmt::Debug;

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

#[derive(Clone)]
pub struct SitesConfigM {
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
    pub parse_config_json: Option<ParserConfig>,
}

impl From<SitesConfig> for SitesConfigM {
    fn from(value: SitesConfig) -> Self {
        let mut pcc = None;
        if let Some(pc) = &value.parser_config {
            pcc = Some(serde_json::from_value(pc.clone()).unwrap());
        }
        Self {
            id: value.id,
            site_name: value.site_name.clone(),
            display_name: value.display_name.clone(),
            base_url: value.base_url.clone(),
            home_team: value.home_team.clone(),
            parser_type: value.parser_type.clone(),
            parser_config: value.parser_config.clone(),
            enabled: value.enabled.clone(),
            last_scraped_at: value.last_scraped_at.clone(),
            scrape_frequency_hours: value.scrape_frequency_hours.clone(),
            notes: value.notes.clone(),
            created_at: value.created_at.clone(),
            updated_at: value.updated_at.clone(),
            parse_config_json: pcc,
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct ParserConfig {
    pub game_type: Option<Vec<String>>,
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

#[derive(Debug, Insertable)]
#[diesel(table_name=crate::schema::events)]
#[diesel(check_for_backend(diesel::mysql::Mysql))]
pub struct InsertEvent {
    pub site: String,
    pub datetime: chrono::NaiveDateTime,
    pub home_team: String,
    pub guest_team: String,
    pub location: Option<String>,
    pub division: Option<String>,
    pub location_id: Option<i32>,
    pub surface_id: i32,
    // pub date_created: chrono::NaiveDateTime,
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

#[derive(Insertable, Queryable, Selectable, Clone)]
#[diesel(table_name=crate::schema::locations)]
#[diesel(check_for_backend(diesel::mysql::Mysql))]
pub struct Location {
    pub id: i32,
    pub address1: Option<String>,
    pub address2: Option<String>,
    pub city: Option<String>,
    pub name: String,
    pub uuid: Option<String>,
    pub recording_hours_local: Option<String>,
    pub postal_code: Option<String>,
    pub all_sheets_count: Option<i32>,
    pub longitude: Option<f32>,
    pub latitude: Option<f32>,
    pub logo_url: Option<String>,
    pub province_id: Option<i32>,
    pub venue_status: Option<String>,
    pub zone: Option<String>,
    pub total_surfaces: Option<i32>,
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

#[derive(Insertable, Queryable, Selectable)]
#[diesel(table_name=crate::schema::sites_locations)]
#[diesel(check_for_backend(diesel::mysql::Mysql))]
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

#[derive(HasQuery)]
#[diesel(table_name=crate::schema::sites_locations)]
#[diesel(check_for_backend(diesel::mysql::Mysql))]
pub struct SitesLoc {
    pub site: String,
    pub location: String,
    pub location_id: i32,
    pub surface_id: i32,
}

#[derive(HasQuery, Clone, Debug)]
#[diesel(table_name=crate::schema::surfaces)]
pub struct SurfaceQuery {
    pub id: i32,
    pub location_id: i32,
    pub name: String,
}

#[derive(HasQuery, Clone, Debug)]
#[diesel(table_name=crate::schema::gamesheet_seasons)]
pub struct SeasonsQuery {
    pub id: u32,
    pub title: Option<String>,
    pub site: String,
}
