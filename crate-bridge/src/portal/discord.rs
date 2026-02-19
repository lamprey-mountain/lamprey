use anyhow::Result;
use common::v1::types::{self, media::MediaRef, util::Diff, EmbedCreate};
use reqwest::Url;
use serenity::all::{
    Message as DcMessage, MessageId as DcMessageId, MessageReferenceKind,
    MessageType as DcMessageType, MessageUpdateEvent as DcMessageUpdate, Reaction as DcReaction,
};
use std::str::FromStr;
use time::OffsetDateTime;
use tracing::{debug, info};

use crate::db::{AttachmentMetadata, Data, MessageMetadata, Puppet};
use crate::portal::Portal;

impl Portal {
    pub(super) async fn sync_discord_member_nick(
        &self,
        user_id: serenity::model::id::UserId,
        nick: Option<String>,
    ) -> Result<()> {
        let ly = self.globals.lamprey_handle().await?;
        let Some(puppet) = self
            .globals
            .get_puppet("discord", &user_id.to_string())
            .await?
        else {
            debug!("no puppet for user {}", user_id);
            return Ok(());
        };

        let patch = types::RoomMemberPatch {
            override_name: Some(nick),
            override_description: None,
            mute: None,
            deaf: None,
            roles: None,
            timeout_until: None,
        };

        ly.room_member_patch(self.room_id(), puppet.id.into(), &patch)
            .await?;
        debug!("synced nickname for user {}", user_id);

        Ok(())
    }

