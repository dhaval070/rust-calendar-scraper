use crate::models;
use crate::schema;

use anyhow::Result;
use diesel::prelude::*;
use diesel::{Connection, MysqlConnection};

use diesel::r2d2::{ConnectionManager, Pool, R2D2Connection};

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
}

impl Repository<diesel::MysqlConnection> {
    pub fn new(dsn: &str) -> Self {
        let mgr = ConnectionManager::<MysqlConnection>::new(dsn);
        Self {
            pool: Pool::builder().build(mgr).expect("failed to connect db"),
        }
    }

    pub fn get_sites(&self, sites: Vec<&str>) -> Result<Vec<models::SitesConfig>> {
        use schema::sites_config;

        let mut conn = self.pool.get()?;

        if sites.len() == 1 && sites[0] == "all" {
            let res = sites_config::table
                .filter(schema::sites_config::enabled.eq(true))
                .select(models::SitesConfig::as_select())
                .load(&mut conn)
                .unwrap();
            return Ok(res);
        }

        let res = sites_config::table
            .filter(sites_config::site_name.eq_any(sites))
            .select(models::SitesConfig::as_select())
            .load(&mut conn)
            .unwrap();
        Ok(res)
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

        println!("matching for site {}", site);

        for q in queries {
            let affected = diesel::sql_query(q)
                .bind::<diesel::sql_types::VarChar, _>(site)
                .execute(&mut conn)?;
            println!("Query affected {} rows", affected);
        }
        println!("ran match");

        let queries = [
            r#"update sites_locations set surface=regexp_substr(location, '\\(.+\\)') where site=? AND surface="""#,
            r#"update sites_locations set surface=regexp_replace(surface, "\\(", '') where site=?"#,
            r#"update sites_locations set surface=regexp_replace(surface, '\\)', '') where site=?"#,
            r#"update sites_locations a, surfaces s set a.surface_id=s.id where s.location_id=a.location_id and position(a.surface in REPLACE(s.name,"\#", ""))<>0 and s.id is not null and a.surface<>"" and a.site=? and a.surface_id=0 and a.location_id > 0"#,
            r#"update sites_locations s, locations l, surfaces r set s.surface_id=r.id where s.location_id=l.id and r.location_id=s.location_id and l.total_surfaces=1 and s.surface_id=0 and s.site=?  and s.location_id > 0"#,
        ];

        for q in queries {
            diesel::sql_query(q)
                .bind::<diesel::sql_types::VarChar, _>(site)
                .execute(&mut conn)?;
        }
        Ok(())
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

        self.match_locations(site_name)
    }

    fn import_games(&self, games: Vec<models::InsertEvent>) -> Result<()> {
        let mut conn = self.pool.get()?;
        diesel::insert_into(schema::events::table)
            .values(&games)
            .on_conflict_do_nothing()
            .execute(&mut conn)?;

        Ok(())
    }
}
