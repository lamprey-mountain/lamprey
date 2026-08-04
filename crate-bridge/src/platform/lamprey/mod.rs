use std::collections::HashMap;

use common::v1::types::misc::UserIdReq;
use common::v1::types::presence::{Activity, Presence, Status};
use common::v1::types::{MessageSync, MessageType, RoomMemberPut};
use futures::StreamExt;
use sdk::http::{Http, MessageCreateOptions};
use sdk::syncer::SyncerEvent;
use time::OffsetDateTime;
use tokio::sync::{broadcast, oneshot};
use tokio::task::JoinSet;
use tracing::{debug, info, warn};

use crate::actor::bridge::BridgeCommand;
use crate::bridge_old::{
    BridgeEvent, BridgeHandle, Platform, PlatformHandle, Portal, PortalEvent, PortalHandle,
    PortalId,
};
use crate::config::LampreyConfig;
use crate::platform::lamprey::client::LampreyClient;
use crate::prelude::*;

// re export lamprey types
pub use common::v1::types::{
    ChannelId, MediaId, Mentions, Message, MessageAttachment, MessageAttachmentCreate,
    MessageAttachmentCreateType, MessageCreate, MessageId, ParseMentions, RoomId, RoomMember, User,
    UserId,
    embed::{Embed, EmbedCreate, EmbedType},
};
pub use common::v2::types::media::{Media, MediaCreate, MediaCreateSource};

mod client;
mod interactions;

pub fn spawn(bridge: BridgeHandle, config: LampreyConfig) -> PlatformHandle {
    let (tx, rx) = oneshot::channel();
    let task = tokio::spawn(Lamprey::connect(bridge, config, tx));
    PlatformHandle {
        name: "lamprey",
        ready: rx,
        task,
    }
}

struct Lamprey {
    bridge: BridgeHandle,
    client: sdk::Client,
    portal_tasks: JoinSet<(PortalId, Result<()>)>,
    portal_handles: HashMap<PortalId, PortalHandle>,
    portal_lookup: HashMap<ChannelId, PortalId>,
}

impl Lamprey {
    async fn connect(
        bridge: BridgeHandle,
        config: LampreyConfig,
        ready_tx: oneshot::Sender<()>,
    ) -> Result<()> {
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
        me.start(ready_tx).await;

        Ok(())
    }

    async fn start(mut self, ready_tx: oneshot::Sender<()>) {
        let sync = self.client.syncer();
        let mut sub = sync.subscribe();
        let mut ctl = self.bridge.events.subscribe();
        sync.connect();

        info!("connected");
        ready_tx.send(()).unwrap();

        loop {
            // TODO: handle cancellation
            tokio::select! {
                Some(event) = sub.next() => self.handle_syncer_event(&event).await.expect("TODO: better error handling"),
                Ok(event) = ctl.recv() => self.handle_bridge_event(&event),
                Some(result) = self.portal_tasks.join_next() => {
                    match result {
                        Ok((portal_id, Err(e))) => {
                            warn!(%portal_id, "portal task failed: {e:?}");
                            // TODO: try to restart portal task on failure
                            if let Some(_handle) = self.portal_handles.remove(&portal_id) {
                                // Potentially notify bridge/portal that it's dead?
                            }
                            self.portal_lookup.retain(|_, v| *v != portal_id);
                        }
                        Ok((portal_id, Ok(()))) => {
                            debug!(%portal_id, "portal task exited cleanly");
                            self.portal_handles.remove(&portal_id);
                            self.portal_lookup.retain(|_, v| *v != portal_id);
                        }
                        Err(e) => {
                            warn!("portal task join error: {e:?}");
                        }
                    }
                },
            }
        }
    }

