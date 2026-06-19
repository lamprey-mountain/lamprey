use std::collections::HashMap;

use common::v1::types::presence::{Activity, Presence, Status};
use common::v1::types::{MessageSync, PuppetCreate};
use common::v2::types::ApplicationId;
use common::v2::types::media::MediaDoneParams;
use futures::StreamExt;
use sdk::http::{Http, MessageCreateOptions};
use sdk::syncer::{SyncerEvent, SyncerState};
use tokio::task::JoinSet;

use crate::bridge::{
    BridgeEvent, BridgeHandle, Platform, Portal, PortalEvent, PortalHandle, PortalId,
};
use crate::config::LampreyConfig;
use crate::lamprey::client::LampreyClient;
use crate::prelude::*;

// re export lamprey types
pub use common::v1::types::{
    ChannelId, MediaId, Message, MessageAttachment, MessageAttachmentCreate,
    MessageAttachmentCreateType, MessageCreate, MessageId, RoomId, UserId,
    embed::{Embed, EmbedCreate, EmbedType},
};
pub use common::v2::types::media::{Media, MediaCreate, MediaCreateSource};

mod client;
mod interactions;

pub fn spawn(bridge: BridgeHandle, config: LampreyConfig) {
    tokio::spawn(Lamprey::connect(bridge, config));
}

struct Lamprey {
    bridge: BridgeHandle,
    client: sdk::Client,
    portal_tasks: JoinSet<(PortalId, Result<()>)>,
    portal_handles: HashMap<PortalId, PortalHandle>,
    portal_lookup: HashMap<ChannelId, PortalId>,
}

impl Lamprey {
    async fn connect(bridge: BridgeHandle, config: LampreyConfig) -> Result<()> {
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
            portal_lookup: HashMap::new(),
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
                Some(event) = sub.next() => self.handle_syncer_event(&event).await.expect("TODO: better error handling"),
                Ok(event) = ctl.recv() => self.handle_bridge_event(&event),
                Some(_) = self.portal_tasks.join_next() => todo!("handle dead portal"),
            }
        }
    }

    async fn handle_syncer_event(&mut self, event: &SyncerEvent) -> Result<()> {
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
                    ..
                } => {
                    if let Some(user) = self
                        .bridge
                        .db
                        .puppet_get_by_lamprey_id(user_id.to_string())
                        .await?
                    {
                        self.route_portal_event(channel_id, PortalEvent::Typing(user));
                    }
                }
                MessageSync::MessageCreate { message } => {
                    // let portal_id = self.portal_lookup.get(&message.channel_id).unwrap();
                    // let _nothing = self
                    //     .bridge
                    //     .db
                    //     .message_create(
                    //         *portal_id,
                    //         bridge::Message {
                    //             source_platform: Platform::Lamprey,
                    //             source_id: message.id.to_string(),
                    //             attachments: vec![], // this will be filled in later
                    //         },
                    //     )
                    //     .await?;
                    self.route_portal_event(
                        &message.channel_id,
                        PortalEvent::MessageCreate(bridge::MessageData::Lamprey(message.clone())),
                    );
                }
                // MessageSync::MessageUpdate { message } => {
                //     self.route_portal_event(
                //         &message.channel_id,
                //         PortalEvent::MessageUpdate {
                //             source_message_id: message.id.to_string(),
                //             content: message.content.clone(),
                //         },
                //     );
                // }
                // MessageSync::MessageDelete {
                //     channel_id,
                //     message_id,
                // } => {
                //     self.route_portal_event(
                //         channel_id,
                //         PortalEvent::MessageDelete {
                //             source_message_id: message_id.to_string(),
                //         },
                //     );
                // }
                // MessageSync::ReactionCreate { channel_id, .. } => {
                //     self.route_portal_event(channel_id, PortalEvent::ReactionCreate);
                // }
                // MessageSync::ReactionDelete { channel_id, .. } => {
                //     self.route_portal_event(channel_id, PortalEvent::ReactionDelete);
                // }
                _ => {}
            },
            SyncerEvent::StateChanged => {
                // TODO: log state changes
            }
        }

        Ok(())
    }

    fn route_portal_event(&self, channel_id: &ChannelId, event: PortalEvent) {
        if let Some(portal_id) = self.portal_lookup.get(channel_id) {
            if let Some(handle) = self.portal_handles.get(portal_id) {
                let _ = handle.events.send(Arc::new(event));
            }
        }
    }

    fn handle_bridge_event(&mut self, event: &BridgeEvent) {
        match event {
            BridgeEvent::PortalInit(id, portal, handle) => {
                if let Some(lamprey) = &portal.lamprey {
                    self.portal_lookup.insert(lamprey.channel_id, *id);
                }
                self.portal_handles.insert(*id, handle.clone());
                self.portal_tasks.spawn(spawn_portal(
                    *id,
                    portal.clone(),
                    handle.clone(),
                    self.client.http(),
                ));
            }
            BridgeEvent::PortalEvent(id, event) => {
                if let Some(handle) = self.portal_handles.get(id) {
                    let _ = handle.events.send(Arc::new(event.clone()));
                }
            }
            _ => todo!(),
        }
    }
}

async fn spawn_portal(
    id: PortalId,
    portal: Portal,
    handle: PortalHandle,
    http: Http,
) -> (PortalId, Result<()>) {
    (id, spawn_portal_inner(id, portal, handle, http).await)
}

async fn spawn_portal_inner(
    portal_id: PortalId,
    portal: Portal,
    handle: PortalHandle,
    http: Http,
) -> Result<()> {
    let mut events = handle.events.subscribe();
    let ly = LampreyClient::new(http, handle.bridge.clone());

    loop {
        let event = events.recv().await?;
        match &*event {
            PortalEvent::Typing(_) => todo!(),
            PortalEvent::MessageCreate(data) => {
                let dm = match data {
                    bridge::MessageData::Lamprey(_) => {
                        // don't send messages from lamprey back to lamprey
                        continue;
                    }
                    bridge::MessageData::Discord(message) => {
                        // TODO: filter out messages on the discord side
                        // message.webhook_id == Some(webhook_id)
                        message
                    }
                };

                let puppet = ly.sync_puppet_discord(dm).await?;

                let mut create = MessageCreate {
                    content: None,
                    attachments: vec![],
                    metadata: None,
                    reply_id: None,
                    embeds: vec![],
                    mentions: Default::default(),
                    components: None,
                    ephemeral: false,
                };

                // TODO: process attachments
                // PERF: process attachments in parallel
                // create.attachments.push()

                // TODO: process embeds

                // TODO: reformat text (mentions, mostly)

                // TODO: populate reply_id

                ly.http
                    .for_puppet(puppet.id)?
                    .message_create_with_options(MessageCreateOptions {
                        channel_id: todo!(),
                        body: create,
                        nonce: None,
                        timestamp: todo!(),
                    })
                    .await?;

                handle
                    .bridge
                    .db
                    .message_create(
                        portal_id,
                        bridge::Message {
                            source_platform: todo!(),
                            source_id: todo!(),
                            attachments: todo!(),
                        },
                    )
                    .await?;
            }
            PortalEvent::MessageUpdate(_, data) => todo!(),
            PortalEvent::MessageDelete(_) => todo!(),
            PortalEvent::ReactionCreate(_, _, _) => todo!(),
            PortalEvent::ReactionDelete(_, _, _) => todo!(),
            PortalEvent::ReactionDeleteEmoji(_, _) => todo!(),
            PortalEvent::ReactionDeleteAll(_, _) => todo!(),
        }
    }
}
