use crate::client::{HttpClient, Response};
use anyhow::anyhow;
use anyhow::{Context, Result};
use dashmap::DashMap;
use scraper::{Html, Selector};
use std::sync::Arc;
use std::sync::LazyLock;
use tokio::sync::RwLock;

pub struct AddressFetcher {
    client: Arc<HttpClient>,
    addresses: Arc<DashMap<String, Arc<RwLock<Address>>>>,
}

#[derive(Clone)]
#[allow(unused)]
struct Address {
    status: AddressStatus,
    address: String,
}

#[derive(Clone, PartialEq)]
enum AddressStatus {
    InFlight,
    Ready,
}

static ADDRESS_SELECTOR: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("div.bg_primary > div > div > div > h2 > small").unwrap());

impl AddressFetcher {
    pub fn new(client: Arc<HttpClient>) -> Self {
        Self {
            client: client,
            addresses: Arc::new(DashMap::new()),
        }
    }

    pub fn total_addresses(&self) {
        println!("total addresses: {}", self.addresses.clone().len());
    }

    pub async fn get_address(&self, _site: &str, scrape_url: &str, class: &str) -> Result<String> {
        let is_local = class == "local";
        let mut scrape_url: String = scrape_url.into();

        if class == "local" && scrape_url.contains("Venues") {
            // remove query string params from https://aceshockey.com/Venues/12/?Day=10&Month=01&Year=2026 for caching
            scrape_url = scrape_url
                .split_terminator("?")
                .next()
                .ok_or(anyhow!("failed to split url {}", scrape_url))?
                .to_string();
        }

        let use_captcha = class == "local";

        let mut current_addr = self.get_cached(&scrape_url);
        let orig_addr = current_addr.clone();

        loop {
            // Fast path: check read lock first
            let r = current_addr.read().await;
            if r.status == AddressStatus::Ready {
                println!("cache hit {}", scrape_url);
                return Ok(r.address.clone());
            }
            drop(r);

            // Acquire write lock
            let mut lock = current_addr.write().await;

            // Double-check after acquiring write lock
            if lock.status == AddressStatus::Ready {
                println!("cache hit {}", scrape_url);
                return Ok(lock.address.clone());
            }

            // Fetch URL while holding write lock
            match self.client.get(&scrape_url, use_captcha).await? {
                Response::Content(contents) => {
                    // Scrape address
                    let address = if is_local {
                        self.scrape_local_address(&contents)
                    } else {
                        self.scrape_remote_address(&contents)
                    };

                    if let Ok(ad) = &address {
                        lock.address = ad.clone();
                        lock.status = AddressStatus::Ready;

                        // Also update original URL if we followed redirects
                        if !Arc::ptr_eq(&current_addr, &orig_addr) {
                            let mut orig_lock = orig_addr.write().await;
                            orig_lock.address = ad.clone();
                            orig_lock.status = AddressStatus::Ready;
                        }
                    }
                    return address;
                }
                Response::Redirect(redirect) => {
                    if redirect.contains("/Human/") {
                        return Err(anyhow::anyhow!("captcha presented for {}", scrape_url));
                    }
                    println!("redirect {}", redirect);
                    // Drop current lock before acquiring new one
                    drop(lock);

                    // Switch to redirect URL's cache entry
                    current_addr = self.get_cached(&redirect);
                    scrape_url = redirect;
                    // Loop will acquire lock on the new URL
                }
            };
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

    fn get_cached(&self, url: &str) -> Arc<RwLock<Address>> {
        self.addresses
            .entry(url.to_string())
            .or_insert_with(|| {
                Arc::new(RwLock::new(Address {
                    status: AddressStatus::InFlight,
                    address: "".into(),
                }))
            })
            .clone()
    }

    pub async fn get_snapshot(&self) -> Vec<(String, String)> {
        let mut col: Vec<(String, Arc<RwLock<Address>>)> = Vec::with_capacity(self.addresses.len());

        for addr in self.addresses.iter() {
            let key = addr.key().clone();
            let g = addr.value().clone();
            col.push((key, g));
        }
        let mut result: Vec<(String, String)> = Vec::with_capacity(col.len());

        for item in col.into_iter() {
            let v = item.1.read().await;
            if v.status == AddressStatus::Ready {
                result.push((item.0, v.address.clone()));
            }
        }
        result
    }

    pub fn load(&self, data: Vec<(String, String)>) {
        for entry in data {
            self.addresses.entry(entry.0).or_insert_with(|| {
                Arc::new(RwLock::new(Address {
                    status: AddressStatus::Ready,
                    address: entry.1,
                }))
            });
        }
    }
}
