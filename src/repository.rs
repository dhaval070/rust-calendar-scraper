use diesel::sql_types::{Integer, VarChar};
use std::collections::HashMap;
use std::sync::LazyLock;

use crate::models;
use crate::models::SurfaceQuery;
use crate::schema;
use crate::types::{SeasonID, SiteName};

use anyhow::Result;
use diesel::prelude::*;
use diesel::{Connection, MysqlConnection};
use regex;

use diesel::r2d2::{ConnectionManager, Pool, R2D2Connection};

static RE: LazyLock<regex::Regex> = LazyLock::new(|| regex::Regex::new("[^a-zA-Z0-9]").unwrap());

#[cfg_attr(test, mockall::automock)]
pub trait RepositoryOps {
    fn import_locations(
        &self,
        _site_name: &str,
        locations: Vec<models::SitesLocation>,
    ) -> Result<()>;
    fn import_games(&self, games: Vec<models::InsertEvent>) -> Result<()>;
}

pub struct Repository<T>
where
    T: Connection + R2D2Connection + 'static,
{
    pool: Pool<ConnectionManager<T>>,
    locations: Vec<models::Location>,
    loc_map: HashMap<i32, models::Location>,
}

impl Repository<diesel::MysqlConnection> {
    pub fn new(dsn: &str) -> Self {
        let mgr = ConnectionManager::<MysqlConnection>::new(dsn);
        let pool = Pool::builder().build(mgr).expect("failed to connect db");
        let mut conn = pool.get().unwrap();
        let rows: Vec<models::Location> = schema::locations::table
            .load::<models::Location>(&mut conn)
            .unwrap();

        let mut loc_map = HashMap::new();
        for r in rows.iter() {
            loc_map.insert(r.id, r.clone());
        }

        Self {
            pool,
            locations: rows,
            loc_map,
        }
    }

    pub fn get_sites(&self, sites: Vec<&str>) -> Result<Vec<models::SitesConfigM>> {
        use schema::sites_config;

        let mut conn = self.pool.get()?;

        let res: Vec<models::SitesConfig> = if sites.len() == 1 && sites[0] == "all" {
            sites_config::table
                .filter(schema::sites_config::enabled.eq(true))
                .select(models::SitesConfig::as_select())
                .load(&mut conn)
                .unwrap()
        } else if sites.len() == 1 && sites[0] == "all-gamesheet" {
            sites_config::table
                .filter(schema::sites_config::enabled.eq(true))
                .filter(schema::sites_config::site_name.like("gs_%"))
                .select(models::SitesConfig::as_select())
                .load(&mut conn)
                .unwrap()
        } else {
            sites_config::table
                .filter(sites_config::site_name.eq_any(sites))
                .select(models::SitesConfig::as_select())
                .load(&mut conn)
                .unwrap()
        };
        let resm: Vec<models::SitesConfigM> = res.into_iter().map(|sc| sc.into()).collect();
        Ok(resm)
    }

    fn match_locations(&self, site: &str) -> Result<()> {
        let mut conn = self.pool.get()?;

        let queries = [
            r#"UPDATE
            sites_locations s,
            locations l,
            provinces p
        SET
            s.location_id = l.id,
            s.match_type='postal code'
        WHERE
            l.postal_code<>'' AND
            position(l.postal_code in s.address) AND
            p.id=l.province_id AND
            p.province_name="Ontario" AND
            s.site=? AND
            s.location_id=0"#,
            //----
            r#"UPDATE
			sites_locations s,
			locations l,
			provinces p
		SET
			s.location_id = l.id,
			s.match_type="partial"
		WHERE
			p.id=l.province_id AND
			p.province_name="Ontario" AND
			position(regexp_substr(address1, '^[a-zA-Z0-9]+ [a-zA-Z0-9]+') in s.address) AND
			position(left(l.postal_code,3) in s.address) AND
			site=? AND
			s.location_id=0"#,
            //----
            r#"UPDATE
			sites_locations s,
			locations l,
			provinces p
		SET
			s.location_id = l.id,
			s.match_type="partial"
		WHERE
			p.id=l.province_id AND
			p.province_name="Ontario" AND
			position(regexp_substr(address1, '^[a-zA-Z0-9]+ [a-zA-Z0-9]+') in s.address) AND
			position(left(l.postal_code,3) in s.address) AND
			site=? AND
			s.location_id=0"#,
            //----
            r#"UPDATE
			sites_locations s,
			locations l,
			provinces p
		SET
			s.location_id = l.id,
			s.match_type='address'
		WHERE
			position(regexp_substr(address1, '^[a-zA-Z0-9]+ [a-zA-Z0-9]+') IN s.address) AND
			p.id=l.province_id AND
			p.province_name="Ontario" AND
			s.site=? AND
			s.location_id=0 AND s.location_id != -1"#,
        ];

        conn.transaction(|conn| -> Result<usize> {
            for q in queries {
                diesel::sql_query(q)
                    .bind::<diesel::sql_types::VarChar, _>(site)
                    .execute(conn)?;
            }

            let queries = [
                r#"update sites_locations set surface=regexp_substr(location, '\\(.+\\)') where site=? AND (surface="" OR surface IS NULL)"#,
                r#"update sites_locations set surface=regexp_replace(surface, "\\(", '') where site=?"#,
                r#"update sites_locations set surface=regexp_replace(surface, '\\)', '') where site=?"#,
                r#"update sites_locations a, surfaces s set a.surface_id=s.id where s.location_id=a.location_id and position(a.surface in REPLACE(s.name,"\#", ""))<>0 and s.id is not null and a.surface<>"" and a.site=? and a.surface_id=0 and a.location_id > 0"#,
                r#"update sites_locations s, locations l, surfaces r set s.surface_id=r.id where s.location_id=l.id and r.location_id=s.location_id and l.total_surfaces=1 and s.surface_id=0 and s.site=?  and s.location_id > 0"#,
            ];

            for q in queries {
                diesel::sql_query(q)
                    .bind::<diesel::sql_types::VarChar, _>(site)
                    .execute(conn)?;
            }
            Ok(0)
        })?;
        Ok(())
    }

