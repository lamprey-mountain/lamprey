use std::collections::HashMap;

use common::v1::types::presence::{Activity, Presence, Status};
use common::v1::types::{MessageSync, PuppetCreate};
use common::v2::types::ApplicationId;
use common::v2::types::media::{MediaDoneParams, MediaReference};
use futures::StreamExt;
use sdk::http::{Http, MessageCreateOptions};
use sdk::syncer::{SyncerEvent, SyncerState};
use tokio::sync::broadcast;
use tokio::task::JoinSet;
use tracing::warn;

use crate::bridge::{
    BridgeEvent, BridgeHandle, Platform, Portal, PortalEvent, PortalHandle, PortalId,
};
use crate::config::LampreyConfig;
use crate::lamprey::client::{ImportUrl, LampreyClient};
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
                    if let Some(db_user) = self
                        .bridge
                        .db
                        .puppet_get_by_lamprey_id(message.author_id.to_string())
                        .await?
                    {
                        if db_user.source_platform != Platform::Lamprey {
                            // make sure not to get stuck in an infinite loop. only forward messages that came from us.
                            return Ok(());
                        }
                    }

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
            _ => {}, // TODO: handle more events
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

    // TODO: backfill should be a task that doesn't block the portal
    // HOWEVER, the portal should bridge messages until backfilling is done
    let mut last_id = portal.lamprey.as_ref().expect("handle None").last_id;
    loop {
        let messages = ly.fetch_after(last_id).await?;

        // break if messages is empty
        let Some(last) = messages.last() else {
            break;
        };

        // try to forward/bridge message. skip if its already bridged.

        // TODO: update db -> portal -> lamprey_last_id
        last_id = last.id;
        // TODO: every time i insert/update a row in the "message" table, also update last_id
    }

    loop {
        let event = match events.recv().await {
            Ok(e) => e,
            Err(broadcast::error::RecvError::Lagged(n)) => {
                warn!(portal_id, n, "portal event receiver lagged, skipping");
                continue;
            }
            Err(broadcast::error::RecvError::Closed) => break,
        };

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

                // TODO: ly -> async fn process_discord_message(&self, ...) -> Result<MessageCreate>
                let mut create = MessageCreate {
                    content: Some(dm.content.clone()),
                    attachments: vec![],
                    metadata: None,
                    reply_id: None,
                    embeds: vec![],
                    mentions: Default::default(),
                    components: None,
                    ephemeral: false,
                };

                // TODO: reformat text (mentions, mostly)
                // see mentions::convert_discord_mentions_to_lamprey

                // populate reply_id
                // ...
                if let Some(reference) = &dm.message_reference {
                    // match reference.kind {
                    //     serenity::all::MessageReferenceKind::Default => {},
                    //     serenity::all::MessageReferenceKind::Forward => {},
                    //     _ => {},
                    // }
                    if let Some(referenced_message) = &dm.referenced_message {
                        // TODO: need a way to look up the bridged message id for the reference
                        // handle.bridge.db.message_get_by_discord_id(...)
                    }
                }

                let sent_message = ly
                    .http
                    .for_puppet(puppet.id)?
                    .message_create_with_options(MessageCreateOptions {
                        channel_id: portal.lamprey.as_ref().unwrap().channel_id,
                        body: create,
                        nonce: None,
                        timestamp: None, // FIXME: timestamp massaging
                    })
                    .await?;

                // FIXME: make sure i don't accidentally overwrite a row (race condition)
                handle
                    .bridge
                    .db
                    .message_create(
                        portal_id,
                        bridge::Message {
                            source_platform: Platform::Lamprey,
                            attachments: vec![], // FIXME: populate from sent_message
                            portal_id,
                            lamprey_message_id: Some(sent_message.id),
                            discord_message_id: Some(dm.id),
                        },
                    )
                    .await?;
            }
            PortalEvent::MessageUpdate(data) => todo!(),
            PortalEvent::MessageDelete(_) => todo!(),
            PortalEvent::ReactionCreate(_, _, _) => todo!(),
            PortalEvent::ReactionDelete(_, _, _) => todo!(),
            PortalEvent::ReactionDeleteEmoji(_, _) => todo!(),
            PortalEvent::ReactionDeleteAll(_, _) => todo!(),
        }
    }

    // log (warn?) on exit?

    Ok(())
}
