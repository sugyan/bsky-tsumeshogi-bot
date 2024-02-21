use crate::scraper::Ogp;
use atrium_api::agent::store::MemorySessionStore;
use atrium_api::agent::AtpAgent;
use atrium_api::types::string::Datetime;
use atrium_xrpc_client::reqwest::ReqwestClient;
use std::sync::{Arc, RwLock};

pub struct BskyAgent {
    agent: AtpAgent<MemorySessionStore, ReqwestClient>,
    session: Arc<RwLock<Option<atrium_api::com::atproto::server::create_session::Output>>>,
}

impl BskyAgent {
    pub async fn login(
        &self,
        identifier: &str,
        password: &str,
    ) -> Result<
        atrium_api::com::atproto::server::create_session::Output,
        atrium_api::xrpc::error::Error<atrium_api::com::atproto::server::create_session::Error>,
    > {
        let output = self.agent.login(identifier, password).await;
        if let Ok(output) = &output {
            if let Ok(mut session) = self.session.write() {
                session.replace(output.clone());
            }
        }
        output
    }
    pub async fn get_feeds(
        &self,
        actor: &str,
    ) -> Result<
        atrium_api::app::bsky::feed::get_author_feed::Output,
        atrium_api::xrpc::error::Error<atrium_api::app::bsky::feed::get_author_feed::Error>,
    > {
        self.agent
            .api
            .app
            .bsky
            .feed
            .get_author_feed(atrium_api::app::bsky::feed::get_author_feed::Parameters {
                actor: actor.parse().expect("invalid actor"),
                cursor: None,
                filter: Some("posts_no_replies".into()),
                limit: 20.try_into().ok(),
            })
            .await
    }
    pub async fn embed_image(
        &self,
        data: Vec<u8>,
        alt: String,
    ) -> Result<
        atrium_api::app::bsky::feed::post::RecordEmbedEnum,
        atrium_api::xrpc::error::Error<atrium_api::com::atproto::repo::upload_blob::Error>,
    > {
        let output = self.agent.api.com.atproto.repo.upload_blob(data).await?;
        Ok(
            atrium_api::app::bsky::feed::post::RecordEmbedEnum::AppBskyEmbedImagesMain(Box::new(
                atrium_api::app::bsky::embed::images::Main {
                    images: vec![atrium_api::app::bsky::embed::images::Image {
                        alt,
                        aspect_ratio: None,
                        image: output.blob,
                    }],
                },
            )),
        )
    }
    pub async fn embed_external(
        &self,
        uri: &str,
        ogp: &Ogp,
        thumb_data: Option<Vec<u8>>,
    ) -> Result<
        atrium_api::app::bsky::feed::post::RecordEmbedEnum,
        atrium_api::xrpc::error::Error<atrium_api::com::atproto::repo::upload_blob::Error>,
    > {
        let thumb = if let Some(data) = thumb_data {
            let output = self.agent.api.com.atproto.repo.upload_blob(data).await?;
            Some(output.blob)
        } else {
            None
        };
        Ok(
            atrium_api::app::bsky::feed::post::RecordEmbedEnum::AppBskyEmbedExternalMain(Box::new(
                atrium_api::app::bsky::embed::external::Main {
                    external: atrium_api::app::bsky::embed::external::External {
                        description: ogp.description.clone(),
                        thumb,
                        title: ogp.title.clone(),
                        uri: uri.into(),
                    },
                },
            )),
        )
    }
    pub async fn create_post(
        &self,
        text: String,
        embed: Option<atrium_api::app::bsky::feed::post::RecordEmbedEnum>,
        facets: Option<Vec<atrium_api::app::bsky::richtext::facet::Main>>,
    ) -> Result<
        atrium_api::com::atproto::repo::create_record::Output,
        atrium_api::xrpc::error::Error<atrium_api::com::atproto::repo::create_record::Error>,
    > {
        let repo = atrium_api::types::string::AtIdentifier::Did(
            self.session.read().unwrap().as_ref().unwrap().did.clone(),
        );
        self.agent
            .api
            .com
            .atproto
            .repo
            .create_record(atrium_api::com::atproto::repo::create_record::Input {
                collection: "app.bsky.feed.post".parse().expect("invalid collection"),
                record: atrium_api::records::Record::AppBskyFeedPost(Box::new(
                    atrium_api::app::bsky::feed::post::Record {
                        created_at: Datetime::now(),
                        embed,
                        entities: None,
                        facets,
                        labels: None,
                        langs: Some(vec!["ja".parse().expect("invalid language")]),
                        reply: None,
                        tags: None,
                        text,
                    },
                )),
                repo,
                rkey: None,
                swap_commit: None,
                validate: None,
            })
            .await
    }
}

impl Default for BskyAgent {
    fn default() -> Self {
        Self {
            agent: AtpAgent::new(
                ReqwestClient::new("https://bsky.social"),
                MemorySessionStore::default(),
            ),
            session: Arc::new(RwLock::new(None)),
        }
    }
}

pub fn create_facets(
    text: String,
    uri: String,
) -> Option<Vec<atrium_api::app::bsky::richtext::facet::Main>> {
    text.find(&uri).map(|pos| {
        let index = atrium_api::app::bsky::richtext::facet::ByteSlice {
            byte_end: pos + uri.len(),
            byte_start: pos,
        };
        vec![atrium_api::app::bsky::richtext::facet::Main {
            features: vec![
                atrium_api::app::bsky::richtext::facet::MainFeaturesItem::Link(Box::new(
                    atrium_api::app::bsky::richtext::facet::Link { uri },
                )),
            ],
            index,
        }]
    })
}

pub fn collect_uris(post_view: &atrium_api::app::bsky::feed::defs::PostView) -> Vec<String> {
    let mut ret = Vec::new();
    if let atrium_api::records::Record::AppBskyFeedPost(record) = &post_view.record {
        // external embed
        if let Some(atrium_api::app::bsky::feed::post::RecordEmbedEnum::AppBskyEmbedExternalMain(
            external,
        )) = &record.embed
        {
            ret.push(external.external.uri.clone());
        }
        // link facet feature
        if let Some(facets) = &record.facets {
            for facet in facets {
                for feature in &facet.features {
                    if let atrium_api::app::bsky::richtext::facet::MainFeaturesItem::Link(link) =
                        feature
                    {
                        ret.push(link.uri.clone());
                    }
                }
            }
        }
    }
    ret
}
