use crate::client;
use crate::site_scraper::ScrapedGame;
use anyhow::{Context, Result};
use chrono::NaiveDate;
use scraper::{ElementRef, Html, Selector};
use std::sync::{Arc, LazyLock};

pub async fn get_games(
    client: Arc<client::HttpClient>,
    site: &str,
    base_url: &str,
    mm: &str,
    yyyy: &str,
) -> Result<Vec<ScrapedGame>> {
    let url = base_url.to_owned() + format!("/Calendar/?Month={}&Year={}", mm, yyyy).as_str();
    let contents = client.get_auto_redirect(&url, None).await?;

    parse_schedules(site, &contents).await
}

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

async fn parse_schedules(site_name: &str, contents: &str) -> Result<Vec<ScrapedGame>> {
    let doc = Html::parse_document(contents);
    let mut games: Vec<ScrapedGame> = Vec::new();

    for ds in doc.select(&*DAY_DETAILS_SELECTOR) {
        let id = ds
            .attr("id")
            .ok_or_else(|| anyhow::anyhow!("id not found"))?;

        let id = id.replace("day-", "");
        // println!("{}", id);

        let dt =
            chrono::NaiveDate::parse_from_str(&id, "%b-%d-%Y").context("failed to parse date")?;

        for item in ds.select(&*EVENT_LIST_SELECTOR) {
            if item.text().any(|t| {
                let t = t.to_lowercase();
                t.contains("practice")
                    || t.contains("tournament")
                    || t.contains("all day")
                    || t.contains("cancelled")
                    || t.contains("time-secondary")
            }) {
                continue;
            }
            let game = scrape_game(item, dt, site_name);
            let game = match game {
                Ok(g) => g,
                Err(e) => {
                    eprintln!("{}", e);
                    continue;
                }
            };

            games.push(game);
        }
    }

    Ok(games)
}

fn scrape_game(item: ElementRef, dt: NaiveDate, site_name: &str) -> Result<ScrapedGame> {
    let tt = item
        .select(&*TIME_SELECTOR)
        .next()
        .context("time not found")?;

    let ts = tt.text().next().context("time ts not found")?;
    let tt = chrono::NaiveTime::parse_from_str(ts, "%I:%M %p").context("date not found")?;
    let dt = dt.and_time(tt);

    let subj_owner = item
        .select(&*SUBJECT_OWNER_SELECTOR)
        .next()
        .context("subj owner not found")?;

    let subj_text = item
        .select(&*SUBJECT_TEXT_SELECTOR)
        .next()
        .context("subj text not found")?;

    let subj_owner = subj_owner.text().next().unwrap();
    let subj_text = subj_text.text().next().unwrap();

    let home_team: String;
    let away_team: String;
    if subj_text.contains("@ ") {
        home_team = subj_text.replace("@ ", "");
        away_team = subj_owner.into();
    } else {
        home_team = subj_owner.into();
        away_team = subj_text.replace("vs ", "").into();
    }

    let loc = item
        .select(&*LOCATION_SELECTOR)
        .next()
        .context("location selector not found")?;

    let loc = loc.text().next().unwrap();

    let division = match item.select(&*GROUP_SELECTOR).next() {
        Some(group) => group.text().next().unwrap(),
        _ => subj_owner,
    };

    let address_node = item
        .first_child()
        .context("first child not found 1")?
        .first_child()
        .context("grand child not found 2")?
        .children()
        .nth(2)
        .context("second node not found 3")?
        .first_child()
        .context("first child not found 4")?;

    let address_element = address_node
        .value()
        .as_element()
        .context("element not found")?;

    let address_url = address_element.attr("href").context("href not found")?;

    Ok(ScrapedGame {
        site_name: site_name.into(),
        date: dt,
        division: division.into(),
        home_team: home_team,
        away_team,
        location: loc.into(),
        address_url: address_url.into(),
        address: "".into(),
    })
}
