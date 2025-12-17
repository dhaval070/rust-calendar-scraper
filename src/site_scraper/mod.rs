use anyhow::{Context, Result};
use chrono::NaiveDate;
use scraper::{ElementRef, Html, Selector};
use std::sync::LazyLock;
// use tower::{Service, ServiceBuilder};
// use tower_reqwest::{HttpClientLayer, set_header::SetRequestHeaderLayer};

use crate::address_fetcher;
use crate::client::HttpClient;
use crate::models;

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

static ADDRESS_SELECTOR: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("div.bg_primary > div > div > div > h2 > small").unwrap());

#[derive(Debug)]
pub struct ScrapedGame {
    pub site_name: String,
    pub date: chrono::NaiveDateTime,
    pub division: String,
    pub home_team: String,
    pub away_team: String,
    pub location: String,
    pub address_url: String,
}

pub struct Scraper {
    client: HttpClient,
}

impl Scraper {
    pub fn new(client: HttpClient) -> Self {
        Scraper { client }
    }

    pub async fn process_site(
        &self,
        site: &models::SitesConfig,
        from_date: NaiveDate,
    ) -> Result<()> {
        println!("processing site {}", site.site_name);

        let mm = from_date.format("%m").to_string();
        let yyyy = from_date.format("%Y").to_string();

        let mut url = site.base_url.clone();
        let s = format!("/Calendar/?Month={}&Year={}", mm, yyyy);
        url.push_str(s.as_str());
        // println!("{}", url);
        // Make HTTP GET request
        let contents = self.client.get(&site.base_url).await?;

        let games = self.scrape_games(&site.site_name, &contents)?;
        println!("Scraped {} games from {}", games.len(), site.site_name);

        for game in games {
            let mut url = game.address_url.clone();
            // let mut contents: String;

            if !url.starts_with("http") {
                let mut base_url = site.base_url.clone();
                base_url.push_str(&url);
                url = base_url.clone();
                let contents = self.client.get(&url).await?;
                let addr = self.scrape_local_address(&contents)?;
                println!("{} {} {}", site.site_name, url, addr);
            } else {
                let mut addr: String = "address not found".into();

                for _ in 0..3 {
                    let contents = self.client.get(&url).await?;
                    addr = match self.scrape_remote_address(&contents) {
                        Ok(addr) => addr,
                        Err(_) => {
                            // eprintln!("{} {} {}", url, e, contents);
                            tokio::time::sleep(tokio::time::Duration::from_secs(4)).await;
                            eprintln!("retrying");
                            continue;
                        }
                    };
                }
                if addr == "address not found" {
                    eprintln!("retry exhausted");
                }

                println!("{} {} {}", site.site_name, url, addr);
            }
        }
        Ok(())
    }

    pub fn scrape_games(&self, site_name: &str, contents: &str) -> Result<Vec<ScrapedGame>> {
        let doc = Html::parse_document(contents);
        let mut games: Vec<ScrapedGame> = Vec::new();

        for ds in doc.select(&*DAY_DETAILS_SELECTOR) {
            let id = ds
                .attr("id")
                .ok_or_else(|| anyhow::anyhow!("id not found"))?;

            let id = id.replace("day-", "");
            // println!("{}", id);

            let dt = chrono::NaiveDate::parse_from_str(&id, "%b-%d-%Y")
                .context("failed to parse date")?;

            for item in ds.select(&*EVENT_LIST_SELECTOR) {
                // let game = self.scrape_game(item, dt, site_name).map_err(|e| {
                //     anyhow::anyhow!(format!(
                //         "{} : {} : err: {}",
                //         site_name,
                //         item.inner_html(),
                //         e
                //     ))
                // });
                let game = self.scrape_game(item, dt, site_name);
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

    fn scrape_game(&self, item: ElementRef, dt: NaiveDate, site_name: &str) -> Result<ScrapedGame> {
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

        // println!(
        //     "{} - {} - {} vs {} @ {}",
        //     dt, division, home_team, away_team, loc
        // );
        Ok(ScrapedGame {
            site_name: site_name.into(),
            date: dt,
            division: division.into(),
            home_team: home_team,
            away_team,
            location: loc.into(),
            address_url: address_url.into(),
        })
    }

    pub fn scrape_local_address(&self, contents: &str) -> Result<String> {
        let doc = Html::parse_document(contents);
        let sel = Selector::parse("div.callout").map_err(|e| anyhow::anyhow!("{}", e))?;
        let element = doc
            .select(&sel)
            .next()
            .context("addr node not found")?
            .first_child()
            .context("first child not found")?
            .first_child()
            .context("grand child not found")?
            .children()
            .nth(1)
            .context("second child not found")?
            .value()
            .as_text()
            .context("text not found")?;

        Ok(element.to_string())
    }

    // e.g. https://www.theonedb.com/Venue/Map/10566?day=19&month=12&year=2025&body=10009
    pub fn scrape_remote_address(&self, contents: &str) -> Result<String> {
        let doc = Html::parse_document(contents);

        let element = doc
            .select(&ADDRESS_SELECTOR)
            .nth(1)
            .context("divsel failed")?;
        let addr = element
            .text()
            .next()
            .expect("addr node not found")
            .to_string();
        Ok(addr)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_scrape_remote_address() {
        let sc = Scraper {
            client: crate::client::HttpClient::new(),
        };
        let contents = fs::read_to_string("addr.html").unwrap();
        let addr = sc.scrape_remote_address(&contents).unwrap();
        assert_eq!("728 Mountain St, Haliburton, ON  ", addr);
    }
}
