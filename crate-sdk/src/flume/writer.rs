use std::time::Duration;

use common::{
    v1::types::{Message, flume::FlumeDeltaCreate},
    v2::types::ChannelId,
};

use crate::flume::FlumeReader;
use crate::{Client, http::Http};

/// allows writing to a flume
pub struct FlumeWriter {
    http: Http,
    message: Message,
}

impl FlumeWriter {
    /// create a new FlumeController, spawning any necessary tasks
    pub(super) fn spawn(client: &Client, message: Message) -> Self {
        let channel_id = message.channel_id;
        let message_id = message.id;
        let http = client.http();
        let http_clone = client.http();

        // TODO: warning log if ping task fails
        let _ping_task = tokio::spawn(async move {
            // in backend, flumes autocommit after 30 seconds
            let mut interval = tokio::time::interval(Duration::from_secs(20));
            loop {
                interval.tick().await;
                let _ = http_clone.flume_ping(channel_id, message_id).await;
            }
        });

        Self { http, message }
    }

    /// get the id of the channel this message was sent in
    #[inline]
    pub fn channel_id(&self) -> ChannelId {
        self.message.channel_id
    }

    /// get the message that this flume is for
    #[inline]
    pub fn message(&self) -> &Message {
        &self.message
    }

    /// update the flume's content
    pub async fn update(&self, delta: FlumeDeltaCreate) {
        // TODO: error handling
        let _ = self
            .http
            .flume_delta(self.channel_id(), self.message.id, &delta)
            .await;
    }

    /// commit the flume
    pub async fn commit(self) {
        // TODO: error handling
        let _ = self
            .http
            .flume_commit(self.channel_id(), self.message.id)
            .await;
    }

    /// get a reader for this flume
    #[inline]
    pub fn reader(&self) -> FlumeReader {
        todo!()
    }
}
