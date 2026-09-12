use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{error, warn};

use serenity::all::{ChannelType, CreateChannel, CreateWebhook};

use crate::bridge_old::{
    BridgeEvent, Portal, PortalDiscord, PortalId, PortalLamprey, Realm, RealmEvent, RealmHandle,
    RealmId,
};
use crate::prelude::*;
use crate::types::ChannelData;
use sdk::http::Http;

pub struct DiscordRealm {
    realm_id: RealmId,
    realm: Realm,
    handle: RealmHandle,
    http: Arc<serenity::all::Http>,
}

impl DiscordRealm {
    pub async fn spawn(
        realm_id: RealmId,
        realm: Realm,
        handle: RealmHandle,
        http: Arc<serenity::all::Http>,
        _cache: Arc<serenity::all::Cache>,
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

                            let create_channel =
                                CreateChannel::new(channel.name.clone()).kind(match channel.ty {
                                    lamprey::ChannelType::Text => ChannelType::Text,
                                    _ => continue,
                                });

                            // TODO: run below code in a tokio task instead of blocking loop

                            // TODO: add audit log reason
                            let discord_channel =
                                match http.create_channel(guild_id, &create_channel, None).await {
                                    Ok(ch) => ch,
                                    Err(e) => {
                                        error!(?e, "failed to create discord channel");
                                        continue;
                                    }
                                };

                            // TODO: deduplicate this code with `/link`
                            let webhook = match discord_channel
                                .create_webhook(&http, CreateWebhook::new("bridge"))
                                .await
                            {
                                Ok(wh) => wh,
                                Err(e) => {
                                    error!(?e, "failed to create webhook");
                                    continue;
                                }
                            };

                            let webhook_url = webhook
                                .url()
                                .expect("webhook url")
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
                                let _ = bridge
                                    .events
                                    .send(Arc::new(BridgeEvent::PortalCreated(portal)));
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
                            let discord_id: serenity::all::UserId = puppet.discord_id;
                            let nick = member.nickname.as_deref().unwrap_or("");
                            let _ = guild_id
                                .edit_member(
                                    &self.http,
                                    discord_id,
                                    serenity::all::EditMember::new().nickname(nick),
                                )
                                .await;
                        }
                    }
                }
            }
        }

        Ok(())
    }
}
