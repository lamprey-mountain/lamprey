use std::{sync::Arc, time::Duration};

use common::v1::types::{
    Message, MessageSync, MessageType,
    components::{Canonical, Components},
    flume::FlumeState,
};
use futures::StreamExt;
use tokio::sync::watch;

use crate::Client;

/// a live updating message
pub struct FlumeReader {
    rx: watch::Receiver<Message>,
}

impl FlumeReader {
    pub(super) fn spawn(client: &Client, message: Message) -> Self {
        let flume_message_id = message.id;
        let mut syncer = client.syncer().sync();
        let (tx, rx) = watch::channel(message);

        // TODO: warning log if sync task fails
        let _sync_task = tokio::spawn(async move {
            while let Some(sync) = syncer.next().await {
                match &*sync {
                    MessageSync::FlumeDelta {
                        channel_id: _,
                        message_id,
                        delta,
                    } if message_id == &flume_message_id => {
                        tx.send_modify(|m| match &mut m.latest_version.message_type {
                            MessageType::DefaultMarkdown(m) => m
                                .components
                                .apply_delta(delta.clone())
                                .expect("TODO: better error handling"),
                            _ => todo!("handle no components"),
                        });
                    }
                    MessageSync::MessageUpdate { message } => {
                        if let Some(f) = &message.flume {
                            if f.state != FlumeState::Live {
                                tx.send_modify(|m| {
                                    m.flume.as_mut().expect("message always has flume").state =
                                        f.state
                                });
                            }
                        }
                    }

                    _ => {}
                }
            }
        });

        Self { rx }
    }

    /// get the message that this flume is for
    ///
    /// clones the message
    pub fn message(&self) -> Message {
        // PERF: don't clone
        self.rx.borrow().to_owned()
    }

    /// get the flume's current content (components)
    pub fn components(&self) -> Components<Canonical> {
        match self.message().latest_version.message_type {
            MessageType::DefaultMarkdown(m) => m.components,
            _ => todo!("handle no components"),
        }
    }

    /// completes when the flume is committed
    ///
    /// returns `true` if the flume was manually committed, and `false` if autocommitted (ie. due to timeout)
    pub async fn finished(&self) -> bool {
        let mut rx = self.rx.clone();
        let message = rx
            .wait_for(|m| {
                m.flume.as_ref().expect("message always has flume").state != FlumeState::Live
            })
            .await
            .expect("TODO: better error handling");
        message
            .flume
            .as_ref()
            .expect("message always has flume")
            .state
            == FlumeState::Committed
    }
}
