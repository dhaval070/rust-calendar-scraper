use calendar_scraper::Repository;
use calendar_scraper::address_fetcher;
use calendar_scraper::client;
use calendar_scraper::client::HttpClient;
use calendar_scraper::config;
use calendar_scraper::site_scraper;
use clap::Parser;
use diesel::prelude::MysqlConnection;
use std::sync::Arc;
// use std::fs;

#[derive(Parser, Debug)]
#[command(name = "scrape")]
struct Scrape {
    #[arg(short, long)]
    sites: String,
    #[arg(short, long)]
    date: Option<String>,
    #[arg(short, long)]
    import_locations: bool,
}

#[tokio::main]
async fn main() {
    let cfg = config::load();

    println!("{:?}", cfg);

    let args = Scrape::parse();
    println!("{:?}", args);

    let dt = match args.date {
        Some(ymd) => chrono::NaiveDate::parse_from_str(&ymd, "%Y-%m-%d").unwrap(),
        _ => chrono::prelude::Local::now().naive_local().date(),
    };

    let sites = args.sites.split(",").collect();
    let mut repo = Repository::<MysqlConnection>::new(&cfg.db_dsn);

    let sc = repo.get_sites(sites).unwrap();

    let mut handles = Vec::new();

    // let client = Arc::new(HttpClient::new());

    let addr_fetcher = Arc::new(address_fetcher::AddressFetcher::new(
        client::HttpClient::new(),
    ));

    let scraper = Arc::new(site_scraper::Scraper::new(
        HttpClient::new(),
        addr_fetcher.clone(),
        Arc::new(repo),
    ));

    for site in sc {
        if site.parser_type == "external" || site.parser_type == "custom" {
            println!("skipping {}", site.site_name);
            continue;
        }
        println!("{} {} {}", site.site_name, site.base_url, site.parser_type);

        let scraper = Arc::clone(&scraper);

        let h = tokio::spawn(async move {
            if let Err(e) = scraper.process_site(&site, dt).await {
                println!("failed {} {}", site.site_name, e);
            };
        });
        handles.push(h);
    }

    for h in handles {
        h.await.unwrap();
    }
    addr_fetcher.total_addresses();
}
