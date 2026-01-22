use anyhow::{Context, Result};
use chrono::NaiveDate;
use scraper::{Html, Selector};
use std::collections::HashMap;
use std::sync::{Arc, LazyLock};

use crate::client::HttpClient;
use crate::models::{self, SitesConfigM};
use crate::site_scraper::gamesheet::Gamesheet;
use crate::{address_fetcher, repository};
pub mod atlantic;
mod day_deails;
pub mod gamesheet;
mod month_based;
mod rockies;

static ADDRESS_SELECTOR: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("div.bg_primary > div > div > div > h2 > small").unwrap());

#[derive(Debug, Clone, serde::Serialize)]
pub struct ScrapedGame {
    pub date: chrono::NaiveDateTime,
    pub site_name: String,
    pub home_team: String,
    pub away_team: String,
    pub location: String,
    pub division: String,
    #[serde(skip)]
    pub address_url: String,
    pub address: String,
}

pub struct Scraper {
    pub client: Arc<HttpClient>,
    address_fetcher: Arc<address_fetcher::AddressFetcher>,
    repo: Arc<dyn repository::RepositoryOps + Send + Sync>,
    import_locations: bool,
    gamesheet: Gamesheet,
}

impl Scraper {
    pub fn new(
        client: Arc<HttpClient>,
        address_fetcher: Arc<address_fetcher::AddressFetcher>,
        repo: Arc<dyn repository::RepositoryOps + Send + Sync>,
        import_locations: bool,
        gamesheet: Gamesheet,
    ) -> Self {
        Scraper {
            client,
            address_fetcher,
            repo,
            import_locations,
            gamesheet,
        }
    }

    pub async fn process_site(
        &self,
        site: &models::SitesConfigM,
        from_date: NaiveDate,
    ) -> Result<Vec<ScrapedGame>> {
        println!("processing site {}", site.site_name);

        let mm = from_date.format("%m").to_string();
        let yyyy = from_date.format("%Y").to_string();

        let games = match site.parser_type.as_str() {
            "day_details" => {
                day_deails::get_games(
                    self.client.clone(),
                    &site.site_name,
                    &site.base_url,
                    &mm,
                    &yyyy,
                )
                .await?
            }
            "month_based" => {
                month_based::get_games(
                    self.client.clone(),
                    &site.site_name,
                    &site.base_url,
                    &mm,
                    &yyyy,
                )
                .await?
            }
            _ => self.custom_get_games(site, &mm, &yyyy).await?,
        };

        println!("Scraped {} games from {}", games.len(), site.site_name);

        let mut tasks = tokio::task::JoinSet::new();

        for mut game in games {
            let fetcher = Arc::clone(&self.address_fetcher);
            let site_name = site.site_name.clone();
            let base_url = site.base_url.clone();

            tasks.spawn(async move {
                if game.address_url != "" {
                    let (url, class) = build_abs_url(&base_url, &game.address_url);
                    let address = fetcher.get_address(&site_name, &url, &class).await;

                    game.address = address.unwrap_or_else(|e| {
                        eprintln!("{}", e);
                        "".into()
                    });
                    println!("url: {}, address: {}", url, game.address);
                }
                game
            });
        }
        let mut games: Vec<ScrapedGame> = Vec::new();

        while let Some(g) = tasks.join_next().await {
            games.push(g.unwrap());
        }

        if self.import_locations {
            let mut h = HashMap::new();
            for g in games.iter() {
                h.insert(
                    g.location.clone(),
                    models::SitesLocation {
                        site: site.site_name.clone(),
                        location: g.location.clone(),
                        location_id: 0,
                        loc: None,
                        surface: None,
                        address: Some(g.address.clone()),
                        match_type: None,
                        surface_id: 0,
                    },
                );
            }

            let repo = Arc::clone(&self.repo);
            let site_name = site.site_name.clone();
            let site_locations = h.into_values().collect();

            tokio::task::spawn_blocking(move || {
                repo.import_locations(&site_name, site_locations).unwrap();
            })
            .await?;
        }
        Ok(games)
    }

    async fn custom_get_games(
        &self,
        site: &SitesConfigM,
        mm: &str,
        yyyy: &str,
    ) -> Result<Vec<ScrapedGame>> {
        match site.site_name.as_str() {
            s if s.starts_with("gs_") => {
                let from_date = format!("{}-{}-01", yyyy, mm);
                self.gamesheet
                    .scrape_games(from_date, &site.site_name)
                    .await
            }
            "atlantichockeyfederation" => {
                atlantic::get_games(self.client.clone(), &site.site_name, mm, yyyy).await
            }
            s if s == "rockieshockeyleague"
                || s == "allpeacehockey"
                || s == "cahlhockey"
                || s == "neahl" =>
            {
                let r = rockies::Rockies::new(self.client.clone(), site.clone());
                r.get_games(mm.to_string(), yyyy.to_string()).await
            }
            _ => return Err(anyhow::anyhow!("unsupported site {}", site.site_name)),
        }
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

fn build_abs_url(base_url: &str, url: &str) -> (String, String) {
    let mut class: String = "remote".into();
    let url = if !url.starts_with("http") {
        class = "local".into();
        let mut base_url = base_url.to_string();
        base_url.push_str(&url);
        base_url
    } else {
        url.to_string()
    };
    (url, class)
}

#[cfg(test)]
mod test {
    use crate::repository::MockRepositoryOps;
    use crate::{address_fetcher::AddressFetcher, client};

    use super::*;
    use std::fs;
    use std::sync::Arc;

    #[test]
    fn test_scrape_remote_address() {
        let client = Arc::new(client::HttpClient::new(1, 1));
        let fetcher = Arc::new(AddressFetcher::new(client.clone()));
        let repo = Arc::new(MockRepositoryOps::new());
        let sc = Scraper {
            client: client.clone(),
            address_fetcher: fetcher,
            repo: repo,
            import_locations: false,
            gamesheet: Gamesheet::new(client.clone(), "".into(), HashMap::new()),
        };
        let contents = fs::read_to_string("addr.html").unwrap();
        let addr = sc.scrape_remote_address(&contents).unwrap();
        assert_eq!("728 Mountain St, Haliburton, ON  ", addr);
    }
}
