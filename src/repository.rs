use crate::models;
use crate::schema;

use anyhow::Result;
use diesel::prelude::*;
use diesel::{Connection, MysqlConnection};

use diesel::r2d2::{ConnectionManager, Pool, R2D2Connection};

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

    pub fn get_sites(&mut self, sites: Vec<&str>) -> Result<Vec<models::SitesConfig>> {
        use schema::sites_config;

        let mut conn = self.pool.get()?;

        if sites.len() == 1 && sites[0] == "all" {
            let res = sites_config::table
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
}
