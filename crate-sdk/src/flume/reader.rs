use common::v1::types::{
    Message, MessageType,
    components::{Canonical, Components},
    flume::FlumeState,
};
use std::sync::Arc;
use tokio::sync::watch;

/// a live updating message
#[derive(Clone)]
pub struct FlumeReader {
    rx: watch::Receiver<Arc<Message>>,
}

impl FlumeReader {
    pub(super) fn new(rx: watch::Receiver<Arc<Message>>) -> Self {
        Self { rx }
    }

    /// get the message that this flume is for
    pub fn message(&self) -> Arc<Message> {
        self.rx.borrow().clone()
    }

    /// get the flume's current content (components)
    pub fn components(&self) -> Components<Canonical> {
        match &self.rx.borrow().latest_version.message_type {
            MessageType::DefaultMarkdown(m) | MessageType::ThreadInitial(m) => m.components.clone(),
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
