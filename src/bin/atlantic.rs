use anyhow::Result;
use calendar_scraper::client;
use calendar_scraper::config;
use calendar_scraper::site_scraper::atlantic;
use chrono::Datelike;
use clap::Parser;
use std::sync::Arc;

#[derive(Parser, Debug)]
#[command(name = "scrape")]
struct Args {
    #[arg(short, long)]
    date: Option<String>,
    #[arg(short, long)]
    import_locations: bool,
    #[arg(long)]
    out_file: String,
    #[arg(long)]
    import_events: bool,
}

const SITE: &str = "atlantichockeyfederation";

#[tokio::main]
async fn main() -> Result<()> {
    let cfg = config::load();

    println!("{:?}", cfg);

    let args = Args::parse();
    println!("{:?}", args);

    let dt = match args.date {
        Some(ymd) => chrono::NaiveDate::parse_from_str(&ymd, "%Y-%m-%d")?,
        _ => chrono::prelude::Local::now().naive_local().date(),
    };

    let c = Arc::new(client::HttpClient::new(
        cfg.max_requests_per_host,
        cfg.max_global_requests,
    ));

    let games = atlantic::get_games(
        c.clone(),
        SITE,
        dt.month().to_string().as_str(),
        dt.year().to_string().as_str(),
    )
    .await
    .unwrap();

    println!("{:?}", games);
    Ok(())
}
