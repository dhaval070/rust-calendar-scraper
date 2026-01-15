use crate::client;
use crate::site_scraper::ScrapedGame;
use anyhow::{Context, Result, anyhow};
use hex;
use hmac::{self, Hmac, Mac};
use md5;
use regex::Regex;
use scraper::{Html, Selector};
use serde::Deserialize;
use sha2::Sha256;
use std::sync::Arc;

#[derive(Debug, Clone)]
struct Division {
    id: String,
    name: String,
}

#[derive(Debug)]
struct Creds {
    username: String,
    secret: String,
    api_url: String,
    league_id: String,
}

#[derive(Deserialize)]
struct Game {
    date: String,
    time: String,
    #[serde(skip)]
    datetime: chrono::NaiveDateTime,
    home_team: String,
    away_team: String,
    location: String,
    #[serde(skip)]
    division: String,
}

#[derive(Deserialize)]
struct ApiResponse {
    games: Vec<Game>,
}

pub async fn get_games(
    c: Arc<client::HttpClient>,
    site: &str,
    mm: &str,
    yyyy: &str,
) -> Result<Vec<ScrapedGame>> {
    let cred = Arc::new(get_creds(c.clone()).await?);
    let divisions = get_divisions(c.clone()).await?;

    let mut tasks = tokio::task::JoinSet::new();

    let s = yyyy.to_owned() + "-" + mm + "-01 00:00:00";
    let cutoff = chrono::NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S")?;

    for d in divisions {
        let c = c.clone();
        let cred = cred.clone();

        tasks.spawn(async move { get_schedules(d.clone(), cred, c, cutoff).await });
    }

    let mut all_games: Vec<Game> = Vec::new();

    while let Some(t) = tasks.join_next().await {
        match t {
            Ok(Ok(mut games)) => {
                all_games.append(&mut games);
            }
            Ok(Err(e)) => {
                eprintln!("task error {}", e);
            }
            Err(e) => {
                eprintln!("join error {}", e);
            }
        }
    }

    let scraped_games = all_games
        .into_iter()
        .map(|g| ScrapedGame {
            date: g.datetime,
            site_name: site.into(),
            home_team: g.home_team,
            away_team: g.away_team,
            location: g.location,
            division: g.division,
            address_url: "".into(),
            address: "".into(),
        })
        .collect();
    Ok(scraped_games)
}

async fn get_divisions(c: Arc<client::HttpClient>) -> Result<Vec<Division>> {
    let url = "https://atlantichockeyfederation.com/game-center/";
    let resp = c.get_auto_redirect(url, None).await?;

    let doc = Html::parse_document(&resp);
    let card_selector = Selector::parse("div.container > div.card").unwrap();
    let name_selector = Selector::parse("h3").unwrap();
    let links_selector = Selector::parse("a").unwrap();

    let re = Regex::new("level_id=([0-9]+)").unwrap();

    let mut result: Vec<Division> = Vec::new();
    for card in doc.select(&card_selector) {
        let name_node = card
            .select(&name_selector)
            .nth(0)
            .ok_or_else(|| anyhow!("division name node not found"))?;
        let name = name_node
            .text()
            .next()
            .ok_or_else(|| anyhow!("division name not found"))?;

        for link in card.select(&links_selector) {
            let href = link
                .attr("href")
                .ok_or_else(|| anyhow!("division href not found"))?;

            let id = re
                .captures(href)
                .ok_or(anyhow!("group id not found"))?
                .get(1)
                .ok_or(anyhow!("group id not found"))?
                .as_str();

            result.push(Division {
                id: id.into(),
                name: name.into(),
            });
            break;
        }
    }

    Ok(result)
}

async fn get_creds(c: Arc<client::HttpClient>) -> Result<Creds> {
    let base_url = "https://atlantichockeyfederation.com/schedule/?level_id=80";

    let resp = c.get_auto_redirect(base_url, None).await?;

    let username_re = Regex::new(r#"username:\s+"([a-zA-Z0-1.]+)""#)?;
    let secret_re = Regex::new(r#"secret:\s+"(\w+)""#)?;
    let api_url_re = Regex::new(r#"api_url:\s+"([a-zA-Z0-1.]+)""#)?;
    let league_id_re = Regex::new(r#"league_id:\s+"(\w+)""#)?;

    let username = capture_creds(username_re, &resp)?.to_string();
    let secret = capture_creds(secret_re, &resp)?.to_string();
    let api_url = capture_creds(api_url_re, &resp)?.to_string();
    let league_id = capture_creds(league_id_re, &resp)?.to_string();

    Ok(Creds {
        username,
        secret,
        api_url,
        league_id,
    })
}

fn capture_creds(re: Regex, resp: &str) -> Result<&str> {
    let s = re
        .captures(resp)
        .ok_or(anyhow!("pattern not found"))?
        .get(1)
        .ok_or(anyhow!("cred not found"))?
        .as_str();
    Ok(s)
}

async fn get_schedules(
    d: Division,
    cred: Arc<Creds>,
    c: Arc<client::HttpClient>,
    cutoff: chrono::NaiveDateTime,
) -> Result<Vec<Game>> {
    let url = sign_url(&d, &cred).context("sign url")?;

    let resp = c.get_auto_redirect(&url, None).await.context("geturl")?;

    let mut res: ApiResponse = serde_json::from_str(&resp)?;

    for g in res.games.iter_mut() {
        g.division = d.name.clone();
    }

    for g in res.games.iter_mut() {
        let s = g.date.to_owned() + " " + &g.time;
        let dt = chrono::NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S")
            .context(format!("parsing datetime '{}'", s))?;

        if dt < cutoff {
            continue;
        }
        g.datetime = dt;
    }

    Ok(res.games)
}

fn sign_url(d: &Division, cred: &Creds) -> Result<String> {
    let url: String = "https://".to_owned() + &cred.api_url + "/get_schedule";
    let tt = chrono::Utc::now().timestamp().to_string();

    let data = md5::compute(b"".as_slice());
    let md5hex = hex::encode(data.as_slice());
    let params: [(&str, &str); 7] = [
        ("league_id", &cred.league_id),
        ("level_id", &d.id),
        ("stat_class", "1"),
        ("season_id", "131"),
        ("auth_key", &cred.username),
        ("auth_timestamp", &tt),
        ("body_md5", &md5hex),
    ];
    let pairs: Vec<String> = params.iter().map(|p| p.0.to_owned() + "=" + p.1).collect();
    let canonical_query = pairs.join("&");
    let string_to_sign = "GET".to_owned() + "\n/get_schedule\n" + &canonical_query;

    let mut hm = Hmac::<Sha256>::new_from_slice(cred.secret.as_bytes())?;
    hm.update(string_to_sign.as_bytes());
    let sig = hex::encode(hm.finalize().into_bytes());

    let url = url + "?" + &canonical_query + "&auth_signature=" + &sig;
    Ok(url)
}
