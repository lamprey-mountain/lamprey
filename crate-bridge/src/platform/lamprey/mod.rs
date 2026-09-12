use std::collections::HashMap;

use common::v1::types::misc::UserIdReq;
use common::v1::types::presence::{Activity, Presence, Status};
use common::v1::types::{MessageSync, MessageType, PaginationQuery};
use futures::StreamExt;
use sdk::syncer::SyncerEvent;
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinSet;
use tracing::{debug, info, trace, warn};

use crate::actor::bridge::BridgeCommand;
use crate::bridge_old::{
    BridgeEvent, BridgeHandle, Platform, PlatformHandle, Portal, PortalEvent, PortalHandle,
    PortalId, Realm, RealmEvent, RealmHandle, RealmId,
};
use crate::config::LampreyConfig;
use crate::platform::lamprey::portal::LampreyPortal;
use crate::platform::lamprey::presence::{PresenceEvent, PresenceRefreshActor};
use crate::platform::lamprey::realm::LampreyRealm;
use crate::prelude::*;
use crate::types::ChannelData;

// re export lamprey types
pub use common::v1::types::{
    Channel, ChannelId, ChannelType, MediaId, Mentions, Message, MessageAttachment,
    MessageAttachmentCreate, MessageAttachmentCreateType, MessageCreate, MessageId, ParseMentions,
    RoleId, Room, RoomId, RoomMember, User, UserId,
    embed::{Embed, EmbedCreate, EmbedType},
    reaction::ReactionKey,
};
pub use common::v2::types::media::{Media, MediaCreate, MediaCreateSource};

