use config::Config;
use serde::Deserialize;

#[derive(Debug,Deserialize)]
#[allow(unused)]
pub struct AppConfig {
    #[serde(rename = "DB_DSN")]
    pub db_dsn :String,

    #[serde(rename = "API_KEY")]
    api_key: String,
}

pub fn load() -> AppConfig {
    let settings = Config::builder().add_source(config::File::with_name("config.yaml")).build().unwrap();


    let cfg :AppConfig =  settings.try_deserialize().unwrap();

    cfg
}
