use anyhow::Result;
use axum::extract::Query;
use axum::{Router, routing::get};
use calendar_scraper::address_fetcher::AddressFetcher;
use calendar_scraper::client;
use calendar_scraper::config;
use clap::Parser;
use serde::Deserialize;
use std::io::Write;
use std::sync::Arc;
use tempfile;
use tokio::net::UnixListener;
use tokio::time::{Duration, sleep};
use wincode;

struct Service {
    fetcher: Arc<AddressFetcher>,
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

#[derive(Parser, Debug)]
#[command(name = "address")]
struct Args {
    #[arg(long)]
    file: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cfg = config::load();
    let listener = UnixListener::bind("\0address-fetcher.sock")?;

    let client = Arc::new(client::HttpClient::new(
        cfg.max_requests_per_host,
        cfg.max_global_requests,
    ));

    let args = Args::parse();

    let addr_fetcher = Arc::new(AddressFetcher::new(client.clone()));
    let srv = Arc::new(Service {
        fetcher: addr_fetcher.clone(),
    });

    let r = Router::<()>::new().route(
        "/",
        get({
            let srv = srv.clone();
            async move |qry| srv.get(qry).await
        }),
    );

    let f = addr_fetcher.clone();
    if let Some(path) = args.file {
        if std::fs::exists(&path).unwrap() {
            let bdata = std::fs::read(&path).unwrap();
            let data: Vec<(String, String)> = wincode::deserialize(&bdata).unwrap();
            println!("cache entries loaded {}", data.len());

            addr_fetcher.load(data);
        }

        tokio::spawn(async move { write_snapshot(f, &path).await });
    }

    axum::serve(listener, r).await?;
    Ok(())
}

async fn write_snapshot(f: Arc<AddressFetcher>, file_path: &str) {
    let path = std::path::Path::new(file_path);
    // let path = std::path::absolute(path).unwrap();
    let dirname = path.parent().unwrap();

    loop {
        let snapshot = f.get_snapshot().await;
        if snapshot.len() == 0 {
            continue;
        }

        println!("snapshot entries {}", snapshot.len());

        let data = wincode::serialize(&snapshot).unwrap();

        let tmp_file = tempfile::NamedTempFile::new_in(dirname).unwrap();
        let mut fh = tmp_file.as_file();

        fh.write(&data).unwrap();
        // fh.write_all(&data).unwrap();
        // fh.flush().await.unwrap();

        std::fs::rename(tmp_file.into_temp_path(), file_path).unwrap();
        sleep(Duration::from_secs(30)).await;
    }
}
