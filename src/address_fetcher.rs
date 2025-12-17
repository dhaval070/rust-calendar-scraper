use crate::client::HttpClient;
use anyhow::{Context, Result};
use dashmap::DashMap;
use scraper::{Html, Selector};
use std::sync::Arc;
use std::sync::LazyLock;
use tokio::sync::RwLock;

#[allow(unused)]
pub struct AddressFetcher {
    client: HttpClient,
    addresses: Arc<DashMap<String, Arc<RwLock<Address>>>>,
}

#[derive(Clone)]
#[allow(unused)]
struct Address {
    status: AddressStatus,
    address: String,
}

#[derive(Clone, PartialEq)]
#[allow(unused)]
enum AddressStatus {
    InFlight,
    Ready,
}

static ADDRESS_SELECTOR: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("div.bg_primary > div > div > div > h2 > small").unwrap());

impl AddressFetcher {
    pub fn new(client: HttpClient) -> Self {
        Self {
            client: client,
            addresses: Arc::new(DashMap::new()),
        }
    }

    pub fn total_addresses(&self) {
        println!("total addresses: {}", self.addresses.clone().len());
    }

    pub async fn get_address(&self, site: &str, base_url: &str, url: &str) -> Result<String> {
        let site = site.to_string();
        let addr = self
            .addresses
            .entry(site)
            .or_insert_with(|| {
                Arc::new(RwLock::new(Address {
                    status: AddressStatus::InFlight,
                    address: "".into(),
                }))
            })
            .clone();

        let r = addr.read().await;
        if r.status == AddressStatus::Ready {
            println!("cache hit");
            return Ok(r.address.clone());
        }
        drop(r);

        let mut lock = addr.write().await;

        if lock.status == AddressStatus::Ready {
            println!("cache hit");
            return Ok(lock.address.clone());
        }

        let address: Result<String>;

        address = if !url.starts_with("http") {
            let mut base_url = base_url.to_string();
            base_url.push_str(&url);

            let contents = self.client.get(&base_url).await?;
            self.scrape_local_address(&contents)
        } else {
            let mut addr: Result<String> = Err(anyhow::anyhow!("address not found"));

            for _ in 0..3 {
                let contents = self.client.get(&url).await?;

                addr = self.scrape_remote_address(&contents);
                if let Ok(_) = addr {
                    break;
                }
                tokio::time::sleep(tokio::time::Duration::from_secs(4)).await;
                eprintln!("retrying");
                continue;
            }
            addr
        };

        if let Ok(addr) = &address {
            lock.address = addr.clone();
            lock.status = AddressStatus::Ready;
        }
        address
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
