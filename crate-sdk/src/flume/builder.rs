use common::{
    v1::types::{
        ParseMentions,
        components::{self, Components},
        flume::FlumeCreate,
        metadata::Metadata,
    },
    v2::types::{ChannelId, MessageId},
};
use futures::future::BoxFuture;

use crate::{Client, flume::FlumeWriter, prelude::*};

/// builder to create a new flume
pub struct FlumeBuilder<'a> {
    client: &'a Client,
    channel_id: ChannelId,
    reply_id: Option<MessageId>,
    mentions: ParseMentions,
    metadata: Option<Metadata>,
    components: Components<components::Create>,
}

impl<'a> FlumeBuilder<'a> {
    pub fn new(client: &'a Client, channel_id: ChannelId) -> Self {
        Self {
            client,
            channel_id,
            reply_id: None,
            mentions: ParseMentions::default(),
            metadata: None,
            components: Components::default(),
        }
    }

    pub fn reply_id(mut self, reply_id: MessageId) -> Self {
        self.reply_id = Some(reply_id);
        self
    }

    pub fn mentions(mut self, mentions: ParseMentions) -> Self {
        self.mentions = mentions;
        self
    }

    pub fn metadata(mut self, metadata: Metadata) -> Self {
        self.metadata = Some(metadata);
        self
    }

    pub fn components(mut self, components: Components<components::Create>) -> Self {
        self.components = components;
        self
    }

    pub async fn spawn(self) -> Result<FlumeWriter> {
        let create = FlumeCreate {
            reply_id: self.reply_id,
            mentions: self.mentions,
            metadata: self.metadata,
            components: self.components,
        };

        let message = self
            .client
            .http()
            .flume_create(self.channel_id, &create)
            .await?;
        let ctl = FlumeWriter::spawn(self.client, message);
        Ok(ctl)
    }
}

impl<'a> IntoFuture for FlumeBuilder<'a> {
    type Output = Result<FlumeWriter>;
    type IntoFuture = BoxFuture<'a, Self::Output>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.spawn())
    }
}
