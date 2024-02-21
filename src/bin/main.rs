use bsky_tsumeshogi_bot::bsky::{collect_uris, create_facets, BskyAgent};
use bsky_tsumeshogi_bot::scraper::{scrape_everyday_links, scrape_tsumeshogi};
use shogi_img::image::codecs::png::PngEncoder;
use shogi_img::shogi_core::Position;
use shogi_img::Generator;
use shogi_kifu_converter::parser;
use std::collections::HashMap;
use std::env;
use std::io::{BufWriter, Cursor};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // prepare bsky agent
    let identifier = env::var("BSKY_IDENTIFIER").expect("BSKY_IDENTIFIER is not set");
    let password = env::var("BSKY_PASSWORD").expect("BSKY_PASSWORD is not set");
    let agent = BskyAgent::default();
    let session = agent.login(&identifier, &password).await?;
    // collect uris from recent posts
    let mut uris = HashMap::new();
    let feeds = agent.get_feeds(&session.did).await?;
    for view_post in feeds.feed {
        for uri in collect_uris(&view_post.post) {
            uris.insert(uri, view_post.post.uri.clone());
        }
    }
    // scrape and post
    for link in scrape_everyday_links().await?.iter().take(3).rev() {
        println!("{link}");
        if let Some(uri) = uris.get(link) {
            println!(" -> already posted at {uri}");
            continue;
        }
        let (ogp, kif) = scrape_tsumeshogi(link).await?;
        let ogp = ogp.expect("failed to scrape OGP data");
        let text = format!("{}\n{}", ogp.title, link);
        let embed = if let Some(kif_url) = kif {
            let pos = parse_kif(&kif_url).await?;
            let mut cursor = Cursor::new(Vec::new());
            Generator::default()
                .generate(pos.initial_position())
                .write_with_encoder(PngEncoder::new(BufWriter::new(&mut cursor)))?;
            agent
                .embed_image(
                    cursor.into_inner(),
                    format!("sfen {}", pos.initial_position().to_sfen_owned()),
                )
                .await?
        } else {
            let thumb_data = if let Some(image) = &ogp.image {
                Some(reqwest::get(image).await?.bytes().await?.to_vec())
            } else {
                None
            };
            agent.embed_external(link, &ogp, thumb_data).await?
        };
        let facets = create_facets(text.clone(), link.clone());
        println!("{:#?}", agent.create_post(text, Some(embed), facets).await);
    }
    Ok(())
}

async fn parse_kif(url: &str) -> Result<Position, Box<dyn std::error::Error>> {
    let jkf = parser::parse_kif_str(
        &reqwest::get(url)
            .await?
            .text_with_charset("Shift_JIS")
            .await?,
    )?;
    Ok(Position::try_from(&jkf)?)
}
