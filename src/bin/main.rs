use bsky_tsumeshogi_bot::scraper::scrape_everyday_links;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    for link in scrape_everyday_links().await? {
        println!("{}", link);
    }
    Ok(())
}