    pub(super) async fn handle_discord_message_create(&mut self, message: DcMessage) -> Result<()> {
        if let Some(member) = &message.member {
            self.sync_discord_member_nick(message.author.id, member.nick.clone())
                .await?;
        }
        let ly = self.globals.lamprey_handle().await?;
        let existing = self.globals.get_message_dc(message.id).await?;
        if existing.is_some() {
            debug!("message already bridged");
            return Ok(());
        }

        let mut puppet = ly
            .puppet_ensure(
                message.author.display_name().to_owned(),
                message.author.id.to_string(),
                self.room_id(),
                message.author.bot,
            )
            .await?;
        debug!("ensured puppet");
        let user_id = puppet.id;

        let db_puppet = self
            .globals
            .get_puppet("discord", &message.author.id.to_string())
            .await?;

        let mut user_patch = types::UserPatch::default();

        // Avatar
        let current_ext_avatar = db_puppet.as_ref().and_then(|p| p.ext_avatar.as_deref());
        let new_ext_avatar = message.author.avatar_url();
        if current_ext_avatar != new_ext_avatar.as_deref() {
            if let Some(url) = new_ext_avatar {
                info!("set user pfp for {}", user_id);
                let name = if url.ends_with(".gif") {
                    "avatar.gif"
                } else {
                    "avatar.png"
                };
                let bytes = reqwest::get(url).await?.error_for_status()?.bytes().await?;
                let media = ly
                    .media_upload(name.to_owned(), bytes.to_vec(), user_id)
                    .await?;
                user_patch.avatar = Some(Some(media.id));
            } else {
                info!("remove user pfp for {}", user_id);
                user_patch.avatar = Some(None);
            }
        }

        // Banner
        let current_ext_banner = db_puppet.as_ref().and_then(|p| p.ext_banner.as_deref());
        let new_ext_banner = message.author.banner_url();
        if current_ext_banner != new_ext_banner.as_deref() {
            if let Some(url) = new_ext_banner {
                info!("set user banner for {}", user_id);
                let name = if url.ends_with(".gif") {
                    "banner.gif"
                } else {
                    "banner.png"
                };
                let bytes = reqwest::get(&url)
                    .await?
                    .error_for_status()?
                    .bytes()
                    .await?;
                let media = ly
                    .media_upload(name.to_owned(), bytes.to_vec(), user_id)
                    .await?;
                user_patch.banner = Some(Some(media.id));
            } else {
                info!("remove user banner for {}", user_id);
                user_patch.banner = Some(None);
            }
        }

        if user_patch.changes(&puppet) {
            puppet = ly.user_update(user_id, &user_patch).await?;
        }

        self.globals
            .insert_puppet(Puppet {
                id: *user_id,
                ext_platform: "discord".to_owned(),
                ext_id: message.author.id.to_string(),
                ext_avatar: message.author.avatar_url(),
                ext_banner: message.author.banner_url(),
                name: puppet.name,
                avatar: puppet.avatar.map(|a| a.to_string()),
                banner: puppet.banner.map(|b| b.to_string()),
                bot: Some(puppet.bot),
            })
            .await?;
        debug!("inserted puppet");

        let mut req = types::MessageCreate {
            content: None,
            attachments: vec![],
            metadata: None,
            reply_id: None,
            override_name: message
                .member
                .and_then(|m| m.nick)
                .or(message.author.global_name)
                .or(Some(message.author.name.clone())),
            embeds: vec![],
            created_at: Some(
                OffsetDateTime::from_unix_timestamp(message.timestamp.unix_timestamp())
                    .unwrap()
                    .into(),
            ),
            mentions: Default::default(),
        };
        for a in &message.attachments {
            let bytes = a.download().await?;
            debug!("downloaded attachment");
            let media = ly
                .media_upload(a.filename.to_owned(), bytes.into(), user_id)
                .await?;
            debug!("reuploaded attachment");
            self.globals
                .insert_attachment(AttachmentMetadata {
                    chat_id: media.id,
                    discord_id: a.id,
                })
                .await?;
            debug!("saved attachment metadata to db");
            req.attachments.push(MediaRef { id: media.id });
        }
        for emb in message.embeds.iter().cloned() {
            let author_avatar = if let Some(url) = emb
                .author
                .as_ref()
                .and_then(|a| a.proxy_icon_url.as_deref())
            {
                let filename = Url::from_str(url)?
                    .path_segments()
                    .and_then(|s| s.last())
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_owned())
                    .unwrap_or_else(|| "file.bin".to_owned());
                let bytes = reqwest::get(url).await?.bytes().await?;
                let media = ly
                    .media_upload(filename.to_owned(), bytes.into(), user_id)
                    .await?;
                Some(MediaRef { id: media.id })
            } else {
                None
            };
            let create = EmbedCreate {
                url: emb.url.and_then(|u| u.parse().ok()),
                title: emb.title,
                description: emb.description,
                color: emb.colour.map(|c| format!("#{}", c.hex())),
                media: if let Some(url) = emb.image.as_ref().and_then(|i| i.proxy_url.as_deref()) {
                    let filename = Url::from_str(url)?
                        .path_segments()
                        .and_then(|s| s.last())
                        .filter(|s| !s.is_empty())
                        .map(|s| s.to_owned())
                        .unwrap_or_else(|| "file.bin".to_owned());
                    let bytes = reqwest::get(url).await?.bytes().await?;
                    let media = ly
                        .media_upload(filename.to_owned(), bytes.into(), user_id)
                        .await?;
                    Some(MediaRef { id: media.id })
                } else {
                    None
                },
                thumbnail: if let Some(url) =
                    emb.thumbnail.as_ref().and_then(|t| t.proxy_url.as_deref())
                {
                    let filename = Url::from_str(url)?
                        .path_segments()
                        .and_then(|s| s.last())
                        .filter(|s| !s.is_empty())
                        .map(|s| s.to_owned())
                        .unwrap_or_else(|| "file.bin".to_owned());
                    let bytes = reqwest::get(url).await?.bytes().await?;
                    let media = ly
                        .media_upload(filename.to_owned(), bytes.into(), user_id)
                        .await?;
                    Some(MediaRef { id: media.id })
                } else {
                    None
                },
                author_name: emb.author.as_ref().map(|a| a.name.clone()),
                author_url: emb.author.and_then(|a| a.url).and_then(|u| u.parse().ok()),
                author_avatar,
            };
            req.embeds.push(create);
        }
        req.content = match message.kind {
            DcMessageType::Regular | DcMessageType::InlineReply => {
                if message.content.is_empty() {
                    if message.attachments.is_empty() && message.embeds.is_empty() {
                        Some("(sticker, poll, or other unsupported message type)".to_string())
                    } else {
                        None
                    }
                } else {
                    Some(message.content)
                }
            }
            other => Some(format!("(discord message: {:?})", other)),
        };
        match message.message_reference.map(|r| r.kind) {
            Some(MessageReferenceKind::Default) => {
                if let Some(reply) = message.referenced_message {
                    let row = self.globals.get_message_dc(reply.id).await?;
                    req.reply_id = row.map(|r| r.chat_id);
                }
            }
            Some(MessageReferenceKind::Forward) => {
                // TODO: support forwards once serenity supports them
            }
            Some(_) | None => {}
        };
        let thread_id = self.thread_id();
        let res = ly.message_create(thread_id, user_id, req).await?;
        debug!("sent message");
        self.globals
            .insert_message(MessageMetadata {
                chat_id: res.id,
                chat_thread_id: thread_id,
                discord_id: message.id,
                discord_channel_id: message.channel_id,
            })
            .await?;
        Ok(())
    }

    pub(super) async fn handle_discord_message_update(
        &mut self,
        update: DcMessageUpdate,
        new_message: Option<DcMessage>,
    ) -> Result<()> {
        if let Some(message) = new_message {
            if let Some(member) = &message.member {
                self.sync_discord_member_nick(message.author.id, member.nick.clone())
                    .await?;
            }
        }
        let ly = self.globals.lamprey_handle().await?;
        let existing = self.globals.get_message_dc(update.id).await?;
        let Some(existing) = existing else {
            debug!("message already bridged");
            return Ok(());
        };

        let message = ly
            .message_get(existing.chat_thread_id, existing.chat_id)
            .await?;
        let user_id = message.author_id;

        let mut req = types::MessagePatch {
            edited_at: update.edited_timestamp.map(|t| {
                OffsetDateTime::from_unix_timestamp(t.unix_timestamp())
                    .unwrap()
                    .into()
            }),
            ..Default::default()
        };
        req.attachments = if let Some(atts) = &update.attachments {
            let mut v = vec![];
            for att in atts {
                let existing = self.globals.get_attachment_dc(att.id).await?;
                if let Some(existing) = existing {
                    v.push(MediaRef {
                        id: existing.chat_id,
                    });
                    continue;
                }
                let bytes = att.download().await?;
                let media = ly
                    .media_upload(att.filename.to_owned(), bytes.into(), user_id)
                    .await?;
                self.globals
                    .insert_attachment(AttachmentMetadata {
                        chat_id: media.id,
                        discord_id: att.id,
                    })
                    .await?;
                v.push(MediaRef { id: media.id });
            }
            Some(v)
        } else {
            None
        };
        req.content = match update.kind {
            Some(k) => Some(match k {
                DcMessageType::Regular | DcMessageType::InlineReply
                    if update.content.as_ref().is_none_or(|c| c.is_empty())
                        && update.attachments.as_ref().is_none_or(|a| a.is_empty()) =>
                {
                    Some("(empty message, or sticker/embeds only)".to_string())
                }
                DcMessageType::Regular | DcMessageType::InlineReply => update.content.clone(),
                other => Some(format!("(discord message: {:?})", other)),
            }),
            None => None,
        };

        let thread_id = self.thread_id();
        ly.message_update(thread_id, existing.chat_id, user_id, req)
            .await?;
        Ok(())
    }

    pub(super) async fn handle_discord_message_delete(
        &mut self,
        message_id: DcMessageId,
    ) -> Result<()> {
        let ly = self.globals.lamprey_handle().await?;
        let Some(existing) = self.globals.get_message_dc(message_id).await? else {
            debug!("message doesnt exist or is already deleted");
            return Ok(());
        };

        let message = ly
            .message_get(existing.chat_thread_id, existing.chat_id)
            .await?;
        let user_id = message.author_id;

        self.globals.delete_message_dc(message_id).await?;
        let thread_id = self.thread_id();
        ly.message_delete(thread_id, existing.chat_id, user_id)
            .await?;
        Ok(())
    }

    pub(super) async fn handle_discord_reaction_add(
        &mut self,
        add_reaction: DcReaction,
    ) -> Result<()> {
        if let (Some(user_id), Some(member)) = (add_reaction.user_id, &add_reaction.member) {
            self.sync_discord_member_nick(user_id, member.nick.clone())
                .await?;
        }
        let ly = self.globals.lamprey_handle().await?;
        let Some(user_id) = add_reaction.user_id else {
            debug!("missing user_id");
            return Ok(());
        };

        let Some(message) = self.globals.get_message_dc(add_reaction.message_id).await? else {
            debug!("missing message");
            return Ok(());
        };

        let puppet = ly
            .puppet_ensure(
                add_reaction
                    .member
                    .as_ref()
                    .map(|m| {
                        m.nick
                            .as_deref()
                            .unwrap_or_else(|| &m.user.display_name())
                            .to_owned()
                    })
                    .unwrap_or_else(|| user_id.to_string()),
                user_id.to_string(),
                self.room_id(),
                add_reaction.member.as_ref().map_or(false, |m| m.user.bot),
            )
            .await?;

        ly.message_react(
            self.thread_id(),
            message.chat_id,
            puppet.id,
            add_reaction.emoji.to_string(),
        )
        .await?;
        Ok(())
    }

    pub(super) async fn handle_discord_reaction_remove(
        &mut self,
        removed_reaction: DcReaction,
    ) -> Result<()> {
        if let (Some(user_id), Some(member)) = (removed_reaction.user_id, &removed_reaction.member)
        {
            self.sync_discord_member_nick(user_id, member.nick.clone())
                .await?;
        }
        let ly = self.globals.lamprey_handle().await?;
        let Some(user_id) = removed_reaction.user_id else {
            debug!("missing user_id");
            return Ok(());
        };

        let Some(message) = self
            .globals
            .get_message_dc(removed_reaction.message_id)
            .await?
        else {
            debug!("missing message");
            return Ok(());
        };

        let puppet = ly
            .puppet_ensure(
                removed_reaction
                    .member
                    .as_ref()
                    .map(|m| {
                        m.nick
                            .as_deref()
                            .unwrap_or_else(|| &m.user.display_name())
                            .to_owned()
                    })
                    .unwrap_or_else(|| user_id.to_string()),
                user_id.to_string(),
                self.room_id(),
                removed_reaction
                    .member
                    .as_ref()
                    .map_or(false, |m| m.user.bot),
            )
            .await?;

        ly.message_unreact(
            self.thread_id(),
            message.chat_id,
            puppet.id,
            removed_reaction.emoji.to_string(),
        )
        .await?;
        Ok(())
    }

    pub(super) async fn handle_discord_typing(
        &mut self,
        user_id: serenity::model::id::UserId,
    ) -> Result<()> {
        let ly = self.globals.lamprey_handle().await?;
        let Some(puppet) = self
            .globals
            .get_puppet("discord", &user_id.to_string())
            .await?
        else {
            debug!("missing puppet");
            return Ok(());
        };

        ly.typing_start(self.thread_id(), puppet.id.into()).await?;
        Ok(())
    }
}
