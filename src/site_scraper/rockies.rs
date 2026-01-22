use crate::site_scraper::ScrapedGame;
use crate::{client, models::SitesConfigM};
use anyhow::{Context, Result, anyhow};
use regex::Regex;
use scraper::{Html, Selector};
use serde::Deserialize;
use std::sync::{Arc, LazyLock};

// https://rockieshockeyleague.com/division/1724/15639/games
// GET https://rockieshockeyleague.com/api/leaguegame/get/2155/ 12605/ 1724/15639/  4553/     0/

static DIVISION_LINK_SELECTOR: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("h4.panel-title >a").unwrap());

static SEASON_SELECTOR: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse(r#"select[id="ddlSeason"] > option"#).unwrap());

static RE_PART: LazyLock<Regex> =
    LazyLock::new(|| regex::Regex::new("getMonthYears/([0-9]+)").unwrap());

static RE_YEARS: LazyLock<Regex> =
    LazyLock::new(|| regex::Regex::new(r#"(?<from>[0-9]+)\s*-\s*(?<to>[0-9]+)"#).unwrap());

static ADDRESS_SELECTOR: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("div.container-fluid > div > h3").unwrap());

#[allow(unused, non_snake_case)]
#[derive(Deserialize, Debug, Clone)]
struct Game {
    ArenaName: String,
    CategoryName: String,
    eDate: String,
    HomeDivision: String,
    HomeTeamName: String,
    AwayTeamName: String,
    Country: String,
    Prov: String,
    RARIDString: String,
    #[serde(skip)]
    Address: String,
    #[serde(skip)]
    naive_dt: chrono::NaiveDateTime,
}

impl From<&Game> for ScrapedGame {
    fn from(g: &Game) -> Self {
        ScrapedGame {
            date: g.naive_dt,
            site_name: "".into(),
            home_team: g.HomeTeamName.clone(),
            away_team: g.AwayTeamName.clone(),
            location: g.ArenaName.clone(),
            division: g.HomeDivision.clone(),
            address_url: "".into(),
            address: "".into(),
        }
    }
}

#[derive(Clone)]
pub struct Rockies {
    client: Arc<client::HttpClient>,
    sc: SitesConfigM,
}

impl Rockies {
    pub fn new(client: Arc<client::HttpClient>, sc: SitesConfigM) -> Self {
        Rockies { client, sc }
    }

    pub async fn get_games(&self, mm: String, yyyy: String) -> Result<Vec<ScrapedGame>> {
        eprintln!("{} {}", mm, yyyy);
        let resp = self
            .client
            .get_auto_redirect(&self.sc.base_url, None)
            .await?;

        let reg = Regex::new("[0-9]+")?;

        // scraper is not thread safe so have to parse in block thread
        let h = tokio::task::spawn_blocking(move || -> Result<Vec<(String, String)>> {
            let doc = Html::parse_document(&resp);

            let nodes = doc.select(&DIVISION_LINK_SELECTOR);
            let mut d: Vec<(String, String)> = Vec::new();

            for node in nodes {
                let parent = node
                    .attr("data-parent")
                    .ok_or(anyhow!("data-parent not found"))?;

                let href = node.attr("href").ok_or(anyhow!("href not found"))?;
                let parent = reg
                    .find(parent)
                    .iter()
                    .next()
                    .context(format!("re fail: {}", parent))?
                    .as_str();
                let child = reg
                    .find(href)
                    .iter()
                    .next()
                    .context(format!("re fail: {}", href))?
                    .as_str();
                d.push((parent.to_string(), child.to_string()));
            }
            Ok(d)
        });
        let mut pc_jobs = tokio::task::JoinSet::new();

        let d = h.await??;

        for data in d {
            let y = yyyy.clone();
            let s = self.clone();
            pc_jobs.spawn(async move { s.get_season_games(data.0, data.1, y).await });
        }
        let mut all_games: Vec<ScrapedGame> = Vec::new();

        while let Some(res) = pc_jobs.join_next().await {
            match res {
                Ok(Ok(games)) => {
                    let mut ag: Vec<ScrapedGame> = games.iter().map(|g| g.into()).collect();
                    all_games.append(&mut ag);
                }
                Ok(Err(e)) => eprintln!("task error {:?}", e),
                Err(e) => eprintln!("join error {}", e),
            }
        }

        Ok(all_games)
    }

    async fn get_season_games(
        &self,
        parent: String,
        child: String,
        yyyy: String,
    ) -> Result<Vec<Game>> {
        let url = self.sc.base_url.to_owned() + "/division/" + &parent + "/" + &child + "/games";

        let resp = self
            .client
            .get_auto_redirect(&url, None)
            .await
            .context("http err")?;

        let cap = RE_PART
            .captures(&resp)
            .ok_or(anyhow!("part capture failed"))
            .context("capture err")?;
        let part = &cap[1];

        let season = scrape_season(&resp, &yyyy)?;
        if season == "" {
            eprintln!("season for given year not found");
            return Ok(Vec::new());
        }

        let mut all_games: Vec<Game> = Vec::new();
        let mut jobs: tokio::task::JoinSet<Result<Game>> = tokio::task::JoinSet::new();

        if let Some(pc) = &self.sc.parse_config_json {
            if let Some(game_type) = &pc.game_type {
                for gt in game_type.iter() {
                    let url = self.sc.base_url.to_owned()
                        + format!(
                            "/api/leaguegame/get/{}/{}/{}/{}/{}/0",
                            &part, season, parent, child, gt
                        )
                        .as_str();

                    eprintln!("url: {}", url);
                    let resp = self
                        .client
                        .get_auto_redirect(&url, None)
                        .await
                        .context(format!("http err {}", url.to_owned()))?;
                    let games: Vec<Game> = serde_json::from_str(&resp).context(url)?;

                    for game in games.iter() {
                        let mut g = game.clone();
                        g.naive_dt =
                            chrono::NaiveDateTime::parse_from_str(&g.eDate, "%Y-%m-%dT%H:%M:%S")
                                .context(format!("date parsing {}", g.naive_dt))?;

                        let c = self.client.clone();
                        jobs.spawn(async move {
                            let address = get_address(c, &g).await?;
                            eprintln!("addr: {}", address);
                            g.Address = address;
                            Ok(g)
                        });
                    }
                }
            }
        }

        while let Some(res) = jobs.join_next().await {
            match res {
                Ok(Ok(game)) => all_games.push(game),
                Ok(Err(e)) => eprintln!("task error {}", e),
                Err(e) => eprintln!("join error {}", e),
            }
        }
        Ok(all_games)
    }
}

async fn get_address(client: Arc<client::HttpClient>, game: &Game) -> Result<String> {
    let url = format!(
        "http://rinkdb.com/v2/view/{}/{}/{}",
        game.Country, game.Prov, game.RARIDString
    );
    let resp = client.get_auto_redirect(&url, None).await?;

    let doc = Html::parse_document(&resp);

    let parts: Vec<String> = doc
        .select(&ADDRESS_SELECTOR)
        .map(|s| s.text().next().unwrap_or_default().to_owned())
        .collect();

    let address = parts.join(" ");
    Ok(address)
}

fn scrape_season(body: &str, yyyy: &str) -> Result<String> {
    let doc = Html::parse_document(body);

    for node in doc.select(&SEASON_SELECTOR) {
        let Some(s) = node.text().next() else {
            return Err(anyhow!("season selector text not found"));
        };

        let cap = RE_YEARS
            .captures(s)
            .ok_or(anyhow!("years parsing failed {}", s))?;

        if &cap["from"] <= yyyy && &cap["to"] >= yyyy {
            let v = node
                .attr("value")
                .ok_or(anyhow!("season value not found"))?;
            return Ok(v.into());
        }
    }

    Ok("".into())
}
