use std::{sync::Arc, time::Duration};

use common::{
    v1::types::{
        Message, MessageSync, MessageType,
        flume::{FlumeDeltaCreate, FlumeState},
    },
    v2::types::{ChannelId, MessageId},
};
use futures::StreamExt;
use tokio::{sync::watch, task::JoinHandle};

use crate::flume::FlumeReader;
use crate::{Client, http::Http};

/// allows writing to a flume
pub struct FlumeWriter {
    http: Http,
    client: Client,
    message: watch::Receiver<Arc<Message>>,
    ping_task: JoinHandle<()>,
    sync_task: JoinHandle<()>,
}

impl FlumeWriter {
    /// create a new FlumeController, spawning any necessary tasks
    pub(super) fn spawn(client: &Client, message: Message) -> Self {
        let channel_id = message.channel_id;
        let message_id = message.id;
        let http = client.http();
        let http_clone = client.http();
        let client_clone = client.clone();

        let message = Arc::new(message);
        let (tx, rx) = watch::channel(message);

        // TODO: add warning log if tasks fail

        let ping_task = tokio::spawn(async move {
            // in backend, flumes autocommit after 30 seconds
            let mut interval = tokio::time::interval(Duration::from_secs(20));
            loop {
                interval.tick().await;
                let _ = http_clone.flume_ping(channel_id, message_id).await;
            }
        });

        let syncer = client.syncer();
        let mut sync_stream = syncer.sync();
        let sync_task = tokio::spawn(async move {
            while let Some(sync) = sync_stream.next().await {
                match &*sync {
                    MessageSync::FlumeDelta {
                        channel_id: _,
                        message_id: _,
                        delta,
                    } => {
                        tx.send_modify(|m| {
                            let mut msg = (**m).clone();
                            match &mut msg.latest_version.message_type {
                                MessageType::DefaultMarkdown(m) | MessageType::ThreadInitial(m) => {
                                    m.components
                                        .apply_delta(delta.clone())
                                        .expect("TODO: better error handling")
                                }
                                _ => todo!("handle no components"),
                            };
                            *m = Arc::new(msg);
                        });
                    }
                    MessageSync::MessageUpdate { message } => {
                        if let Some(f) = &message.flume {
                            if f.state != FlumeState::Live {
                                tx.send_modify(|m| {
                                    let mut msg = (**m).clone();
                                    msg.flume.as_mut().expect("message always has flume").state =
                                        f.state;
                                    *m = Arc::new(msg);
                                });
                            }
                        }
                    }
                    _ => {}
                }
            }
        });

        Self {
            http,
            client: client_clone,
            message: rx,
            ping_task,
            sync_task,
        }
    }

    /// get the id of the message
    #[inline]
    pub fn message_id(&self) -> MessageId {
        self.message.borrow().id
    }

    /// get the id of the channel this message was sent in
    #[inline]
    pub fn channel_id(&self) -> ChannelId {
        self.message.borrow().channel_id
    }

    /// get the message that this flume is for
    pub fn message(&self) -> Arc<Message> {
        self.message.borrow().clone()
    }

    /// update the flume's content
    pub async fn update(&self, delta: FlumeDeltaCreate) {
        // TODO: error handling (at least a log)
        let _ = self
            .http
            .flume_delta(self.channel_id(), self.message_id(), &delta)
            .await;
    }

    /// commit the flume
    pub async fn commit(self) {
        // TODO: error handling (at least a log)
        let _ = self
            .http
            .flume_commit(self.channel_id(), self.message_id())
            .await;
    }

    /// get a reader for this flume
    #[inline]
    pub fn reader(&self) -> FlumeReader {
        FlumeReader::new(self.message.clone())
    }
}

impl Drop for FlumeWriter {
    fn drop(&mut self) {
        self.ping_task.abort();
        self.sync_task.abort();
    }
}
