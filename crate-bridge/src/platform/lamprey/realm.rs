use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{error, warn};

use crate::bridge_old::{
    BridgeEvent, Portal, PortalDiscord, PortalId, PortalLamprey, Realm, RealmEvent, RealmHandle,
    RealmId,
};
use crate::prelude::*;
use crate::types::ChannelData;
use common::v1::types::{ChannelCreate, ChannelType, RoomMemberPut};
use sdk::http::Http;

pub struct LampreyRealm {
    realm_id: RealmId,
    realm: Realm,
    handle: RealmHandle,
    http: Http,
}

impl LampreyRealm {
    pub async fn spawn(
        realm_id: RealmId,
        realm: Realm,
        handle: RealmHandle,
        http: Http,
    ) -> (RealmId, Result<()>) {
        let me = Self {
            realm_id,
            realm,
            handle,
            http,
        };
        (realm_id, me.run().await)
    }

    async fn run(&self) -> Result<()> {
        let room_id = self.realm.lamprey.as_ref().unwrap().room_id;
        let mut events = self.handle.events.subscribe();
        loop {
            let event = match events.recv().await {
                Ok(e) => e,
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    warn!(realm_id=%self.realm_id, n, "realm event receiver lagged, skipping");
                    continue;
                }
                Err(broadcast::error::RecvError::Closed) => break,
            };
            match &*event {
                RealmEvent::ChannelCreate(chan) => {
                    if !self.realm.continuous {
                        continue;
                    }

                    let (create, chan, webhook) = match chan {
                        ChannelData::Lamprey { .. } => continue,
                        ChannelData::Discord {
                            channel: chan,
                            webhook,
                        } => {
                            let create = ChannelCreate {
                                name: chan.name.clone(),
                                description: chan.topic.clone(),
                                ty: match chan.kind {
                                    discord::ChannelType::Text => ChannelType::Text,
                                    discord::ChannelType::News => ChannelType::Text,
                                    discord::ChannelType::Category => ChannelType::Category,

                                    // TODO: voice bridging
                                    // TODO: thread/forum bridging
                                    // discord::ChannelType::Voice => ChannelType::Voice,
                                    // discord::ChannelType::NewsThread => todo!(),
                                    // discord::ChannelType::PublicThread => todo!(),
                                    // discord::ChannelType::PrivateThread => todo!(),
                                    // discord::ChannelType::Stage => todo!(),
                                    // discord::ChannelType::Forum => todo!(),

                                    // not supported
                                    _ => continue,
                                },
                                // parent_id: chan.parent_id, // TODO: map discord channel id to lamprey channel id
                                nsfw: chan.nsfw,
                                ..Default::default()
                            };
                            (create, chan, webhook)
                        }
                    };

                    let channel = match self.http.channel_create_room(room_id, &create).await {
                        Ok(chan) => chan,
                        Err(err) => {
                            error!("couldn't create channel: {err}");
                            continue;
                        }
                    };

                    let Some((webhook_id, webhook_url)) = webhook else {
                        // this doesn't have an associated webhook (eg. this is a category channel), so do create a channel but don't create a portal
                        // TODO: store channel id mappings
                        continue;
                    };

                    let portal_id = PortalId::new();
                    let portal = Portal {
                        id: portal_id,
                        realm_id: Some(self.realm_id),
                        lamprey: Some(PortalLamprey {
                            channel_id: channel.id,
                            room_id,
                            last_id: channel.last_message_id.unwrap_or_default(), // NOTE: this will always be default
                        }),
                        discord: Some(PortalDiscord {
                            guild_id: chan.guild_id,
                            parent_id: chan.parent_id,
                            channel_id: chan.id,
                            webhook_url: webhook_url.clone(),
                            webhook_id: Some(*webhook_id),
                            last_id: chan.last_message_id.unwrap_or_default(),
                        }),
                    };

                    if self
                        .handle
                        .bridge
                        .db
                        .portal_create(portal.clone())
                        .await
                        .is_ok()
                    {
                        let _ = self
                            .handle
                            .bridge
                            .events
                            .send(Arc::new(BridgeEvent::PortalCreated(portal)));
                    }
                }
                RealmEvent::MemberUpdate(member) => {
                    let Ok(Some(puppet)) = self
                        .handle
                        .bridge
                        .db
                        .puppet_get_by_lamprey_id(member.user_lamprey_id.to_string())
                        .await
                    else {
                        continue;
                    };
                    if puppet.source_platform == crate::types::Platform::Discord {
                        let room_id = self.realm.lamprey.as_ref().unwrap().room_id;
                        let lamprey_id = member.user_lamprey_id;

                        let _ = self
                            .http
                            .room_member_add(
                                room_id,
                                lamprey_id.into(),
                                &RoomMemberPut {
                                    override_name: member.nickname.clone(),
                                    ..Default::default()
                                },
                            )
                            .await;
                    }
                }
            }
        }
        Ok(())
    }
}