    async fn handle_syncer_event(&mut self, event: &SyncerEvent) -> Result<()> {
        debug!("handle_syncer_event {event:?}");
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

                    // FIXME: allow sending messages containing !accept and !reject if theres no pending link
                    match &message.latest_version.message_type {
                        MessageType::DefaultMarkdown(m) => match m.content.as_deref() {
                            Some(c @ "!accept" | c @ "!reject") => {
                                let accepted = c == "!accept";
                                let channel =
                                    self.client.http().channel_get(message.channel_id).await?;
                                let Some(room_id) = channel.room_id else {
                                    let _ = self
                                        .client
                                        .http()
                                        .message_create(
                                            message.channel_id,
                                            &MessageCreate {
                                                content: Some(
                                                    "Only channels in rooms can be bridged"
                                                        .to_string(),
                                                ),
                                                ..Default::default()
                                            },
                                        )
                                        .await;
                                    return Ok(());
                                };
                                let _ = self
                                    .bridge
                                    .commands
                                    .send(BridgeCommand::LinkResponse {
                                        lamprey_room_id: room_id,
                                        lamprey_channel_id: message.channel_id,
                                        accepted,
                                    })
                                    .await;
                                // FIXME: send msg on bridge event portal created instead of here
                                let msg = if accepted {
                                    "portal successfully created!"
                                } else {
                                    "portal request denied"
                                };
                                let _ = self
                                    .client
                                    .http()
                                    .message_create(
                                        message.channel_id,
                                        &MessageCreate {
                                            content: Some(msg.to_string()),
                                            ..Default::default()
                                        },
                                    )
                                    .await;
                                return Ok(());
                            }
                            _ => {}
                        },
                        _ => {}
                    }

                    // PERF: cache this
                    let user = self
                        .client
                        .http()
                        .user_get(UserIdReq::UserId(message.author_id))
                        .await?;

                    // // TODO: fetch room member if message.room_id is some
                    // let room_member = self
                    //     .client
                    //     .http()
                    //     .room_member_get(room_id, UserIdReq::UserId(message.author_id))
                    //     .await?;

                    self.route_portal_event(
                        &message.channel_id,
                        PortalEvent::MessageCreate(bridge_old::MessageData::Lamprey {
                            message: Box::new(message.clone()),
                            user: Box::new(user.inner),
                            room_member: None,
                            info: Box::new(bridge_old::LampreyInfo {
                                cdn_url: self.client.http().cdn_url().clone(),
                            }),
                        }),
                    );
                }
                MessageSync::MessageUpdate { message } => {
                    if let Some(db_user) = self
                        .bridge
                        .db
                        .puppet_get_by_lamprey_id(message.author_id.to_string())
                        .await?
                    {
                        if db_user.source_platform != Platform::Lamprey {
                            return Ok(());
                        }
                    }

                    // PERF: cache this
                    let user = self
                        .client
                        .http()
                        .user_get(UserIdReq::UserId(message.author_id))
                        .await?;

                    self.route_portal_event(
                        &message.channel_id,
                        PortalEvent::MessageUpdate(bridge_old::MessageData::Lamprey {
                            message: Box::new(message.clone()),
                            user: Box::new(user.inner),
                            room_member: None,
                            info: Box::new(bridge_old::LampreyInfo {
                                cdn_url: self.client.http().cdn_url().clone(),
                            }),
                        }),
                    );
                }
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
            BridgeEvent::PortalInit(portal, handle) => {
                self.init_portal(portal, handle);
            }
            BridgeEvent::PortalCreated(portal) => {
                let handle = self.bridge.create_portal_handle(portal.id);
                self.init_portal(portal, &handle);
            }
            BridgeEvent::PortalEvent(id, event) => {
                if let Some(handle) = self.portal_handles.get(id) {
                    let _ = handle.events.send(Arc::new(event.clone()));
                }
            }
            BridgeEvent::LinkRequest { lamprey_channel_id } => {
                let channel_id = *lamprey_channel_id;
                let http = self.client.http().clone();
                tokio::spawn(async move {
                    let _ = http.message_create_with_options(
                        sdk::http::MessageCreateOptions {
                            channel_id,
                            body: MessageCreate {
                                // TODO: say which guild/channel is requesting to link
                                content: Some("A discord channel is requesting to link with this channel. Reply with !accept or !reject".to_string()),
                                ..Default::default()
                            },
                            nonce: None,
                            timestamp: None,
                        }
                    ).await;
                });
            }
            _ => {
                // TODO: handle more events
            }
        }
    }

    fn init_portal(&mut self, portal: &Portal, handle: &PortalHandle) {
        let channel_id = if let Some(lamprey) = &portal.lamprey {
            self.portal_lookup.insert(lamprey.channel_id, portal.id);
            lamprey.channel_id
        } else {
            // we aren't part of this bridge
            return;
        };
        self.portal_handles.insert(portal.id, handle.clone());
        self.portal_tasks.spawn(spawn_portal(
            portal.id,
            portal.clone(),
            handle.clone(),
            self.client.http(),
            channel_id,
        ));
    }
}

