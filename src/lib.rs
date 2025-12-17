pub mod address_fetcher;
pub mod client;
pub mod config;
pub mod models;
pub mod repository;
pub mod schema;
pub mod site_scraper;
pub mod venue_import;
pub use repository::Repository;

use diesel::prelude::*;
use diesel::{Connection, MysqlConnection, mysql::Mysql};

use diesel::r2d2::{ConnectionManager, Pool, R2D2Connection};

pub fn connect_db(cfg: &config::AppConfig) -> MysqlConnection {
    MysqlConnection::establish(&cfg.db_dsn).unwrap()
}

pub fn get_db_pool<Dbconnection: R2D2Connection + 'static>(
    cfg: &config::AppConfig,
) -> Pool<ConnectionManager<Dbconnection>> {
    let mgr = ConnectionManager::<Dbconnection>::new(&cfg.db_dsn);
    Pool::builder().build(mgr).expect("failed to connect db")
}

pub fn insert_event(dbh: &mut MysqlConnection, e: &models::NewEvent) {
    diesel::insert_into(schema::events::table)
        .values(e)
        .execute(dbh)
        .unwrap();
}

pub fn get_sites<C>(conn: &mut C, sites: Vec<&str>) -> Vec<models::SitesConfig>
where
    C: Connection<Backend = Mysql> + diesel::connection::LoadConnection,
{
    use schema::sites_config;

    sites_config::table
        .filter(sites_config::site_name.eq_any(sites))
        .select(models::SitesConfig::as_select())
        .load(conn)
        .unwrap()
}
