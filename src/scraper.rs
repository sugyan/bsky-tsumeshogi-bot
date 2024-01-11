use anyhow::Result;
use regex::Regex;
use select::document::Document;
use select::predicate::{Attr, Name, Predicate};
use std::collections::HashMap;

#[derive(Debug)]
pub struct Ogp {
    pub title: String,
    pub description: String,
    pub image: Option<String>,
}

pub async fn scrape_everyday_links() -> Result<Vec<String>> {
    let res = reqwest::get("https://www.shogi.or.jp/tsume_shogi/everyday/").await?;
    let doc = Document::from(res.text().await?.as_str());
    collect_everyday_links(&doc)
}

pub async fn scrape_tsumeshogi(link: &str) -> Result<(Option<Ogp>, Option<String>)> {
    let res = reqwest::get(link).await?;
    let doc = Document::from(res.text().await?.as_str());
    Ok((extract_ogp(&doc), extract_kif(&doc)))
}

fn collect_everyday_links(doc: &Document) -> Result<Vec<String>> {
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

// og:* attributes from meta tags
fn extract_ogp(doc: &Document) -> Option<Ogp> {
    let mut hm = HashMap::new();
    for meta in doc.find(Name("meta")) {
        if let Some(property) = meta.attr("property") {
            if let Some(key) = property.strip_prefix("og:") {
                if let Some(value) = meta.attr("content") {
                    hm.insert(key, value);
                }
            }
        }
    }
    if ["title", "description"]
        .iter()
        .all(|key| hm.contains_key(key))
    {
        Some(Ogp {
            title: hm["title"].into(),
            description: hm["description"].into(),
            image: hm.get("image").map(|&s| s.into()),
        })
    } else {
        None
    }
}

// kif link from script tags
fn extract_kif(doc: &Document) -> Option<String> {
    let re = Regex::new(r"'(https://www\.shogi\.or\.jp/tsume_shogi/data/[^/]+\.kif)'")
        .expect("regex should be valid");
    doc.find(Name("script"))
        .find_map(|script| re.captures(&script.text()).map(|caps| caps[1].to_string()))
}
