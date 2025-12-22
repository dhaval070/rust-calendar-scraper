use anyhow::Result;
// use diesel::prelude::MysqlConnection;
use crate::site_scraper::ScrapedGame;
use scraper::{ElementRef, Selector};

pub fn parse_schedules(
    site: &str,
    contents: String,
    mm: &str,
    yyyy: &str,
) -> Result<Vec<ScrapedGame>> {
    let day_details_selector = Selector::parse("div.day-details").unwrap();
    let items_selector = Selector::parse("div.event-list-item").unwrap();
    let time_selector = Selector::parse("div.time-primary").unwrap();
    let subj_group_selector = Selector::parse("div.subject-group").unwrap();
    let subj_owner_selector = Selector::parse("div.subject-owner").unwrap();
    let subj_text_selector = Selector::parse("div.subject-text").unwrap();
    let location_selector = Selector::parse("div.location").unwrap();
    let location_link_selector = Selector::parse("div > div > div:nth-of-type(3) > a").unwrap();

    let doc = scraper::Html::parse_document(contents.as_str());

    let mut games: Vec<ScrapedGame> = Vec::new();
    for dd in doc.select(&day_details_selector) {
        for item in dd.select(&items_selector) {
            let tt = item.select(&time_selector).next().unwrap();

            let date = parse_datetime(tt, mm, yyyy);

            let Some(sgroup) = item.select(&subj_group_selector).next() else {
                continue;
            };
            let Some(sowner) = item.select(&subj_owner_selector).next() else {
                continue;
            };
            let Some(stext) = item.select(&subj_text_selector).next() else {
                continue;
            };
            let Some(sloc) = item.select(&location_selector).next() else {
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

            let link = match item.select(&location_link_selector).next() {
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