async fn spawn_portal(
    id: PortalId,
    portal: Portal,
    handle: PortalHandle,
    http: Http,
    channel_id: ChannelId,
) -> (PortalId, Result<()>) {
    (
        id,
        spawn_portal_inner(id, portal, handle, http, channel_id).await,
    )
}

async fn spawn_portal_inner(
    portal_id: PortalId,
    portal: Portal,
    handle: PortalHandle,
    http: Http,
    channel_id: ChannelId,
) -> Result<()> {
    let mut events = handle.events.subscribe();
    let ly = LampreyClient::new(http, handle.bridge.clone(), channel_id);

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
                warn!(%portal_id, n, "portal event receiver lagged, skipping");
                continue;
            }
            Err(broadcast::error::RecvError::Closed) => break,
        };

        match &*event {
            PortalEvent::Typing(_) => todo!(),
            PortalEvent::MessageCreate(data) => {
                let dm = match data {
                    bridge_old::MessageData::Lamprey { .. } => {
                        // don't send messages from lamprey back to lamprey
                        continue;
                    }
                    bridge_old::MessageData::Discord { message } => {
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
                if let Some(reference) = &dm.message_reference {
                    if dm.kind == serenity::all::MessageType::InlineReply
                        && reference.kind == serenity::all::MessageReferenceKind::Default
                    {
                        if let Some(discord_reply_id) = reference.message_id {
                            if let Some(msg) = handle
                                .bridge
                                .db
                                .message_get_by_discord_id(portal_id, discord_reply_id)
                                .await?
                            {
                                if let Some(lamprey_reply_id) = msg.lamprey_message_id {
                                    create.reply_id = Some(lamprey_reply_id);
                                }
                            }
                        }
                    }
                }

                // make sure the puppet is a room member, otherwise it won't be able to send any messages
                // PERF: don't send this request for every message, cache this
                if let Some(lamprey_cfg) = &portal.lamprey {
                    ly.http
                        .room_member_add(
                            lamprey_cfg.room_id,
                            UserIdReq::UserId(puppet.id),
                            &RoomMemberPut::default(),
                        )
                        .await?;
                }

                let sent_message = ly
                    .http
                    .for_puppet(puppet.id)?
                    .message_create_with_options(MessageCreateOptions {
                        channel_id: portal.lamprey.as_ref().unwrap().channel_id,
                        body: create,
                        nonce: None,
                        timestamp: Some(
                            OffsetDateTime::from_unix_timestamp_nanos(
                                dm.timestamp.timestamp_nanos_opt().unwrap() as i128,
                            )
                            .unwrap()
                            .into(),
                        ),
                    })
                    .await?;

                // FIXME: make sure i don't accidentally overwrite a row (race condition)
                handle
                    .bridge
                    .db
                    .message_create(
                        portal_id,
                        bridge_old::Message {
                            source_platform: Platform::Lamprey,
                            attachments: vec![], // FIXME: populate from sent_message
                            portal_id,
                            lamprey_message_id: Some(sent_message.id),
                            discord_message_id: Some(dm.id),
                        },
                    )
                    .await?;
            }
            PortalEvent::MessageUpdate(data) => {
                let dm = match data {
                    bridge_old::MessageData::Lamprey { .. } => {
                        // don't send edits from lamprey back to lamprey
                        continue;
                    }
                    bridge_old::MessageData::Discord { message } => message,
                };

                if let Some(msg) = handle
                    .bridge
                    .db
                    .message_get_by_discord_id(portal_id, dm.id)
                    .await?
                {
                    if let Some(lamprey_message_id) = msg.lamprey_message_id {
                        let patch = common::v1::types::MessagePatch {
                            content: Some(Some(dm.content.clone())),
                            // TODO: also edit attachments, embeds, components
                            attachments: None,
                            reply_id: None,
                            embeds: None,
                            metadata: None,
                            components: None,
                        };
                        ly.http
                            .message_edit(channel_id, lamprey_message_id, &patch)
                            .await?;
                    }
                }
            }
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
