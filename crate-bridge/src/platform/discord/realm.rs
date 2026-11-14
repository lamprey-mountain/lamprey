use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{error, warn};

use twilight_http::Client as HttpClient;

use crate::bridge_old::{
    BridgeEvent, Portal, PortalDiscord, PortalId, PortalLamprey, Realm, RealmEvent, RealmHandle,
    RealmId,
};
use crate::platform::discord::{ChannelType, webhook_url};
use crate::prelude::*;
use crate::types::ChannelData;
use twilight_model::id::Id;

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
                            // the same channel can be replayed on reconnect
                            if self
                                .handle
                                .bridge
                                .db
                                .portal_get_by_lamprey_channel(channel.id.to_string())
                                .await?
                                .is_some()
                            {
                                continue;
                            }

                            let guild_id = self.realm.discord.as_ref().unwrap().guild_id;
                            let http = self.http.clone();
                            let bridge = self.handle.bridge.clone();

                            let name = channel
                                .name
                                .clone()
                                .or_else(|| channel.auto_name.clone())
                                .unwrap_or_default();

                            let kind = match channel.ty {
                                lamprey::ChannelType::Text => ChannelType::GuildText,
                                lamprey::ChannelType::ThreadPublic
                                | lamprey::ChannelType::ThreadPrivate => {
                                    warn!(
                                        channel_id = %channel.id,
                                        "cannot bridge lamprey thread: parent channel id translation is not implemented"
                                    );
                                    continue;
                                }
                                _ => continue,
                            };

                            // TODO: run below code in a tokio task instead of blocking loop

                            // TODO: add audit log reason
                            let discord_channel =
                                match http.create_guild_channel(guild_id, &name).kind(kind).await {
                                    Ok(r) => match r.model().await {
                                        Ok(ch) => ch,
                                        Err(e) => {
                                            error!(?e, "failed to create discord channel");
                                            continue;
                                        }
                                    },
                                    Err(e) => {
                                        error!(?e, "failed to create discord channel");
                                        continue;
                                    }
                                };

                            // TODO: deduplicate this code with `/link`
                            let webhook =
                                match http.create_webhook(discord_channel.id, "bridge").await {
                                    Ok(r) => match r.model().await {
                                        Ok(wh) => wh,
                                        Err(e) => {
                                            error!(?e, "failed to create webhook");
                                            continue;
                                        }
                                    },
                                    Err(e) => {
                                        error!(?e, "failed to create webhook");
                                        continue;
                                    }
                                };

                            let Some(webhook_token) = webhook.token.as_deref() else {
                                error!("webhook response is missing its token");
                                continue;
                            };

                            let webhook_url = webhook_url(webhook.id, webhook_token);

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
                                    last_id: discord_channel
                                        .last_message_id
                                        .map(|id| id.cast())
                                        .unwrap_or(discord::MIN_MESSAGE_ID),
                                }),
                            };

                            match bridge.db.portal_create(portal.clone()).await {
                                Ok(()) => {
                                    let handle = bridge.create_portal_handle(portal.id);
                                    let _ = bridge
                                        .events
                                        .send(Arc::new(BridgeEvent::PortalCreated(portal, handle)));
                                }
                                Err(err) => error!(
                                    portal_id = %portal.id,
                                    "failed to store portal for newly created channel: {err}"
                                ),
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
                            let discord_id = puppet.discord_id;
                            let nick = member.nickname.as_deref().unwrap_or("");
                            let _ = self
                                .http
                                .update_guild_member(guild_id, discord_id)
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
