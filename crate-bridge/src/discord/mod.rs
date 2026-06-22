use std::collections::HashMap;
use std::sync::Arc;

use serenity::all::{ExecuteWebhook, GatewayIntents};
use tokio::sync::{broadcast, mpsc, oneshot};
use tokio::task::JoinSet;
use tracing::{debug, info, warn};

use crate::bridge::{MessageData, PlatformHandle, Portal, PortalHandle, PortalId};
use crate::prelude::*;
use crate::{
    bridge::{BridgeEvent, BridgeHandle, PortalEvent},
    config::DiscordConfig,
};

mod events;
mod interactions;

// re export discord (serenity) types
pub use serenity::all::{
    Attachment, AttachmentId, ChannelId, CreateAllowedMentions, CreateEmbed, Embed, GuildId,
    Message, MessageId, User, UserId,
};

pub fn spawn(bridge: BridgeHandle, config: DiscordConfig) -> PlatformHandle {
    let (tx, rx) = oneshot::channel();
    let task = tokio::spawn(Discord::connect(bridge, config, tx));
    PlatformHandle {
        name: "discord",
        ready: rx,
        task,
    }
}

struct Discord {
    bridge: BridgeHandle,
    rx: mpsc::Receiver<events::DiscordEvent>,
    portal_tasks: JoinSet<(PortalId, Result<()>)>,
    portal_handles: HashMap<PortalId, PortalHandle>,
    portal_lookup: HashMap<ChannelId, PortalId>,
    http: Arc<serenity::all::Http>,
    cache: Arc<serenity::all::Cache>,
}

impl Discord {
    async fn connect(
        bridge: BridgeHandle,
        config: DiscordConfig,
        ready_tx: oneshot::Sender<()>,
    ) -> Result<()> {
        let (tx, rx) = mpsc::channel(1024);
        let handler = events::Handler { tx };
        let client = serenity::Client::builder(
            &config.token.load().expect("failed to load token"),
            GatewayIntents::all(),
        )
        .event_handler(handler)
        .await
        .map_err(|e| anyhow::anyhow!("Error creating client: {:?}", e))?;

        let http = client.http.clone();
        let cache = client.cache.clone();

        let me = Self {
            bridge,
            rx,
            portal_tasks: JoinSet::new(),
            portal_handles: HashMap::new(),
            portal_lookup: HashMap::new(),
            http,
            cache,
        };
        me.start(client, ready_tx).await?;

        Ok(())
    }

    async fn start(
        mut self,
        mut client: serenity::Client,
        ready_tx: oneshot::Sender<()>,
    ) -> Result<()> {
        tokio::spawn(async move {
            if let Err(why) = client.start().await {
                eprintln!("Client error: {:?}", why);
            }
        });

        let mut bridge_events = self.bridge.events.subscribe();
        ready_tx.send(()).unwrap();

        loop {
            tokio::select! {
                Ok(event) = bridge_events.recv() => {
                    self.handle_bridge_event(&event);
                }
                Some(event) = self.rx.recv() => {
                    match event {
                        events::DiscordEvent::MessageCreate(message) => {
                            self.route_portal_event(
                                message.channel_id,
                                PortalEvent::MessageCreate(MessageData::Discord(message)),
                            );
                        }
                    }
                }
            }
        }
    }

    fn route_portal_event(&self, channel_id: ChannelId, event: PortalEvent) {
        info!("forwarding event to portal for channel: {:?}", channel_id);
        if let Some(portal_id) = self.portal_lookup.get(&channel_id) {
            if let Some(handle) = self.portal_handles.get(portal_id) {
                let _ = handle.events.send(Arc::new(event));
            }
        }
    }

    fn handle_bridge_event(&mut self, event: &BridgeEvent) {
        match event {
            BridgeEvent::PortalInit(id, portal, handle) => {
                if let Some(discord) = &portal.discord {
                    self.portal_lookup.insert(discord.channel_id, *id);
                }
                self.portal_handles.insert(*id, handle.clone());
                self.portal_tasks.spawn(spawn_portal(
                    *id,
                    portal.clone(),
                    handle.clone(),
                    self.http.clone(),
                    self.cache.clone(),
                ));
            }
            BridgeEvent::PortalEvent(id, event) => {
                if let Some(handle) = self.portal_handles.get(id) {
                    let _ = handle.events.send(Arc::new(event.clone()));
                }
            }
            _ => {} // TODO: handle more events
        }
    }
}

async fn spawn_portal(
    id: PortalId,
    portal: Portal,
    handle: PortalHandle,
    http: Arc<serenity::all::Http>,
    cache: Arc<serenity::all::Cache>,
) -> (PortalId, Result<()>) {
    (
        id,
        spawn_portal_inner(id, portal, handle, http, cache).await,
    )
}

async fn spawn_portal_inner(
    portal_id: PortalId,
    portal: Portal,
    handle: PortalHandle,
    http: Arc<serenity::all::Http>,
    cache: Arc<serenity::all::Cache>,
) -> Result<()> {
    let mut events = handle.events.subscribe();

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
                let msg = match data {
                    MessageData::Lamprey(m) => m,
                    MessageData::Discord(_) => continue,
                };

                let discord_cfg = portal.discord.as_ref().unwrap();
                let webhook_url = &discord_cfg.webhook_url;
                let webhook = serenity::all::Webhook::from_url(&http, webhook_url.as_str()).await?;

                let msg_inner = match &msg.latest_version.message_type {
                    common::v1::types::MessageType::DefaultMarkdown(m) => m,
                    _ => {
                        debug!("unsupported lamprey message type");
                        // TODO: format and send anyways?
                        continue;
                    }
                };

                let content = msg_inner.content.to_owned().unwrap_or_else(|| {
                    if msg_inner.attachments.is_empty()
                        && msg_inner.embeds.is_empty()
                        && msg_inner.components.is_empty()
                    {
                        "(no content?)".to_owned()
                    } else {
                        "".to_owned()
                    }
                });

                // TODO: handle reply_id
                // TODO: handle embeds (download, reupload)
                // TODO: handle attachments
                // TODO: handle mentions

                let builder = ExecuteWebhook::new().content(content);
                // // TODO: set profile
                // .username(username)
                // .avatar_url(avatar_url)
                // // TODO: handle threads
                // .in_thread(thread_id)
                // // TODO: handle other stuff
                // .files(files)
                // .components(components)
                // .embeds(embeds)
                // .allowed_mentions(allowed_mentions);

                webhook.execute(&http, false, builder).await?;
            }
            PortalEvent::MessageUpdate(_) => todo!(),
            PortalEvent::MessageDelete(_) => todo!(),
            PortalEvent::ReactionCreate(_, _, _) => todo!(),
            PortalEvent::ReactionDelete(_, _, _) => todo!(),
            PortalEvent::ReactionDeleteEmoji(_, _) => todo!(),
            PortalEvent::ReactionDeleteAll(_, _) => todo!(),
        }
    }

    Ok(())
}
