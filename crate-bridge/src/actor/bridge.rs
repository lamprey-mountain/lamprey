use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc};
use tracing::debug;
use url::Url;

use crate::bridge_old::{
    Portal, PortalCreate, PortalDiscord, PortalEvent, PortalHandle, PortalId, PortalLamprey,
};
use crate::database::Database;
use crate::prelude::*;
use crate::types::{PendingLink, PendingLinkId};

// TODO: make these commands platform-agnostic
#[derive(Debug, Clone)]
pub enum BridgeCommand {
    /// Discord requests a link
    LinkRequest {
        discord_guild_id: discord::GuildId,
        discord_channel_id: discord::ChannelId,
        lamprey_channel_id: lamprey::ChannelId,
        webhook_url: Url,
    },

    /// Lamprey received !accept or !reject
    LinkResponse {
        lamprey_channel_id: lamprey::ChannelId,
        accepted: bool,
    },

    /// Discord requests unlink
    PortalUnlink {
        discord_channel_id: discord::ChannelId,
    },
}

/// an event that's broadcast to a bridge
// TODO: clean up/remove unused variants (after making sure they actually aren't needed)
// TODO: dont use bridge_old types
#[derive(Debug, Clone)]
pub enum BridgeEvent {
    /// load a realm from the database
    RealmInit(bridge_old::Realm),

    /// load a portal from the database
    PortalInit(Portal, PortalHandle),

    /// a portal has been newly created
    PortalCreated(Portal),

    /// an event for a portal
    PortalEvent(PortalId, PortalEvent),

    /// a portal should be deleted
    ///
    /// the sender of this event should delete stuff from the database
    PortalDeleted(PortalId),

    /// a portal has been requested to be created
    PortalRequest(PortalCreate),

    // TODO: make these events platform agnostic
    /// Lamprey should send "Reply !accept or !reject"
    LinkRequest {
        lamprey_channel_id: lamprey::ChannelId,
    },

    /// Discord should notify user of success/failure
    LinkResponse {
        discord_channel_id: discord::ChannelId,
        accepted: bool,
    },
    // TODO: more events
    // PortalUpdate,
    // UserUpdate,
    // MemberCreate,
    // MemberUpdate,
    // MemberDelete,
    // PresenceUpdate,

    // UserUpdate {
    //     source_user_id: UserId,
    //     avatar_url: Option<Url>,
    //     banner_url: Option<Url>,
    // },
}

impl BridgeEvent {
    /// Returns `true` if the bridge event is [`RealmInit`].
    ///
    /// [`RealmInit`]: BridgeEvent::RealmInit
    #[must_use]
    pub fn is_realm_init(&self) -> bool {
        matches!(self, Self::RealmInit(..))
    }
}

pub struct BridgeActor {
    rx: mpsc::Receiver<BridgeCommand>,
    events: broadcast::Sender<Arc<BridgeEvent>>,
    db: Arc<dyn Database>,
    pending_links: HashMap<lamprey::ChannelId, PendingLink>,
}

impl BridgeActor {
    pub fn new(
        rx: mpsc::Receiver<BridgeCommand>,
        events: broadcast::Sender<Arc<BridgeEvent>>,
        db: Arc<dyn Database>,
    ) -> Self {
        Self {
            rx,
            events,
            db,
            pending_links: HashMap::new(),
        }
    }

    pub async fn run(mut self) {
        while let Some(cmd) = self.rx.recv().await {
            self.handle_command(cmd).await;
        }
    }

    async fn handle_command(&mut self, cmd: BridgeCommand) {
        debug!("bridge got command {cmd:?}");
        match cmd {
            BridgeCommand::LinkRequest {
                discord_guild_id,
                discord_channel_id,
                lamprey_channel_id,
                webhook_url,
            } => {
                // 1. Check DB for existing link (both sides)
                if let Ok(Some(_)) = self
                    .db
                    .portal_get_by_discord_channel(discord_channel_id.to_string())
                    .await
                {
                    let _ = self.events.send(Arc::new(BridgeEvent::LinkResponse {
                        discord_channel_id,
                        accepted: false,
                    }));
                    return;
                }

                if let Ok(Some(_)) = self
                    .db
                    .portal_get_by_lamprey_channel(lamprey_channel_id.to_string())
                    .await
                {
                    let _ = self.events.send(Arc::new(BridgeEvent::LinkResponse {
                        discord_channel_id,
                        accepted: false,
                    }));
                    return;
                }

                // 2. Check self.pending_links
                if self.pending_links.contains_key(&lamprey_channel_id) {
                    let _ = self.events.send(Arc::new(BridgeEvent::LinkResponse {
                        discord_channel_id,
                        accepted: false,
                    }));
                    return;
                }

                // 3. Store in pending_links
                let pending_id = PendingLinkId::new();
                self.pending_links.insert(
                    lamprey_channel_id,
                    PendingLink {
                        id: pending_id,
                        discord_guild_id,
                        discord_channel_id,
                        lamprey_channel_id,
                        webhook_url,
                        confirmation_message_id: None,
                    },
                );

                // 4. Send BridgeEvent::SendConfirmationRequest to Lamprey
                let _ = self
                    .events
                    .send(Arc::new(BridgeEvent::LinkRequest { lamprey_channel_id }));
            }

            BridgeCommand::LinkResponse {
                lamprey_channel_id,
                accepted,
            } => {
                // 1. Remove from pending_links
                if let Some(pending) = self.pending_links.remove(&lamprey_channel_id) {
                    // 2. If accepted, create Portal in DB, broadcast PortalCreated
                    if accepted {
                        let portal_id = PortalId::new();
                        let portal = Portal {
                            id: portal_id,
                            realm_id: None,
                            lamprey: Some(PortalLamprey {
                                channel_id: pending.lamprey_channel_id,
                                // FIXME: populate these fields
                                room_id: lamprey::RoomId::new(),
                                last_id: lamprey::MessageId::new(),
                            }),
                            discord: Some(PortalDiscord {
                                guild_id: pending.discord_guild_id,
                                parent_id: None,
                                channel_id: pending.discord_channel_id,
                                webhook_url: pending.webhook_url,
                                // FIXME: populate this field
                                last_id: discord::MessageId::default(),
                            }),
                        };

                        if self.db.portal_create(portal.clone()).await.is_ok() {
                            let _ = self
                                .events
                                .send(Arc::new(BridgeEvent::PortalCreated(portal)));
                        }
                    }

                    // TODO: more granular link response status
                    // enum LinkResponseStatus {
                    //     Accepted,
                    //     Rejected,
                    //     Errored, // with eg. sqlx db error
                    // }

                    // 3. Broadcast LinkResult to Discord
                    let _ = self.events.send(Arc::new(BridgeEvent::LinkResponse {
                        discord_channel_id: pending.discord_channel_id,
                        accepted,
                    }));
                }
            }

            BridgeCommand::PortalUnlink { discord_channel_id } => {
                if let Ok(Some(portal)) = self
                    .db
                    .portal_get_by_discord_channel(discord_channel_id.to_string())
                    .await
                {
                    let _ = self.db.portal_delete(portal.id).await;
                    let _ = self
                        .events
                        .send(Arc::new(BridgeEvent::PortalDeleted(portal.id)));
                }
            }
        }
    }
}
