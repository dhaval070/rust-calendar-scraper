use crate::client::{HttpClient, Response};
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

    pub async fn get_address(&self, _site: &str, base_url: &str, url: &str) -> Result<String> {
        let (mut url, is_local) = self.build_abs_url(base_url, url);

        let mut current_addr = self.get_cached(&url);
        let orig_addr = current_addr.clone();

        loop {
            // Fast path: check read lock first
            let r = current_addr.read().await;
            if r.status == AddressStatus::Ready {
                println!("cache hit");
                return Ok(r.address.clone());
            }
            drop(r);

            // Acquire write lock
            let mut lock = current_addr.write().await;

            // Double-check after acquiring write lock
            if lock.status == AddressStatus::Ready {
                println!("cache hit");
                return Ok(lock.address.clone());
            }

            // Fetch URL while holding write lock
            match self.client.get(&url).await? {
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
                        return Err(anyhow::anyhow!("captcha presented for {}", url));
                    }
                    println!("redirect {}", redirect);
                    // Drop current lock before acquiring new one
                    drop(lock);

                    // Switch to redirect URL's cache entry
                    current_addr = self.get_cached(&redirect);
                    url = redirect;
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

    fn build_abs_url(&self, base_url: &str, url: &str) -> (String, bool) {
        let mut is_local: bool = false;
        let url = if !url.starts_with("http") {
            is_local = true;
            let mut base_url = base_url.to_string();
            base_url.push_str(&url);
            base_url
        } else {
            url.to_string()
        };
        (url, is_local)
    }
}
