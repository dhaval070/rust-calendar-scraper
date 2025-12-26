fn main() {
    use scraper::Selector;
    
    let result = Selector::parse("div > div > div[2] > a");
    match result {
        Ok(_) => println!("✓ Selector 'div > div > div[2] > a' is SUPPORTED"),
        Err(e) => println!("✗ Selector 'div > div > div[2] > a' is NOT supported: {:?}", e),
    }
    
    let result2 = Selector::parse("div > div > div > a");
    match result2 {
        Ok(_) => println!("✓ Selector 'div > div > div > a' is SUPPORTED"),
        Err(e) => println!("✗ NOT supported: {:?}", e),
    }
}
