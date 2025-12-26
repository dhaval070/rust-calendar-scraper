use crate::site_scraper;
use anyhow::Result;
use csv;

pub fn write_output(games: &Vec<site_scraper::ScrapedGame>, w: impl std::io::Write) -> Result<()> {
    let mut wrt = csv::Writer::from_writer(w);

    for g in games.iter() {
        wrt.serialize(g)?;
    }
    Ok(())
}