mod client;
mod interactions;
mod portal;
mod presence;
mod realm;

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
    realm_tasks: JoinSet<(RealmId, Result<()>)>,
    presence_tx: mpsc::UnboundedSender<PresenceEvent>,
    portal_handles: HashMap<PortalId, PortalHandle>,
    portal_lookup: HashMap<ChannelId, PortalId>,
    portal_data: HashMap<PortalId, Portal>,
    realm_handles: HashMap<RealmId, RealmHandle>,
    realm_lookup: HashMap<ChannelId, RealmId>,
    realm_data: HashMap<RealmId, Realm>,
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

        let presence_tx = PresenceRefreshActor::spawn(client.http());

        let me = Self {
            bridge,
            client,
            portal_tasks: JoinSet::new(),
            realm_tasks: JoinSet::new(),
            presence_tx,
            portal_handles: HashMap::new(),
            portal_lookup: HashMap::new(),
            portal_data: HashMap::new(),
            realm_handles: HashMap::new(),
            realm_lookup: HashMap::new(),
            realm_data: HashMap::new(),
        };
        me.start(ready_tx).await;

        Ok(())
    }

    fn spawn_portal_task(&mut self, portal_id: PortalId, channel_id: ChannelId) {
        let portal = self.portal_data.get(&portal_id).unwrap().clone();
        let handle = self.portal_handles.get(&portal_id).unwrap().clone();
        self.portal_tasks.spawn(LampreyPortal::spawn(
            portal_id,
            portal,
            handle,
            self.client.http(),
            channel_id,
        ));
    }

    fn spawn_realm_task(&mut self, realm_id: RealmId) {
        let realm = self.realm_data.get(&realm_id).unwrap().clone();
        let handle = self.realm_handles.get(&realm_id).unwrap().clone();
        self.realm_tasks.spawn(LampreyRealm::spawn(
            realm_id,
            realm,
            handle,
            self.client.http(),
        ));
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
                            // try to restart portal task on failure
                            if let Some(&channel_id) = self
                                .portal_lookup
                                .iter()
                                .find(|(_, v)| v == &&portal_id)
                                .map(|(k, _)| k)
                            {
                                // TODO: exponential backoff? (same in discord)
                                self.spawn_portal_task(portal_id, channel_id);
                            }
                        }
                        Ok((portal_id, Ok(()))) => {
                            debug!(%portal_id, "portal task exited cleanly");
                            self.portal_handles.remove(&portal_id);
                            self.portal_data.remove(&portal_id);
                            self.portal_lookup.retain(|_, v| *v != portal_id);
                        }
                        Err(e) => {
                            warn!("portal task join error: {e:?}");
                        }
                    }
                },
                Some(result) = self.realm_tasks.join_next() => {
                    match result {
                        Ok((realm_id, Err(e))) => {
                            warn!(%realm_id, "realm task failed: {e:?}");
                            self.spawn_realm_task(realm_id);
                        }
                        Ok((realm_id, Ok(()))) => {
                            debug!(%realm_id, "realm task exited cleanly");
                            self.realm_handles.remove(&realm_id);
                            self.realm_data.remove(&realm_id);
                            self.realm_lookup.retain(|_, v| *v != realm_id);
                        }
                        Err(e) => {
                            warn!("realm task join error: {e:?}");
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
                // MessageSync::RoomUpdate { room } => todo!(),
                MessageSync::ChannelCreate { channel } => {
                    let channel_data = ChannelData::Lamprey {
                        channel: channel.clone(),
                    };

                    let Some(room_id) = channel.room_id else {
                        return Ok(());
                    };

                    let Some(realm_id) = self.realm_data.iter().find_map(|(id, realm)| {
                        if let Some(r_lamprey) = &realm.lamprey {
                            if r_lamprey.room_id == room_id && realm.continuous {
                                return Some(*id);
                            }
                        }
                        None
                    }) else {
                        return Ok(());
                    };

                    let handle = self.realm_handles.get(&realm_id).unwrap();
                    let _ = handle
                        .events
                        .send(Arc::new(RealmEvent::ChannelCreate(channel_data)));
                }
                MessageSync::ChannelUpdate { channel } => {
                    if self.portal_lookup.contains_key(&channel.id) {
                        let channel_data = ChannelData::Lamprey {
                            channel: channel.clone(),
                        };
                        self.route_portal_event(
                            &channel.id,
                            PortalEvent::ChannelUpdate(channel_data),
                        );
                    }

                    // TODO: channel delete on channel.removed_at?
                    // self.route_portal_event(channel_id, PortalEvent::ChannelDelete);
                }
                // MessageSync::UserUpdate { user } => todo!(), // ignore updates for your own puppets
                // MessageSync::RoomMemberCreate { member, user } => todo!(),
                MessageSync::RoomMemberUpdate { member, user } => {
                    let Some(realm_id) = self.realm_data.iter().find_map(|(id, realm)| {
                        if let Some(r_lamprey) = &realm.lamprey {
                            if r_lamprey.room_id == member.room_id {
                                return Some(*id);
                            }
                        }
                        None
                    }) else {
                        return Ok(());
                    };

                    let nickname = member.override_name.clone();
                    let realm_member = bridge_old::RealmMember {
                        realm_id,
                        user_lamprey_id: member.user_id,
                        nickname,
                    };

                    let _ = self
                        .bridge
                        .db
                        .realm_member_upsert(realm_member.clone())
                        .await;

                    if let Some(handle) = self.realm_handles.get(&realm_id) {
                        let _ = handle
                            .events
                            .send(Arc::new(RealmEvent::MemberUpdate(realm_member)));
                    }
                }
                // MessageSync::RoomMemberDelete { room_id, user_id } => todo!(),

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
                        if user.source_platform != Platform::Lamprey {
                            return Ok(());
                        }

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
                        MessageType::DefaultMarkdown(m) | MessageType::ThreadInitial(m) => {
                            match m.content.as_deref() {
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

                                    // FIXME: make it clear which request you're `!accept`ing or `!reject`ing (don't accept/reject *everything* at once)

                                    let _ = self
                                        .bridge
                                        .commands
                                        .send(BridgeCommand::RealmLinkResponse {
                                            lamprey_room_id: room_id,
                                            accepted,
                                        })
                                        .await;

                                    let _ = self
                                        .bridge
                                        .commands
                                        .send(BridgeCommand::PortalLinkResponse {
                                            lamprey_room_id: room_id,
                                            lamprey_channel_id: message.channel_id,
                                            accepted,
                                            lamprey_last_id: channel
                                                .last_message_id
                                                .unwrap_or_default(),
                                        })
                                        .await;

                                    // FIXME: send msg on bridge event portal created instead of here
                                    let msg = if accepted {
                                        "portal/realm request accepted!"
                                    } else {
                                        "portal/realm request denied"
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
                            }
                        }
                        _ => {}
                    }

                    // PERF: cache this
                    let user = self
                        .client
                        .http()
                        .user_get(UserIdReq::UserId(message.author_id))
                        .await?;

                    // PERF: cache this too
                    let room_member = if let Some(room_id) = message.room_id {
                        let member = self
                            .client
                            .http()
                            .room_member_get(room_id, message.author_id.into())
                            .await?;
                        Some(Box::new(member))
                    } else {
                        None
                    };

                    self.route_portal_event(
                        &message.channel_id,
                        PortalEvent::MessageCreate(bridge_old::MessageData::Lamprey {
                            message: Box::new(message.clone()),
                            user: Box::new(user.inner),
                            room_member,
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

                    // NOTE: do i want to pass room_member here?

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
                MessageSync::MessageDelete {
                    channel_id,
                    message_id,
                    room_id: _,
                } => {
                    if let Some(portal_id) = self.portal_lookup.get(&channel_id) {
                        if let Ok(Some(msg)) = self
                            .bridge
                            .db
                            .message_get_by_lamprey_id(*portal_id, *message_id)
                            .await
                        {
                            self.route_portal_event(channel_id, PortalEvent::MessageDelete(msg.id));
                        }
                    }
                }
                MessageSync::ReactionCreate {
                    room_id: _,
                    channel_id,
                    message_id,
                    user_id,
                    key,
                } => {
                    if let Some(portal_id) = self.portal_lookup.get(&channel_id) {
                        let Some(msg) = self
                            .bridge
                            .db
                            .message_get_by_lamprey_id(*portal_id, *message_id)
                            .await?
                        else {
                            return Ok(());
                        };

                        let Some(user) = self
                            .bridge
                            .db
                            .puppet_get_by_lamprey_id(user_id.to_string())
                            .await?
                        else {
                            return Ok(());
                        };

                        self.route_portal_event(
                            channel_id,
                            PortalEvent::ReactionCreate(
                                msg.id,
                                crate::bridge_old::ReactionKey::Lamprey(key.clone()),
                                user,
                            ),
                        );
                    }
                }
                MessageSync::ReactionDelete {
                    room_id: _,
                    channel_id,
                    message_id,
                    user_id,
                    key,
                } => {
                    if let Some(portal_id) = self.portal_lookup.get(&channel_id) {
                        let Some(msg) = self
                            .bridge
                            .db
                            .message_get_by_lamprey_id(*portal_id, *message_id)
                            .await?
                        else {
                            return Ok(());
                        };

                        let Some(user) = self
                            .bridge
                            .db
                            .puppet_get_by_lamprey_id(user_id.to_string())
                            .await?
                        else {
                            return Ok(());
                        };

                        self.route_portal_event(
                            channel_id,
                            PortalEvent::ReactionDelete(
                                msg.id,
                                crate::bridge_old::ReactionKey::Lamprey(key.clone()),
                                user,
                            ),
                        );
                    }
                }
                MessageSync::ReactionDeleteKey {
                    room_id: _,
                    channel_id,
                    message_id,
                    key,
                } => {
                    if let Some(portal_id) = self.portal_lookup.get(&channel_id) {
                        let Some(msg) = self
                            .bridge
                            .db
                            .message_get_by_lamprey_id(*portal_id, *message_id)
                            .await?
                        else {
                            return Ok(());
                        };

                        self.route_portal_event(
                            channel_id,
                            PortalEvent::ReactionDeleteKey(
                                msg.id,
                                crate::bridge_old::ReactionKey::Lamprey(key.clone()),
                            ),
                        );
                    }
                }
                MessageSync::ReactionDeleteAll {
                    room_id: _,
                    channel_id,
                    message_id,
                } => {
                    if let Some(portal_id) = self.portal_lookup.get(&channel_id) {
                        let Some(msg) = self
                            .bridge
                            .db
                            .message_get_by_lamprey_id(*portal_id, *message_id)
                            .await?
                        else {
                            return Ok(());
                        };

                        self.route_portal_event(channel_id, PortalEvent::ReactionDeleteAll(msg.id));
                    }
                }
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
            BridgeEvent::RealmInit(realm, handle) => {
                self.realm_handles.insert(realm.id, handle.clone());
                self.realm_data.insert(realm.id, realm.clone());
                self.spawn_realm_task(realm.id);
            }
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
            BridgeEvent::PortalLinkRequest { lamprey_channel_id } => {
                let channel_id = *lamprey_channel_id;
                let http = self.client.http();
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
            BridgeEvent::RealmLinkRequest { lamprey_room_id } => {
                let http = self.client.http();
                let lamprey_room_id = *lamprey_room_id;
                tokio::spawn(async move {
                    let channel_id = async {
                        // prefer welcome_channel_id
                        if let Ok(room) = http.room_get(lamprey_room_id).await {
                            if let Some(id) = room.welcome_channel_id {
                                return Some(id);
                            }
                        }

                        // fallback to first channel
                        // FIXME: don't try to send messages to channels we don't have permissions in
                        if let Ok(channels) = http
                            .channel_list(
                                lamprey_room_id,
                                &PaginationQuery {
                                    limit: Some(10),
                                    ..Default::default()
                                },
                            )
                            .await
                        {
                            if let Some(channel) =
                                channels.items.iter().find(|c| c.ty == ChannelType::Text)
                            {
                                return Some(channel.id);
                            }
                        }

                        None
                    }
                    .await;

                    if let Some(channel_id) = channel_id {
                        // TODO: handle err
                        let _ = http.message_create(
                            channel_id,
                            &MessageCreate {
                                content: Some(
                                    "A discord guild is requesting to link with this room. Reply with !accept or !reject"
                                        .to_string(),
                                ),
                                ..Default::default()
                            },
                        ).await;
                    } else {
                        // TODO: handle this better somehow? instead of silently failing?
                        // maybe send warning back to discord
                        warn!("no valid lamprey channel to send link request to!");
                    }
                });
            }
            BridgeEvent::PresenceUpdate(presence) => {
                let bridge = self.bridge.clone();
                let presence = presence.clone();
                let presence_tx = self.presence_tx.clone();

                // TODO: warn!() on err
                tokio::spawn(async move {
                    let discord_id = presence.user.id.to_string();
                    let Some(puppet) = bridge
                        .db
                        .puppet_get_by_discord_id(discord_id.clone())
                        .await?
                    else {
                        trace!(user_id=%presence.user.id, "no puppet found for discord user");
                        return Ok(());
                    };

                    let user_id = puppet.lamprey_id;
                    let status = match presence.status {
                        discord::OnlineStatus::Online => Status::Online,
                        discord::OnlineStatus::Idle => Status::Away,
                        discord::OnlineStatus::DoNotDisturb => Status::Busy,
                        discord::OnlineStatus::Invisible | discord::OnlineStatus::Offline => {
                            Status::Offline
                        }
                        _ => Status::Online,
                    };

                    let activities = presence
                        .activities
                        .iter()
                        .filter(|a| a.kind == discord::ActivityType::Custom)
                        .filter_map(|a| a.state.clone())
                        .map(|text| Activity::Custom {
                            text,
                            clear_at: None,
                        })
                        .collect();

                    let ly_presence = Presence { status, activities };

                    let _ = presence_tx.send(PresenceEvent::Update(user_id, ly_presence));

                    Result::Ok(())
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
        self.portal_data.insert(portal.id, portal.clone());
        self.spawn_portal_task(portal.id, channel_id);
    }
}
