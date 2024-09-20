use crate::scraper::Ogp;
use bsky_sdk::api;
use bsky_sdk::api::types::TryFromUnknown;
use bsky_sdk::api::types::{string::Datetime, Union};
use bsky_sdk::{BskyAgent, Result};
use std::ops::Deref;

pub struct BotAgent {
    agent: BskyAgent,
}

impl Deref for BotAgent {
    type Target = BskyAgent;

    fn deref(&self) -> &Self::Target {
        &self.agent
    }
}

impl BotAgent {
    pub async fn new() -> Result<Self> {
        Ok(Self {
            agent: BskyAgent::builder().build().await?,
        })
    }
    pub async fn get_feeds(
        &self,
        actor: &str,
    ) -> Result<api::app::bsky::feed::get_author_feed::Output> {
        Ok(self
            .agent
            .api
            .app
            .bsky
            .feed
            .get_author_feed(
                api::app::bsky::feed::get_author_feed::ParametersData {
                    actor: actor.parse().expect("invalid actor"),
                    cursor: None,
                    filter: Some("posts_no_replies".into()),
                    limit: 20.try_into().ok(),
                }
                .into(),
            )
            .await?)
    }
    pub async fn embed_image(
        &self,
        data: Vec<u8>,
        alt: String,
    ) -> Result<Union<api::app::bsky::feed::post::RecordEmbedRefs>> {
        let output = self.agent.api.com.atproto.repo.upload_blob(data).await?;
        Ok(Union::Refs(
            api::app::bsky::feed::post::RecordEmbedRefs::AppBskyEmbedImagesMain(Box::new(
                api::app::bsky::embed::images::MainData {
                    images: vec![api::app::bsky::embed::images::ImageData {
                        alt,
                        aspect_ratio: None,
                        image: output.data.blob,
                    }
                    .into()],
                }
                .into(),
            )),
        ))
    }
    pub async fn embed_external(
        &self,
        uri: &str,
        ogp: &Ogp,
        thumb_data: Option<Vec<u8>>,
    ) -> Result<Union<api::app::bsky::feed::post::RecordEmbedRefs>> {
        let thumb = if let Some(data) = thumb_data {
            let output = self.agent.api.com.atproto.repo.upload_blob(data).await?;
            Some(output.data.blob)
        } else {
            None
        };
        Ok(Union::Refs(
            api::app::bsky::feed::post::RecordEmbedRefs::AppBskyEmbedExternalMain(Box::new(
                api::app::bsky::embed::external::MainData {
                    external: api::app::bsky::embed::external::ExternalData {
                        description: ogp.description.clone(),
                        thumb,
                        title: ogp.title.clone(),
                        uri: uri.into(),
                    }
                    .into(),
                }
                .into(),
            )),
        ))
    }
    pub async fn create_post(
        &self,
        text: String,
        embed: Option<Union<api::app::bsky::feed::post::RecordEmbedRefs>>,
        facets: Option<Vec<api::app::bsky::richtext::facet::Main>>,
    ) -> Result<api::com::atproto::repo::create_record::Output> {
        self.agent
            .create_record(api::app::bsky::feed::post::RecordData {
                created_at: Datetime::now(),
                embed,
                entities: None,
                facets,
                labels: None,
                langs: Some(vec!["ja".parse().expect("invalid language")]),
                reply: None,
                tags: None,
                text,
            })
            .await
    }
}

pub fn create_facets(
    text: String,
    uri: String,
) -> Option<Vec<api::app::bsky::richtext::facet::Main>> {
    text.find(&uri).map(|pos| {
        let index = api::app::bsky::richtext::facet::ByteSliceData {
            byte_end: pos + uri.len(),
            byte_start: pos,
        };
        vec![api::app::bsky::richtext::facet::MainData {
            features: vec![Union::Refs(
                api::app::bsky::richtext::facet::MainFeaturesItem::Link(Box::new(
                    api::app::bsky::richtext::facet::LinkData { uri }.into(),
                )),
            )],
            index: index.into(),
        }
        .into()]
    })
}

pub fn collect_uris(post_view: &api::app::bsky::feed::defs::PostView) -> Vec<String> {
    let mut ret = Vec::new();
    if let Ok(record) =
        api::app::bsky::feed::post::Record::try_from_unknown(post_view.record.clone())
    {
        // external embed
        if let Some(Union::Refs(
            api::app::bsky::feed::post::RecordEmbedRefs::AppBskyEmbedExternalMain(external),
        )) = &record.embed
        {
            ret.push(external.external.uri.clone());
        }
        // link facet feature
        if let Some(facets) = &record.facets {
            for facet in facets {
                for feature in &facet.features {
                    if let Union::Refs(api::app::bsky::richtext::facet::MainFeaturesItem::Link(
                        link,
                    )) = feature
                    {
                        ret.push(link.uri.clone());
                    }
                }
            }
        }
    }
    ret
}
