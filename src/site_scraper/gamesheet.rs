use std::collections::HashMap;

use super::ScrapedGame;
use crate::client::HttpClient;
use crate::types::{SeasonID, SiteName};
use anyhow::{Result, anyhow};
use chrono::{DateTime, NaiveDateTime};
use serde::Deserialize;

use std::sync::Arc;
use strfmt::strfmt;

const URL: &str = "https://gateway.gamesheet.io/stats/schedule?filter[seasons]={season}&filter[start]={from_date}&filter[end]=2026-04-30&filter[teams]&filter[divisions]";
// 2025-09-01

pub struct Gamesheet {
    client: Arc<HttpClient>,
    api_key: String,
    seasons: HashMap<SiteName, SeasonID>,
    headers: http::HeaderMap,
}

#[derive(Debug, serde::Deserialize)]
struct Response {
    // status: String,
    data: Vec<Data>,
}

fn deserialize_naive_datetime<'de, D>(deserializer: D) -> Result<NaiveDateTime, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    // Try RFC3339 (with timezone) and convert to naive UTC
    if let Ok(dt) = DateTime::parse_from_rfc3339(&s) {
        return Ok(dt.naive_utc());
    }
    // Try plain NaiveDateTime with optional fractional seconds
    if let Ok(ndt) = NaiveDateTime::parse_from_str(&s, "%Y-%m-%dT%H:%M:%S%.f") {
        return Ok(ndt);
    }
    Err(serde::de::Error::custom(format!(
        "failed to parse datetime: {}",
        s
    )))
}

#[derive(Debug, Deserialize)]
struct Data {
    #[serde(deserialize_with = "deserialize_naive_datetime")]
    date: NaiveDateTime,
    games: Vec<GameJson>,
}

#[allow(non_snake_case)]
#[derive(Debug, Deserialize)]
struct GameJson {
    // id: i64,
    location: String,
    #[serde(deserialize_with = "deserialize_naive_datetime")]
    #[allow(non_snake_case)]
    scheduledStartTime: NaiveDateTime,
    #[allow(non_snake_case)]
    home: TeamJson,
    visitor: TeamJson,
}

#[derive(Debug, serde::Deserialize)]
struct TeamJson {
    title: String,
    division: DivisionJson,
}

#[derive(Debug, serde::Deserialize)]
struct DivisionJson {
    title: String,
}

impl Gamesheet {
    pub fn new(
        client: Arc<HttpClient>,
        api_key: String,
        seasons: HashMap<SiteName, SeasonID>,
    ) -> Self {
        let mut headers = http::HeaderMap::new();
        headers.insert(
            http::header::HeaderName::from_static("x-gamesheet-partner-apikey"),
            http::HeaderValue::from_str(&api_key).unwrap(),
        );

        Self {
            client,
            api_key,
            seasons,
            headers,
        }
    }

    pub async fn scrape_games(
        &self,
        from_date: String,
        site_name: &str,
    ) -> Result<Vec<ScrapedGame>> {
        let mut vars = std::collections::HashMap::new();
        vars.insert("from_date".to_string(), from_date);
        let season_id: String = self
            .seasons
            .get(&site_name.into())
            .ok_or(anyhow!("season not found"))?
            .to_string();

        vars.insert("season".to_string(), season_id);
        vars.insert("X-Gamesheet-Partner-ApiKey".into(), self.api_key.clone());

        let u = strfmt(URL, &vars)?;

        let contents = self
            .client
            .get_auto_redirect(&u, Some(self.headers.clone()))
            .await?;

        let resp: Response = serde_json::from_str(&contents).map_err(|err| {
            println!("{}", contents);
            err
        })?;

        let mut games: Vec<ScrapedGame> = Vec::new();
        for data in resp.data {
            println!("{}", data.date);
            for pgame in data.games {
                let g = ScrapedGame {
                    date: pgame.scheduledStartTime,
                    site_name: site_name.into(),
                    home_team: pgame.home.title,
                    away_team: pgame.visitor.title,
                    location: pgame.location,
                    division: pgame.home.division.title,
                    address_url: "".into(),
                    address: "".into(),
                };
                games.push(g);
            }
        }
        Ok(games)
    }
}
