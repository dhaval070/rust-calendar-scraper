use crate::models;
use crate::schema;
use std::fs;

use diesel::{RunQueryDsl, prelude::MysqlConnection};

pub fn parse_file(path: &str) -> Vec<models::LocationJson> {
    let contents = fs::read_to_string(path).unwrap();

    // println!("{}", contents);
    let loc: Vec<models::LocationJson> = serde_json::from_str(&contents).unwrap();
    loc
}

pub fn import(conn: &mut MysqlConnection, locations: Vec<models::LocationJson>) {
    let mut loc_inserts: Vec<models::Location> = Vec::with_capacity(100);
    let mut provinces: Vec<models::Provnice> = Vec::new();
    let mut surfaces: Vec<models::Surface> = Vec::new();

    // let provinces :Vec<models::pro
    for loc in locations {
        provinces.push(models::Provnice {
            id: loc.province.id,
            province_name: loc.province.name,
            country: loc.province.country.name,
        });

        for s in loc.surfaces {
            surfaces.push(models::Surface {
                id: s.id,
                location_id: loc.id,
                name: loc.name.clone(),
                uuid: loc.uuid.clone(),
                orderIndex: s.orderIndex,
                venue_id: s.venueId,
                closed_from: s.closedFrom.unwrap_or(0),
                coming_soon: s.comingSoon,
                online: s.online,
                status: s.surfaceStatus.name,
                sports: s.sports.iter().map(|s| s.name.clone()).collect(),
                first_media_date: match s.firstMedia {
                    Some(m) => m.firstMediaDate,
                    _ => 0,
                },
            });
        }

        let vn = if let Some(st) = loc.venue_status {
            st.name
        } else {
            "".into()
        };

        let u: String = match loc.logo_url {
            Some(lu) => {
                let v: Vec<String> = lu.into_values().collect();
                v.join(",")
            }
            _ => "".into(),
        };
        let zones = match loc.zoneIds {
            Some(z) => z.name,
            _ => "".into(),
        };

        let loc = models::Location {
            id: loc.id,
            address1: loc.address1.unwrap_or("".into()),
            address2: loc.address2.unwrap_or("".into()),
            city: loc.city.unwrap_or("".into()),
            name: loc.name,
            uuid: loc.uuid,
            recording_hours_local: loc.recording_hours_local.unwrap_or("".into()),
            postal_code: loc.postalCode.unwrap_or("".into()),
            all_sheets_count: loc.all_sheets_count.unwrap_or(0),
            longitude: loc.longitude,
            latitude: loc.latitude,
            logo_url: u,
            province_id: Some(loc.province.id),
            venue_status: vn,
            zone: zones,
            total_surfaces: 0,
            deleted_at: None,
        };

        println!("{}", loc.id);

        loc_inserts.push(loc);
    }

    for c in provinces.chunks(100) {
        diesel::insert_into(schema::provinces::table)
            .values(c)
            .on_conflict_do_nothing()
            .execute(conn)
            .unwrap();
    }

    for c in loc_inserts.chunks(100) {
        diesel::insert_into(schema::locations::table)
            .values(c)
            .on_conflict_do_nothing()
            .execute(conn)
            .unwrap();
    }

    for c in surfaces.chunks(100) {
        diesel::insert_into(schema::surfaces::table)
            .values(c)
            .on_conflict_do_nothing()
            .execute(conn)
            .unwrap();
    }
}
