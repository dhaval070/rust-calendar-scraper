use anyhow::Result;
// use diesel::prelude::MysqlConnection;
use crate::site_scraper::ScrapedGame;
use scraper::{ElementRef, Selector};
use std::sync::LazyLock;

static DAY_DETAILS_SELECTOR: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("div.day-details").unwrap());

static EVENT_LIST_SELECTOR: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("div.event-list-item").unwrap());

static TIME_SELECTOR: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("div.time-primary").unwrap());

static SUBJECT_OWNER_SELECTOR: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("div.subject-owner").unwrap());

static SUBJECT_TEXT_SELECTOR: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("div.subject-text").unwrap());
static LOCATION_SELECTOR: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("div.location").unwrap());

static GROUP_SELECTOR: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("div.subject-group").unwrap());

static LOC_LINK_SELECTOR: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("div > div > div:nth-of-type(3) > a").unwrap());

pub fn parse_schedules(
    site: &str,
    contents: String,
    mm: &str,
    yyyy: &str,
) -> Result<Vec<ScrapedGame>> {
    let doc = scraper::Html::parse_document(contents.as_str());

    let mut games: Vec<ScrapedGame> = Vec::new();
    for dd in doc.select(&DAY_DETAILS_SELECTOR) {
        for item in dd.select(&EVENT_LIST_SELECTOR) {
            let tt = item.select(&TIME_SELECTOR).next().unwrap();

            let date = parse_datetime(tt, mm, yyyy);

            let Some(sgroup) = item.select(&GROUP_SELECTOR).next() else {
                continue;
            };
            let Some(sowner) = item.select(&SUBJECT_OWNER_SELECTOR).next() else {
                continue;
            };
            let Some(stext) = item.select(&SUBJECT_TEXT_SELECTOR).next() else {
                continue;
            };
            let Some(sloc) = item.select(&LOCATION_SELECTOR).next() else {
                continue;
            };

            let team1 = match sowner.text().next() {
                Some(d) => d.to_string(),
                _ => continue,
            };
            let team2 = match stext.text().next() {
                Some(d) => d.to_string(),
                _ => continue,
            };

            let teams: (String, String) = if team2.contains("@") {
                (team2.replace("@ ", "").into(), team1.clone())
            } else {
                (team1.clone(), team2.replace("vs ", "").into())
            };

            let division = match sgroup.text().next() {
                Some(d) => d.to_string(),
                _ => team1,
            };

            let location = match sloc.text().next() {
                Some(l) => l.to_string(),
                _ => continue,
            };

            let link = match item.select(&LOC_LINK_SELECTOR).next() {
                Some(l) => l,
                _ => {
                    eprintln!("loc link not found");
                    continue;
                }
            };

            let address_url = match link.attr("href") {
                Some(l) => l.to_string(),
                _ => {
                    eprintln!("loc link url not found");
                    continue;
                }
            };

            games.push(ScrapedGame {
                site_name: site.into(),
                date: date,
                division,
                home_team: teams.0,
                away_team: teams.1,
                location,
                address_url,
                address: "".into(),
            });
        }
    }
    Ok(games)
}

fn parse_datetime(node: ElementRef, mm: &str, yyyy: &str) -> chrono::NaiveDateTime {
    let t = node.text().nth(1).unwrap();
    let dt = node.text().nth(0).unwrap();
    let day = dt.split(" ").nth(1).unwrap();
    let mut dt_str = yyyy.to_string();

    dt_str.push_str("-");
    dt_str.push_str(mm.to_string().as_str());
    dt_str.push_str("-");
    dt_str.push_str(day);
    dt_str.push_str(" ");
    dt_str.push_str(t);

    chrono::NaiveDateTime::parse_from_str(dt_str.as_str(), "%Y-%m-%d %I:%M %p").unwrap()
}
