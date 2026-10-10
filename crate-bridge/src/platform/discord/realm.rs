use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{error, warn};

use twilight_http::Client as HttpClient;
use twilight_model::channel::ChannelType;
use twilight_model::id::Id;

use crate::bridge_old::{
    BridgeEvent, Portal, PortalDiscord, PortalId, PortalLamprey, Realm, RealmEvent, RealmHandle,
    RealmId,
};
use crate::prelude::*;
use crate::types::ChannelData;

pub struct DiscordRealm {
    realm_id: RealmId,
    realm: Realm,
    handle: RealmHandle,
    http: Arc<HttpClient>,
}

impl DiscordRealm {
    pub async fn spawn(
        realm_id: RealmId,
        realm: Realm,
        handle: RealmHandle,
        http: Arc<HttpClient>,
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

                    match chan {
                        ChannelData::Discord { .. } => continue,
                        ChannelData::Lamprey { channel } => {
                            let guild_id = self.realm.discord.as_ref().unwrap().guild_id;
                            let http = self.http.clone();
                            let bridge = self.handle.bridge.clone();

                            // Twilight channel creation
                            let channel_type = match channel.ty {
                                lamprey::ChannelType::Text => ChannelType::GuildText,
                                _ => continue,
                            };
                            
                            let discord_channel = http
                                .create_guild_channel(Id::new(guild_id.get()), &channel.name)
                                .kind(channel_type)
                                .await?
                                .model()
                                .await?;

                            // Twilight webhook creation
                            let webhook = http
                                .create_webhook(Id::new(discord_channel.id.get()), "bridge")
                                .await?
                                .model()
                                .await?;

                            let webhook_url = webhook
                                .url()
                                .parse()
                                .expect("invalid webhook url");

                            let portal_id = PortalId::new();
                            let portal = Portal {
                                id: portal_id,
                                realm_id: Some(self.realm_id),
                                lamprey: Some(PortalLamprey {
                                    channel_id: channel.id,
                                    room_id: channel.room_id.unwrap(),
                                    last_id: channel.last_message_id.unwrap_or_default(),
                                }),
                                discord: Some(PortalDiscord {
                                    guild_id,
                                    parent_id: None,
                                    channel_id: discord_channel.id,
                                    webhook_url,
                                    webhook_id: Some(webhook.id),
                                    last_id: discord_channel.last_message_id.unwrap_or_default(),
                                }),
                            };

                            if bridge.db.portal_create(portal.clone()).await.is_ok() {
                                let handle = bridge.create_portal_handle(portal.id);
                                let _ = bridge
                                    .events
                                    .send(Arc::new(BridgeEvent::PortalCreated(portal, handle)));
                            }
                        }
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
                    if puppet.source_platform == crate::types::Platform::Lamprey {
                        if let Some(discord_cfg) = self.realm.discord.as_ref() {
                            let guild_id = discord_cfg.guild_id;
                            let discord_id = puppet.discord_id; // Assuming puppet_get_by_lamprey_id returns Twilight Id?
                            
                            let nick = member.nickname.as_deref().unwrap_or("");
                            let _ = self.http
                                .update_guild_member(Id::new(guild_id.get()), Id::new(discord_id.get()))
                                .nick(Some(nick))
                                .await;
                        }
                    }
                }
            }
        }

        Ok(())
    }
}
