use select::document::Document;
use select::predicate::{Attr, Name, Predicate};

pub async fn scrape_everyday_links() -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let res = reqwest::get("https://www.shogi.or.jp/tsume_shogi/everyday/").await?;
    let doc = Document::from(res.text().await?.as_str());

    let mut ret = Vec::new();
    if let Some(list) = doc
        .find(Attr("id", "contents").descendant(Name("ul")))
        .next()
    {
        for node in list.find(Name("li")) {
            if let Some(link) = node.find(Name("a")).next() {
                if let Some(href) = link.attr("href") {
                    ret.push(href.into());
                }
            }
        }
    }
    Ok(ret)
}
