use anyhow::Result;
use axum::extract::Query;
use axum::{Router, routing::get};
use calendar_scraper::address_fetcher;
use calendar_scraper::client;
use calendar_scraper::config;
use serde::Deserialize;
use std::sync::Arc;
use tokio::net::UnixListener;

struct Service {
    fetcher: Arc<address_fetcher::AddressFetcher>,
}

impl Service {
    async fn get(&self, qry: Query<AddressQuery>) -> String {
        let address = self
            .fetcher
            .get_address("", &qry.scrape_url, &qry.class)
            .await
            .unwrap_or_default();
        println!("{} {}", qry.scrape_url, address);
        address
    }
}

#[derive(Deserialize, Debug)]
struct AddressQuery {
    scrape_url: String,
    class: String, // remote or local
}

#[tokio::main]
async fn main() -> Result<()> {
    let cfg = config::load();
    let listener = UnixListener::bind("\0address-fetcher.sock")?;

    let client = Arc::new(client::HttpClient::new(
        cfg.max_requests_per_host,
        cfg.max_global_requests,
    ));

    let addr_fetcher = Arc::new(address_fetcher::AddressFetcher::new(client.clone()));
    let srv = Arc::new(Service {
        fetcher: addr_fetcher,
    });

    let r = Router::<()>::new().route(
        "/",
        get({
            let srv = srv.clone();
            async move |qry| srv.get(qry).await
        }),
    );

    axum::serve(listener, r).await?;
    Ok(())
}
