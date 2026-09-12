use std::collections::HashMap;

use common::{
    v1::types::{
        MessageAttachmentCreate, MessageAttachmentCreateType, MessageAttachmentType, MessageCreate,
        MessageType, ParseMentions, RoomMemberPut, misc::UserIdReq, reaction::ReactionKeyParam,
    },
    v2::types::ChannelId,
};
use sdk::http::{Http, MessageCreateOptions};
use time::OffsetDateTime;
use tokio::sync::broadcast;
use tracing::{Instrument, debug, error, warn};

use crate::bridge_old as bridge;
use crate::{
    bridge_old::{Portal, PortalEvent, PortalHandle},
    platform::lamprey::client::{ImportUrl, LampreyClient},
    prelude::*,
    types::Platform,
    util::mentions::MessageTransformer,
};

pub struct LampreyPortal {
    portal_id: PortalId,
    portal: Portal,
    handle: PortalHandle,
    http: Http,
    channel_id: ChannelId,
}

impl LampreyPortal {
    pub async fn spawn(
        portal_id: PortalId,
        portal: Portal,
        handle: PortalHandle,
        http: Http,
        channel_id: ChannelId,
    ) -> (PortalId, Result<()>) {
        let me = Self {
            portal_id,
            portal,
            handle,
            http,
            channel_id,
        };
        (portal_id, me.run().await)
    }

    async fn run(self) -> Result<()> {
        let mut events = self.handle.events.subscribe();

        // TODO(?): maybe store this in LampreyPortal (instead of http?). or maybe inline LampreyClient logic into LampreyPortal?
        let ly = LampreyClient::new(
            self.http.clone(),
            self.handle.bridge.clone(),
            self.channel_id,
        );

        self.backfill(&ly)
            .instrument(tracing::debug_span!(
                "lamprey backfill",
                portal_id = %self.portal_id,
                channel_id = %self.channel_id,
            ))
            .await?;

        loop {
            let event = match events.recv().await {
                Ok(e) => e,
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    warn!(portal_id=%self.portal_id, n, "portal event receiver lagged, skipping");
                    continue;
                }
                Err(broadcast::error::RecvError::Closed) => break,
            };

            debug!("lamprey portal recv event: {event:?}");

            if let Err(err) = self.handle_event(&ly, &event).await {
                error!("error while handling event {event:?}: {err}");
            }
        }

