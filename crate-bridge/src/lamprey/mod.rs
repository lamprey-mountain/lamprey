use std::collections::HashMap;

use common::v1::types::MessageSync;
use common::v1::types::presence::{Activity, Presence, Status};
use futures::StreamExt;
use sdk::syncer::{SyncerEvent, SyncerState};
use tokio::task::JoinSet;

use crate::bridge::{BridgeEvent, BridgeHandle, Portal, PortalHandle, PortalId};
use crate::config::LampreyConfig;
use crate::prelude::*;

// re export lamprey types
pub use common::v2::types::{ChannelId, MediaId, MessageId, RoomId, UserId};

mod interactions;

pub fn spawn(bridge: BridgeHandle, config: LampreyConfig) {
    tokio::spawn(Lamprey::connect(bridge, config));
}

struct Lamprey {
    bridge: BridgeHandle,
    client: sdk::Client,
    portal_tasks: JoinSet<(PortalId, Result<()>)>,
    portal_handles: HashMap<PortalId, PortalHandle>,
}

impl Lamprey {
    async fn connect(bridge: BridgeHandle, config: LampreyConfig) -> Result<()> {
        // TODO: build client from config
        let client = sdk::Client::builder()
            .api_url(config.api_url.clone())
            .sync_url(config.ws_url.clone().unwrap_or(config.api_url.clone()))
            .cdn_url(config.cdn_url.clone().unwrap_or(config.api_url.clone()))
            .token(config.token.load()?.to_string().into());

        let client = client
            .presence(Presence {
                status: Status::Online,
                activities: vec![Activity::Custom {
                    text: "bridging".to_string(),
                    clear_at: None,
                }],
            })
            .build()
            .await?;

        let me = Self {
            bridge,
            client,
            portal_tasks: JoinSet::new(),
            portal_handles: HashMap::new(),
        };
        me.start().await;

        Ok(())
    }

    async fn start(mut self) {
        let sync = self.client.syncer();
        let mut sub = sync.subscribe();
        let mut ctl = self.bridge.events.subscribe();
        sync.connect();

        loop {
            // TODO: handle cancellation
            tokio::select! {
                Some(event) = sub.next() => self.handle_syncer_event(&event),
                Ok(event) = ctl.recv() => self.handle_bridge_event(&event),
                Some(_) = self.portal_tasks.join_next() => todo!("handle dead portal"),
            }
        }
    }

    fn handle_syncer_event(&mut self, event: &SyncerEvent) {
        match event {
            SyncerEvent::Message(_) => {}
            SyncerEvent::Sync(sync) => match &**sync {
                // events relevant to realms
                MessageSync::RoomUpdate { room } => todo!(),
                MessageSync::ChannelCreate { channel } => todo!(),
                MessageSync::ChannelUpdate { channel } => todo!(),
                MessageSync::UserUpdate { user } => todo!(),
                MessageSync::RoomMemberCreate { member, user } => todo!(),
                MessageSync::RoomMemberUpdate { member, user } => todo!(),
                MessageSync::RoomMemberDelete { room_id, user_id } => todo!(),

                // events relevant to portals
                MessageSync::ChannelTyping {
                    channel_id,
                    user_id,
                    until,
                } => todo!(),
                MessageSync::MessageCreate { message } => todo!(),
                MessageSync::MessageUpdate { message } => todo!(),
                MessageSync::MessageDelete {
                    channel_id,
                    message_id,
                } => todo!(),
                MessageSync::MessageVersionDelete {
                    channel_id,
                    message_id,
                    version_id,
                } => todo!(),
                MessageSync::ReactionCreate {
                    user_id,
                    channel_id,
                    message_id,
                    key,
                } => todo!(),
                MessageSync::ReactionDelete {
                    user_id,
                    channel_id,
                    message_id,
                    key,
                } => todo!(),
                MessageSync::ReactionDeleteKey {
                    channel_id,
                    message_id,
                    key,
                } => todo!(),
                MessageSync::ReactionDeleteAll {
                    channel_id,
                    message_id,
                } => todo!(),

                _ => {}
            },
            SyncerEvent::StateChanged => {
                // TODO: log state changes
                match self.client.syncer().state() {
                    SyncerState::Disconnected => todo!(),
                    SyncerState::Waiting => todo!(),
                    SyncerState::Connecting => todo!(),
                    SyncerState::Authenticating => todo!(),
                    SyncerState::Resuming => todo!(),
                    SyncerState::Connected => todo!(),
                }
            }
        }
    }

    fn handle_bridge_event(&mut self, event: &BridgeEvent) {
        match event {
            BridgeEvent::RealmInit(realm) => todo!(),
            BridgeEvent::PortalInit(id, portal, handle) => {
                self.portal_tasks
                    .spawn(spawn_portal(*id, portal.clone(), handle.clone()));
            }
            BridgeEvent::PortalEvent(id, event) => {
                todo!("forward event to portal handle")
            }
            _ => todo!(),
        }
    }
}

async fn spawn_portal(
    id: PortalId,
    portal: Portal,
    handle: PortalHandle,
) -> (PortalId, Result<()>) {
    (id, spawn_portal_inner(portal, handle).await)
}

async fn spawn_portal_inner(portal: Portal, handle: PortalHandle) -> Result<()> {
    todo!()
}