    fn match_gamesheet(&self, site: &str) -> Result<()> {
        let mut conn = self.pool.get()?;
        let unmatched_loc: Vec<models::SitesLocation>;
        use schema::sites_locations;

        unmatched_loc = sites_locations::table
            .filter(sites_locations::location_id.eq(0))
            .load::<models::SitesLocation>(&mut conn)?;

        let mut matched_loc: Vec<(&str, i32)> = Vec::with_capacity(unmatched_loc.iter().len());

        for site_loc in unmatched_loc.iter() {
            let mut best_len = 0;
            let mut best_match: i32 = 0;

            let sl = &site_loc.location;

            for loc in self.locations.iter() {
                if sl.contains(&loc.name) {
                    if sl.len() > best_len {
                        best_match = loc.id;
                        best_len = loc.name.len();
                    }
                }
            }
            if best_match > 0 {
                matched_loc.push((&sl, best_match));
            }
        }
        if matched_loc.len() > 0 {
            let query = r#"UPDATE sites_locations SET location_id=? WHERE site=? AND location=?"#;

            conn.transaction(|conn| -> Result<usize> {
                for m in matched_loc {
                    diesel::sql_query(query)
                        .bind::<diesel::sql_types::Integer, _>(m.1)
                        .bind::<diesel::sql_types::VarChar, _>(site)
                        .bind::<diesel::sql_types::VarChar, _>(m.0)
                        .execute(conn)
                        .or(Err(anyhow::anyhow!("failed to set location_id")))?;
                }
                Ok(0)
            })?;
        }

        let queries = [
            // set surface id if matched location has just 1 surface.
            r#"UPDATE sites_locations sl
			JOIN surfaces s ON sl.location_id = s.location_id AND s.deleted_at IS NULL
			JOIN (SELECT location_id FROM surfaces WHERE deleted_at IS NULL GROUP BY location_id HAVING COUNT(*) = 1) single
				ON sl.location_id = single.location_id
			SET sl.surface_id = s.id
			WHERE sl.site = ? AND sl.location_id > 0"#,
            // set surface id where remaining part of surface location matches with surface name
            r#"UPDATE sites_locations sl, locations l, surfaces s
			SET sl.surface_id=s.id
			WHERE
			sl.site=? AND sl.location_id=l.id AND s.location_id=l.id
			AND sl.surface_id=0 AND sl.location_id > 0
			AND s.deleted_at IS NULL
			AND locate(s.name, trim(replace(sl.location, l.name, '')))>0"#,
        ];

        conn.transaction(|conn| -> Result<usize> {
            for q in queries {
                diesel::sql_query(q)
                    .bind::<diesel::sql_types::VarChar, _>(site)
                    .execute(conn)?;
            }
            Ok(0 as usize)
        })?;

        let loc_surface = models::SurfaceQuery::query().load(&mut conn)?;

        let mut smap: HashMap<i32, Vec<models::SurfaceQuery>> = HashMap::new();

        for ls in loc_surface {
            let e = smap.entry(ls.location_id).or_insert(Vec::new());
            e.push(ls);
        }

        let mut site_loc = models::SitesLoc::query()
            .filter(schema::sites_locations::surface_id.eq(0))
            .load(&mut conn)?;

        for sl in site_loc.iter_mut() {
            if sl.location_id == 0 {
                let Some(loc_id) = self.match_location_by_tokens(sl) else {
                    continue;
                };
                diesel::sql_query(
                    r#"update sites_locations set location_id=? where site=? and location=?"#,
                )
                .bind::<Integer, _>(loc_id)
                .bind::<VarChar, _>(&sl.site)
                .bind::<VarChar, _>(&sl.location)
                .execute(&mut conn)?;
                sl.location_id = loc_id;
            }
            let Some(surface) = self.match_gameshet_surface(sl, &smap) else {
                continue;
            };

            diesel::sql_query(
                r#"update sites_locations set surface_id=? where site=? and location=?"#,
            )
            .bind::<Integer, _>(surface.id)
            .bind::<VarChar, _>(&sl.site)
            .bind::<VarChar, _>(&sl.location)
            .execute(&mut conn)?;
        }
        Ok(())
    }