        Ok(())
    }

    async fn backfill(&self, ly: &LampreyClient) -> Result<()> {
        // TODO: backfill should be a task that doesn't block the portal
        // HOWEVER, the portal should bridge messages until backfilling is done
        let mut last_id = self.portal.lamprey.as_ref().expect("handle None").last_id;

        debug!(last_id=%last_id, "start backfill");

        loop {
            let messages = match ly.fetch_after(last_id).await {
                Ok(m) => m,
                Err(e) => {
                    warn!(%last_id, "failed to fetch_after messages: {e:?}");
                    return Ok(());
                }
            };

            // break if messages is empty
            if messages.is_empty() {
                return Ok(());
            }

            debug!(count=%messages.len(), "backfill messages");

            for message in messages {
                let message_id = message.id;
                let author_id = message.author_id;
                let event = PortalEvent::MessageCreate(bridge_old::MessageData::Lamprey {
                    message: Box::new(message),
                    user: Box::new(ly.http.user_get(UserIdReq::UserId(author_id)).await?.inner),
                    room_member: None,
                    info: Box::new(bridge_old::LampreyInfo {
                        cdn_url: ly.http.cdn_url().clone(),
                    }),
                });

                if self.handle.events.send(Arc::new(event)).is_err() {
                    error!(%message_id, "portal event queue is full, dropping message");
                }

                last_id = message_id;
            }
        }
    }

    async fn handle_event(&self, ly: &LampreyClient, event: &PortalEvent) -> Result<()> {
        match event {
            PortalEvent::Typing(user) => {
                ly.http
                    .for_puppet(user.lamprey_id)?
                    .channel_typing(self.channel_id)
                    .await?;
            }
            PortalEvent::MessageCreate(data) => {
                // PERF: parse after checking MessageData::Lamprey
                let transformer = MessageTransformer::parse(data);

                let dm = match data {
                    bridge_old::MessageData::Lamprey { .. } => {
                        // don't send messages from lamprey back to lamprey
                        return Ok(());
                    }
                    bridge_old::MessageData::Discord { message } => message,
                };

                // check if message has already been bridged
                if self
                    .handle
                    .bridge
                    .db
                    .message_get_by_discord_id(self.portal_id, dm.id)
                    .await?
                    .is_some()
                {
                    return Ok(());
                }

                let puppet = ly.sync_puppet_discord(dm).await?;

                let (parsed_content, allowed_mentions) = match transformer {
                    Some(t) => {
                        // PERF: fetch users concurrently (though, this might not be a good idea with sqlite?)
                        let mut user_mappings = HashMap::new();
                        for uid in t.mentioned_users() {
                            if let Ok(Some(u)) = self
                                .handle
                                .bridge
                                .db
                                .puppet_get_by_discord_id(uid.to_string())
                                .await
                            {
                                user_mappings.insert(uid.to_string(), u.lamprey_id);
                            }
                        }

                        // TODO: handle role and channel mappings

                        let (parsed, mentions) =
                            t.to_lamprey(&user_mappings, &HashMap::new(), &HashMap::new());
                        let parsed = if parsed.is_empty() {
                            None
                        } else {
                            Some(parsed)
                        };
                        (parsed, Some(mentions))
                    }
                    None => (None, None),
                };

                // TODO: ly -> async fn process_discord_message(&self, ...) -> Result<MessageCreate>
                let mut create = MessageCreate {
                    content: parsed_content,
                    mentions: allowed_mentions.unwrap_or_else(ParseMentions::nothing),
                    ..Default::default()
                };

                // populate reply_id
                if let Some(reference) = &dm.message_reference {
                    if dm.kind == serenity::all::MessageType::InlineReply
                        && reference.kind == serenity::all::MessageReferenceKind::Default
                    {
                        if let Some(discord_reply_id) = reference.message_id {
                            if let Some(msg) = self
                                .handle
                                .bridge
                                .db
                                .message_get_by_discord_id(self.portal_id, discord_reply_id)
                                .await?
                            {
                                if let Some(lamprey_reply_id) = msg.lamprey_message_id {
                                    create.reply_id = Some(lamprey_reply_id);
                                }
                            }
                        }
                    }
                }

                for att in &dm.attachments {
                    let mut import = ImportUrl::from(att.clone());
                    import.user_id = Some(puppet.id);
                    if let Ok(media) = ly.import_url(import).await {
                        create.attachments.push(MessageAttachmentCreate {
                            ty: MessageAttachmentCreateType::Media {
                                media: common::v2::types::media::MediaReference::Media {
                                    media_id: media.id,
                                },
                                alt: None,
                                filename: None,
                            },
                            // TODO: set spoiler field
                            spoiler: false,
                        });
                    }
                }

                // make sure the puppet is a room member, otherwise it won't be able to send any messages
                // PERF: don't send this request for every message, cache this
                if let Some(lamprey_cfg) = &self.portal.lamprey {
                    if ly
                        .http
                        .room_member_get(lamprey_cfg.room_id, puppet.id.into())
                        .await
                        .is_err()
                    {
                        ly.http
                            .room_member_add(
                                lamprey_cfg.room_id,
                                UserIdReq::UserId(puppet.id),
                                &RoomMemberPut::default(),
                            )
                            .await?;
                    }
                }

                let sent_message = ly
                    .http
                    .for_puppet(puppet.id)?
                    .message_create_with_options(MessageCreateOptions {
                        channel_id: self.portal.lamprey.as_ref().unwrap().channel_id,
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
                self.handle
                    .bridge
                    .db
                    .message_create(
                        self.portal_id,
                        bridge_old::Message {
                            id: crate::types::MessageId::new(),
                            source_platform: Platform::Discord,
                            attachments: vec![], // FIXME: populate from sent_message
                            portal_id: self.portal_id,
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
                        return Ok(());
                    }
                    bridge_old::MessageData::Discord { message } => message,
                };

                let puppet = ly.sync_puppet_discord(dm).await?;

                let Some(portal_msg) = self
                    .handle
                    .bridge
                    .db
                    .message_get_by_discord_id(self.portal_id, dm.id)
                    .await?
                else {
                    // message isn't bridged to lamprey, or it failed to bridge
                    return Ok(());
                };

                let Some(lamprey_message_id) = portal_msg.lamprey_message_id else {
                    return Ok(());
                };

                let mut attachments = vec![];
                for att in &dm.attachments {
                    // Check if this attachment was already imported
                    if let Some(existing_media_id) = portal_msg
                        .attachments
                        .iter()
                        .find(|(_, d_id)| d_id == &att.id)
                        .map(|(l_id, _)| *l_id)
                    {
                        attachments.push(MessageAttachmentCreate {
                            ty: MessageAttachmentCreateType::Media {
                                media: common::v2::types::media::MediaReference::Media {
                                    media_id: existing_media_id,
                                },
                                alt: None,
                                filename: None,
                            },
                            // TODO: populate spoiler field
                            spoiler: false,
                        });
                    } else {
                        // Import new attachment
                        let mut import = ImportUrl::from(att.clone());
                        import.user_id = Some(puppet.id);
                        if let Ok(media) = ly.import_url(import).await {
                            attachments.push(MessageAttachmentCreate {
                                ty: MessageAttachmentCreateType::Media {
                                    media: common::v2::types::media::MediaReference::Media {
                                        media_id: media.id,
                                    },
                                    alt: None,
                                    filename: None,
                                },
                                // TODO: populate spoiler field
                                spoiler: false,
                            });
                        }
                    }
                }

                // TODO: also edit embeds, components
                let patch = common::v1::types::MessagePatch {
                    content: Some(if dm.content.is_empty() {
                        None
                    } else {
                        Some(dm.content.clone())
                    }),
                    attachments: Some(attachments),
                    ..Default::default()
                };

                let edited = ly
                    .http
                    .message_edit(self.channel_id, lamprey_message_id, &patch)
                    .await?;

                let mut new_attachments = Vec::new();
                if let MessageType::DefaultMarkdown(m) = &edited.latest_version.message_type {
                    for (i, attachment) in m.attachments.iter().enumerate() {
                        let MessageAttachmentType::Media { media } = &attachment.ty;
                        if let Some(discord_att) = dm.attachments.get(i) {
                            new_attachments.push((media.id, discord_att.id));
                        }
                    }
                }

                let updated_message = crate::bridge_old::Message {
                    attachments: new_attachments,
                    ..portal_msg
                };

                // WARNING: if the bridge ever has more than two endpoints, i need to handle race conditions/conflicts/overwriting here
                self.handle
                    .bridge
                    .db
                    .message_update(self.portal_id, updated_message)
                    .await?;
            }
            PortalEvent::MessageDelete(message_id) => {
                if let Some(msg) = self.handle.bridge.db.message_get(*message_id).await? {
                    if let (Some(lamprey_message_id), Some(discord_message_id)) =
                        (msg.lamprey_message_id, msg.discord_message_id)
                    {
                        // TODO: propagate (or at least log) errors
                        // if http.message_delete fails it should still delete from the db(?)
                        let _ = ly
                            .http
                            .message_delete(self.channel_id, lamprey_message_id)
                            .await;
                        let _ = self
                            .handle
                            .bridge
                            .db
                            .message_delete_by_discord(self.portal_id, discord_message_id)
                            .await;
                    }
                }
            }

            PortalEvent::ReactionCreate(message_id, reaction_key, user) => {
                if let Some(msg) = self.handle.bridge.db.message_get(*message_id).await? {
                    if let Some(lamprey_message_id) = msg.lamprey_message_id {
                        let key = match reaction_key {
                            bridge::ReactionKey::Lamprey(_key) => return Ok(()),
                            bridge::ReactionKey::Discord(key) => match key {
                                discord::ReactionType::Unicode(emoji) => {
                                    ReactionKeyParam::Text(emoji.to_owned())
                                }
                                // TODO: support custom reactions
                                // discord::ReactionType::Custom { animated, id, name } => todo!(),
                                _ => return Ok(()),
                            },
                        };
                        ly.http
                            .for_puppet(user.lamprey_id)?
                            .reaction_create(
                                self.channel_id,
                                lamprey_message_id,
                                key.to_string(),
                                user.lamprey_id.into(),
                            )
                            .await?;
                    }
                }
            }
            PortalEvent::ReactionDelete(message_id, reaction_key, user) => {
                if let Some(msg) = self.handle.bridge.db.message_get(*message_id).await? {
                    if let Some(lamprey_message_id) = msg.lamprey_message_id {
                        let key = match reaction_key {
                            bridge::ReactionKey::Lamprey(_key) => return Ok(()),
                            bridge::ReactionKey::Discord(key) => match key {
                                discord::ReactionType::Unicode(emoji) => {
                                    ReactionKeyParam::Text(emoji.to_owned())
                                }
                                // TODO: support custom reactions
                                // discord::ReactionType::Custom { animated, id, name } => todo!(),
                                _ => return Ok(()),
                            },
                        };
                        ly.http
                            .for_puppet(user.lamprey_id)?
                            .reaction_delete(
                                self.channel_id,
                                lamprey_message_id,
                                key.to_string(),
                                user.lamprey_id.into(),
                            )
                            .await?;
                    }
                }
            }
            PortalEvent::ReactionDeleteKey(message_id, reaction_key) => {
                if let Some(msg) = self.handle.bridge.db.message_get(*message_id).await? {
                    if let Some(lamprey_message_id) = msg.lamprey_message_id {
                        let key = match reaction_key {
                            bridge::ReactionKey::Lamprey(_key) => return Ok(()),
                            bridge::ReactionKey::Discord(key) => match key {
                                discord::ReactionType::Unicode(emoji) => {
                                    ReactionKeyParam::Text(emoji.to_owned())
                                }
                                // TODO: support custom reactions
                                // discord::ReactionType::Custom { animated, id, name } => todo!(),
                                _ => return Ok(()),
                            },
                        };

                        ly.http
                            .reaction_delete_key(
                                self.channel_id,
                                lamprey_message_id,
                                key.to_string(),
                            )
                            .await?;
                    }
                }
            }
            PortalEvent::ReactionDeleteAll(message_id) => {
                if let Some(msg) = self.handle.bridge.db.message_get(*message_id).await? {
                    if let Some(lamprey_message_id) = msg.lamprey_message_id {
                        ly.http
                            .reaction_delete_all(self.channel_id, lamprey_message_id)
                            .await?;
                    }
                }
            }
            _ => {}
        }

        Ok(())
    }
}
