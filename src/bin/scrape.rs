use calendar_scraper::Repository;
use calendar_scraper::address_fetcher;
use calendar_scraper::client;
use calendar_scraper::client::HttpClient;
use calendar_scraper::cmdutils;
use calendar_scraper::config;
use calendar_scraper::site_scraper;
use clap::Parser;
use diesel::prelude::MysqlConnection;
use std::sync::Arc;
// use std::fs;

#[derive(Parser, Debug)]
#[command(name = "scrape")]
struct Args {
    #[arg(short, long)]
    sites: String,
    #[arg(short, long)]
    date: Option<String>,
    #[arg(short, long)]
    import_locations: bool,
    #[arg(long)]
    out_file: String,
}

#[tokio::main]
async fn main() {
    let cfg = config::load();

    println!("{:?}", cfg);

    let args = Args::parse();
    println!("{:?}", args);

    let dt = match args.date {
        Some(ymd) => chrono::NaiveDate::parse_from_str(&ymd, "%Y-%m-%d").unwrap(),
        _ => chrono::prelude::Local::now().naive_local().date(),
    };

    let sites = args.sites.split(",").collect();
    let mut repo = Repository::<MysqlConnection>::new(&cfg.db_dsn);

    let sc = repo.get_sites(sites).unwrap();

    let addr_fetcher = Arc::new(address_fetcher::AddressFetcher::new(
        client::HttpClient::new(),
    ));

    let scraper = Arc::new(site_scraper::Scraper::new(
        HttpClient::new(),
        addr_fetcher.clone(),
        Arc::new(repo),
    ));

    let mut path = std::path::PathBuf::new();
    path.push(args.out_file.clone().as_str());
    path = std::path::absolute(path).unwrap();

    let mut handles = Vec::new();

    for site in sc {
        if site.parser_type == "external" || site.parser_type == "custom" {
            println!("skipping {}", site.site_name);
            continue;
        }
        println!("{} {} {}", site.site_name, site.base_url, site.parser_type);

        let scraper = Arc::clone(&scraper);

        let out_file = args.out_file.clone();
        let p = path.clone();

        let h = tokio::spawn(async move {
            match scraper.process_site(&site, dt).await {
                Ok(games) => {
                    let wrt: Box<dyn std::io::Write> = match out_file.as_str() {
                        "-" => Box::new(std::io::stdout()),
                        _ => {
                            let dir = p.parent().unwrap();
                            let path = dir.join(
                                site.site_name + "_" + p.file_name().unwrap().to_str().unwrap(),
                            );

                            let file = std::fs::File::create(path).unwrap();
                            Box::new(file)
                        }
                    };
                    cmdutils::write_output(games, wrt).unwrap();
                }
                Err(e) => println!("failed {} {}", site.site_name, e),
            };
        });
        handles.push(h);
    }

    for h in handles {
        h.await.unwrap();
    }
    addr_fetcher.total_addresses();
}
