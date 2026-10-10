use std::sync::{Arc, atomic::AtomicU8};

use crate::{
    prelude::*,
    syncer::{Syncer, SyncerHandle, SyncerState},
};
use common::v1::types::{SessionToken, presence::Presence};
use reqwest::Url;
use tokio::sync::{broadcast, mpsc};

#[derive(Default)]
pub struct SyncerBuilder {
    sync_url: Option<Url>,
    token: Option<SessionToken>,
    presence: Option<Presence>,
}

impl SyncerBuilder {
    pub fn sync_url(mut self, url: Url) -> Self {
        self.sync_url = Some(url);
        self
    }

    pub fn token(mut self, token: SessionToken) -> Self {
        self.token = Some(token);
        self
    }

    pub fn presence(mut self, presence: Presence) -> Self {
        self.presence = Some(presence);
        self
    }

    pub fn build(self) -> Result<SyncerHandle> {
        let (cmd_tx, cmd_rx) = mpsc::channel(100);
        let (evt_tx, _) = broadcast::channel(100);

        let syncer = Syncer {
            state: AtomicU8::new(SyncerState::Disconnected.into()),
            client: None,
            resume: None,
            rx: cmd_rx,
            tx: evt_tx.clone(),
        };

        let token = self
            .token
            .ok_or_else(|| Error::MissingBuilderField("token".to_string()))?;
        let base_url = self
            .sync_url
            .ok_or_else(|| Error::MissingBuilderField("sync_url".to_string()))?;

        tokio::spawn(syncer.run(token, base_url));

        Ok(SyncerHandle {
            state: Arc::new(AtomicU8::new(SyncerState::Connecting.into())),
            tx: cmd_tx,
            rx: evt_tx.subscribe(),
        })
    }
}