    fn match_gameshet_surface<'a>(
        &self,
        sl: &mut models::SitesLoc,
        smap: &'a HashMap<i32, Vec<models::SurfaceQuery>>,
    ) -> Option<&'a models::SurfaceQuery> {
        let surfaces = smap.get(&sl.location_id)?;

        if surfaces.len() == 1 {
            return surfaces.get(0);
        }
        //
        let surface = self.match_surface_by_sanitized_name(sl, smap);

        if let None = surface {
            return self.match_by_last_word(sl, smap);
        };
        surface
    }

    fn match_surface_by_sanitized_name<'a>(
        &self,
        sl: &mut models::SitesLoc,
        smap: &'a HashMap<i32, Vec<models::SurfaceQuery>>,
    ) -> Option<&'a SurfaceQuery> {
        let loc = self.loc_map.get(&sl.location_id)?;

        let remaining = sl.location.replace(&loc.name, "");
        let sanitized = RE.replace(&remaining, "").to_lowercase();
        if sanitized.len() == 0 {
            return None;
        }

        for surface in smap.get(&sl.location_id)?.iter() {
            let s = RE.replace(&surface.name, "").to_string().to_lowercase();

            if s != "" && sanitized.contains(&s) {
                return Some(surface);
            }
        }
        None
    }

    fn match_by_last_word<'a>(
        &self,
        sl: &mut models::SitesLoc,
        smap: &'a HashMap<i32, Vec<models::SurfaceQuery>>,
    ) -> Option<&'a SurfaceQuery> {
        let last_word = sl.location.split_terminator(" ").last()?.to_lowercase();
        if last_word == "" {
            return None;
        }

        smap.get(&sl.location_id)?
            .iter()
            .find(|s| s.name.to_lowercase().contains(&last_word))
    }

    fn match_location_by_tokens(&self, sl: &mut models::SitesLoc) -> Option<i32> {
        'outer: for w in sl.location.split_terminator(" ") {
            let mut loc_id = 0;

            for loc in self.locations.iter() {
                if loc.name.contains(w) {
                    if loc_id != 0 {
                        // skip if matched multiple locations
                        continue 'outer;
                    }
                    loc_id = loc.id;
                }
            }
            if loc_id == 0 {
                continue;
            }
            return Some(loc_id);
        }
        None
    }

    pub fn gamesheet_season_map(&self) -> HashMap<SiteName, SeasonID> {
        let mut smap: HashMap<SiteName, SeasonID> = HashMap::new();
        let mut conn = self.pool.get().unwrap();
        let result = models::SeasonsQuery::query().load(&mut conn).unwrap();

        for r in result {
            smap.insert(r.site.as_str().into(), r.id.into());
        }
        smap
    }
}

impl RepositoryOps for Repository<diesel::MysqlConnection> {
    fn import_locations(
        &self,
        site_name: &str,
        locations: Vec<models::SitesLocation>,
    ) -> Result<()> {
        let mut conn = self.pool.get()?;

        println!("in import locations");
        diesel::insert_into(schema::sites_locations::table)
            .values(&locations)
            .on_conflict_do_nothing()
            .execute(&mut conn)?;

        match site_name {
            s if s.starts_with("gs_")
                || s == "rockieshockeyleague"
                || s == "allpeacehockey"
                || s == "cahlhockey"
                || s == "neahl" =>
            {
                self.match_gamesheet(site_name)
            }
            _ => self.match_locations(site_name),
        }
    }

    fn import_games(&self, games: Vec<models::InsertEvent>) -> Result<()> {
        let mut conn = self.pool.get()?;
        conn.transaction(|conn| {
            for chunk in games.chunks(1000) {
                diesel::insert_into(schema::events::table)
                    .values(chunk)
                    .on_conflict_do_nothing()
                    .execute(conn)?;
            }
            Ok(())
        })
    }
}
