use config::Config;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[allow(unused)]
pub struct AppConfig {
    #[serde(rename = "DB_DSN")]
    pub db_dsn: String,

    #[serde(rename = "MAX_REQUESTS_PER_HOST")]
    pub max_requests_per_host: usize,

    #[serde(rename = "MAX_GLOBAL_REQUESTS")]
    pub max_global_requests: usize,

    #[serde(rename = "GAMESHEET_API_KEY")]
    pub gamesheet_api_key: String,

    #[serde(rename = "ADDRESS_CACHE_FILE")]
    pub address_cache_file: String,
}

pub fn load() -> AppConfig {
    let settings = Config::builder()
        .add_source(config::File::with_name("config.yaml"))
        .build()
        .unwrap();

    let cfg: AppConfig = settings.try_deserialize().unwrap();

    cfg
}
