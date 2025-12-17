use calendar_scraper::config;
use calendar_scraper::connect_db;

use calendar_scraper::venue_import;

fn main() {
    let cfg = config::load();

    let mut dbh = connect_db(&cfg);

    println!("{:?}", cfg);

    let f = venue_import::parse_file("m.json");

    venue_import::import(&mut dbh, f);
}
