use atrium_api::app::bsky::feed::defs::PostViewEmbedEnum;
use bsky_tsumeshogi_bot::bsky::BskyAgent;
use bsky_tsumeshogi_bot::scraper::{scrape_everyday_links, scrape_tsumeshogi};
use std::collections::HashMap;
use std::env;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // prepare bsky agent
    let agent = BskyAgent::default();
    let session = agent
        .login(&env::var("BSKY_IDENTIFIER")?, &env::var("BSKY_PASSWORD")?)
        .await?;
    // collect uris from recent posts
    let mut uris = HashMap::new();
    let feeds = agent.get_feeds(&session.did).await?;
    for post in feeds.feed {
        if let Some(PostViewEmbedEnum::AppBskyEmbedExternalView(embed)) = post.post.embed {
            uris.insert(embed.external.uri, post.post.uri);
        }
    }
    // scrape and post
    for link in scrape_everyday_links().await?.iter().take(3).rev() {
        println!("{link}");
        if let Some(uri) = uris.get(link) {
            println!(" -> already posted at {uri}");
            continue;
        }
        let (ogp, _kif) = scrape_tsumeshogi(link).await?;
        let ogp = ogp.expect("failed to scrape OGP data");
        let thumb_data = if let Some(image) = &ogp.image {
            Some(reqwest::get(image).await?.bytes().await?.to_vec())
        } else {
            None
        };
        let embed = agent.embed_external(link, &ogp, thumb_data).await?;
        println!("{:#?}", agent.create_post(ogp.title, Some(embed)).await);
    }
    Ok(())
}
