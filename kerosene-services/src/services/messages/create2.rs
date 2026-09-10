use std::{collections::HashMap, time::Duration};

use common::{
    v1::types::{
        Channel, EmbedCreate, Mentions, MentionsChannel, MentionsEmoji, MentionsRole, MentionsUser,
        Message, MessageAttachment, MessageAttachmentCreate, MessageAttachmentCreateType,
        MessageAttachmentType, MessageCreate, MessageDefaultMarkdown, MessageInteraction,
        MessagePatch, MessageSync, MessageType, MessageVersion, Permission, User,
        components::{self, Component, ComponentType, Components},
        emoji::EmojiOwner,
        util::Time,
    },
    v2::types::{
        AUTOMOD_USER_ID, ChannelId, MessageId, RoomId, SERVER_USER_ID, UserId,
        media::MediaReference,
    },
};
use futures::{FutureExt, TryFutureExt, try_join};
use futures_util::future::try_join_all;
use kerosene_core::{
    error::{ApiError, ErrorCode},
    types::permission::requirements::Requirements,
};
use lamprey_backend_data_postgres::{DbMessageAttachment, MediaLinkType};
use validator::Validate;

use crate::{
    globals::messaging::Broadcast,
    prelude::*,
    services::{
        automod::AutomodContext,
        messages::{ServiceMessages, markdown, util::MediaRegistry2},
    },
    types::DbMessageCreate,
};

// remove Author
// fn arst(user: &User) {
//     user.id == SERVER_USER_ID;
//     user.id == AUTOMOD_USER_ID;
//     user.webhook.is_some();
// }

/// A request to create a new message.
#[derive(Debug)]
pub struct Create {
    pub id: MessageId,
    pub channel_id: ChannelId,
    pub user_id: UserId,
    pub payload: Box<CreateType>,
    pub nonce: Option<String>,
    pub timestamp: Option<Time>,
    pub interaction: Option<MessageInteraction>,
}

/// What kind of message are we creating?
#[derive(Debug)]
pub enum CreateType {
    Default(MessageCreate),
    ThreadInitial(MessageCreate),
    Custom(MessageType),
}

impl CreateType {
    pub fn message_create(&self) -> Option<&MessageCreate> {
        match self {
            CreateType::Default(m) | CreateType::ThreadInitial(m) => Some(m),
            CreateType::Custom(_m) => None,
        }
    }

    pub fn validate(&self) -> Result<()> {
        match self.message_create() {
            Some(m) => m.validate()?,
            None => {}
        }
        Ok(())
    }

    pub fn attachments(&self) -> Option<&[MessageAttachmentCreate]> {
        self.message_create().map(|m| m.attachments.as_slice())
    }

    pub fn embeds(&self) -> Option<&[EmbedCreate]> {
        self.message_create().map(|m| m.embeds.as_slice())
    }
}

/// A request to edit an existing message.
#[derive(Debug)]
pub struct Edit {
    pub id: MessageId,
    pub channel_id: ChannelId,
    pub user_id: UserId,
    pub payload: Box<MessagePatch>,
    pub nonce: Option<String>,
    pub timestamp: Option<Time>,
}

impl Create {
    pub fn new_default(body: MessageCreate, channel_id: ChannelId, user_id: UserId) -> Self {
        Self::new(CreateType::Default(body), channel_id, user_id)
    }

    pub fn new(payload: CreateType, channel_id: ChannelId, user_id: UserId) -> Self {
        Self {
            payload: Box::new(payload),
            channel_id,
            user_id,
            id: MessageId::new(),
            nonce: None,
            timestamp: None,
            interaction: None,
        }
    }

    /// explicitly set an id for this message
    pub fn id(mut self, id: MessageId) -> Self {
        self.id = id;
        self
    }

    /// set the nonce (idempotency-key)
    pub fn nonce(mut self, nonce: Option<String>) -> Self {
        self.nonce = nonce;
        self
    }

    /// override the `created_at` timestamp for the message
    pub fn timestamp(mut self, timestamp: Option<Time>) -> Self {
        self.timestamp = timestamp;
        self
    }

    /// set interaction metadata for the message
    pub fn interaction(mut self, interaction: Option<MessageInteraction>) -> Self {
        self.interaction = interaction;
        self
    }
}

impl Edit {
    pub fn new(
        body: MessagePatch,
        message_id: MessageId,
        channel_id: ChannelId,
        user_id: UserId,
    ) -> Self {
        Self {
            id: message_id,
            channel_id,
            user_id,
            payload: Box::new(body),
            nonce: None,
            timestamp: None,
        }
    }

    /// set the nonce (idempotency-key)
    pub fn nonce(mut self, nonce: Option<String>) -> Self {
        self.nonce = nonce;
        self
    }

    /// override the `created_at` timestamp for the message version
    pub fn timestamp(mut self, timestamp: Option<Time>) -> Self {
        self.timestamp = timestamp;
        self
    }
}

fn calculate_requirements(create: &Create, channel: &Channel) -> Requirements {
    let mut re = Requirements::new_channel(create.channel_id);
    re.slowmode_message();

    if channel.is_thread() {
        re.permission(Permission::MessageCreateThread);
    } else {
        re.permission(Permission::MessageCreate);
    }

    if create.payload.attachments().is_some() {
        re.permission(Permission::MessageAttachments);
    }

    if create.payload.embeds().is_some() {
        re.permission(Permission::MessageEmbeds);
    }

    if create.timestamp.is_some() {
        re.permission(Permission::IntegrationsBridge);
    }

    re
}

fn message_to_db(m: &Message) -> DbMessageCreate {
    let (attachments, embeds, components, message_type) = match &m.latest_version.message_type {
        MessageType::DefaultMarkdown(md) | MessageType::ThreadInitial(md) => (
            md.attachments
                .iter()
                .map(|a| match &a.ty {
                    MessageAttachmentType::Media { media } => DbMessageAttachment {
                        media_id: media.id,
                        spoiler: a.spoiler,
                    },
                    // MessageAttachmentType::Forward { .. } => todo!("handle forward"),
                })
                .collect(),
            md.embeds.clone(),
            md.components.clone().into_thin().inner,
            m.latest_version.message_type.clone(),
        ),
        _ => (
            vec![],
            vec![],
            vec![],
            m.latest_version.message_type.clone(),
        ),
    };

    DbMessageCreate {
        id: Some(m.id),
        channel_id: m.channel_id,
        attachments,
        author_id: m.author_id,
        embeds,
        components,
        message_type,
        created_at: Some(m.created_at.into()),
        removed_at: m.removed_at.map(|t| t.into()),
        mentions: m.latest_version.mentions.clone(),
        flume: m.flume.as_ref().and_then(|a| serde_json::to_value(a).ok()),
        interaction: m
            .interaction
            .as_ref()
            .and_then(|a| serde_json::to_value(a).ok()),

        // NOTE: ephemeral messages are never returned in the db?
        ephemeral: m.ephemeral,
    }
}

impl ServiceMessages {
    pub async fn create2(&self, create: Create) -> Result<Message> {
        let srv = self.globals.services();
        let (channel, user) = futures::try_join!(
            srv.channels.get(create.channel_id, None),
            srv.users.get(create.user_id, None),
        )?;

        // 1. authorize
        create.payload.validate()?;
        let re = calculate_requirements(&create, &channel);

        // if message author is a puppet, use the puppeteer's permissions
        // NOTE: this behavior is intentionally different from before!
        let auth_user_id = if let Some(puppet) = &user.puppet {
            (*puppet.owner_id).into()
        } else {
            user.id
        };

        // let perms = srv
        //     .perms
        //     .for_channel3(Some(auth_user_id), channel.id)
        //     .await?;
        // TODO: use this instead
        // let srv.perms.enforce(...).await?;

        let removed_at = async {
            let Some(room_id) = channel.room_id else {
                return Ok(None);
            };

            let Some(json) = create.payload.message_create() else {
                return Ok(None);
            };

            let automod = srv.automod.load(room_id).await?;
            let ctx = AutomodContext {
                room_id,
                user_id: create.user_id,
                channel_id: Some(create.channel_id),
                message_id: Some(create.id),
            };

            let scan = automod.scan(json, &ctx).await;
            if scan.is_triggered() {
                srv.automod.enforce(&scan, &ctx).await?;
                scan.ensure_unblocked()?;
                if scan.should_remove() {
                    return Ok(Some(Time::now_utc()));
                }
            }

            Result::Ok(None)
        };

        // 2. prepare

        // sanitize content
        let content = async {
            if let Some((content, mentions)) = create
                .payload
                .message_create()
                .and_then(|c| c.content.as_deref().map(|a| (a, &c.mentions)))
            {
                let allow_external_emoji = todo!();
                let mentions_ids = markdown::parse_mentions(content, mentions);
                let mentions = self
                    .sanitize_mentions2(mentions_ids, channel.room_id, allow_external_emoji)
                    .await?;
                let allowed_emoji: Vec<_> = mentions.emojis.iter().map(|e| e.id).collect();
                let content = markdown::strip_emoji(content, &allowed_emoji);
                Ok(Some((content, mentions)))
            } else {
                Ok(None)
            }
        };

        // process attachments
        let attachments = async {
            if let Some(attachments) = create.payload.message_create().map(|c| &c.attachments)
                && !attachments.is_empty()
            {
                let mut futs = Vec::new();
                for att in attachments.clone() {
                    let MessageAttachmentCreateType::Media { media, .. } = att.ty;
                    futs.push(
                        self.fetch_media2(media, create.user_id)
                            .map_ok(move |media| {
                                MessageAttachment {
                                    // TODO: add alt, filename fields to MessageAttachmentType::Media
                                    ty: MessageAttachmentType::Media { media },
                                    spoiler: att.spoiler,
                                }
                            }),
                    );
                }
                try_join_all(futs).await
            } else {
                Ok(vec![])
            }
        };

        // process embeds
        let embeds = async {
            if let Some(embeds) = create.payload.message_create().map(|c| &c.embeds)
                && !embeds.is_empty()
            {
                let mut futs = Vec::new();
                for e in embeds.clone() {
                    futs.push(self.embed_from_create(e, create.user_id));
                }
                try_join_all(futs).await
            } else {
                Ok(vec![])
            }
        };

        // process components
        let components = async {
            if let Some(components) = create
                .payload
                .message_create()
                .and_then(|c| c.components.as_ref())
                && !components.is_empty()
            {
                let mut media_refs = vec![];
                components.collect_media_refs(&mut media_refs);

                let mut futs = Vec::new();
                for m in media_refs.clone() {
                    futs.push(self.fetch_media2(m, create.user_id));
                }

                let media_fetched = try_join_all(futs).await?;
                let mut media_map: HashMap<_, _> =
                    media_refs.into_iter().zip(media_fetched.clone()).collect();

                let parsed = components.clone().parse(None, &|m| {
                    // TODO: use media_map.remove(&m); requires FnMut
                    Ok(media_map
                        .get(&m)
                        .expect("media_map missing a media reference")
                        .to_owned())
                })?;

                Ok(Some((parsed, media_fetched)))
            } else {
                Ok(None)
            }
        };

        let (content, attachments, embeds, components, removed_at) =
            try_join!(content, attachments, embeds, components, removed_at)?;

        // collect media
        let mut registry = MediaRegistry2::new();
        for att in &attachments {
            let MessageAttachmentType::Media { media } = &att.ty;
            registry.insert(media);
        }
        for embed in &embeds {
            if let Some(media) = &embed.media {
                registry.insert(media);
            }
            if let Some(thumbnail) = &embed.thumbnail {
                registry.insert(thumbnail);
            }
            if let Some(author_avatar) = &embed.author_avatar {
                registry.insert(author_avatar);
            }
        }
        for media in components
            .as_ref()
            .map(|(_, a)| a.as_slice())
            .unwrap_or_default()
        {
            registry.insert(media);
        }
        registry.check(create.user_id)?;

        // 3. commit

        // skip everything except media validation for ephemeral messages
        let ephemeral = create.payload.message_create().is_some_and(|c| c.ephemeral);

        let mut txn = self.globals.begin().await?;

        // validate media
        // PERF: batch media link query
        for media in registry.media() {
            let existing = txn.media_link_select(media.id).await?;
            let already_linked_to_this = existing.iter().any(|l| {
                l.link_type == MediaLinkType::Message && l.target_id == create.id.into_inner()
            });

            if !existing.is_empty() && !already_linked_to_this {
                return Err(Error::ApiError(ApiError::from_code(
                    ErrorCode::MediaAlreadyUsed,
                )));
            }
        }

        // construct message
        let message = Message {
            id: create.id,
            channel_id: channel.id,
            room_id: channel.room_id,
            latest_version: MessageVersion {
                version_id: (*create.id).into(),
                author_id: None,
                message_type: match &*create.payload {
                    CreateType::Default(m) | CreateType::ThreadInitial(m) => {
                        let inner = MessageDefaultMarkdown {
                            content: content.as_ref().map(|(s, _)| s.to_owned()),
                            attachments: attachments.clone(),
                            metadata: m.metadata.clone(),
                            reply_id: m.reply_id,
                            embeds: embeds.clone(),
                            components: components
                                .as_ref()
                                .map(|(c, _)| c.clone())
                                .unwrap_or_default(),
                        };
                        if matches!(*create.payload, CreateType::ThreadInitial(_)) {
                            MessageType::ThreadInitial(inner)
                        } else {
                            MessageType::DefaultMarkdown(inner)
                        }
                    }
                    CreateType::Custom(m) => m.clone(),
                },
                mentions: content
                    .as_ref()
                    .map(|(_, m)| m.to_owned())
                    .unwrap_or_default(),
                created_at: create.timestamp.unwrap_or_else(Time::now_utc),
                deleted_at: None,
            },
            pinned: None,
            reactions: Default::default(),
            deleted_at: None,
            removed_at,
            created_at: create.timestamp.unwrap_or_else(Time::now_utc),
            author_id: create.user_id,
            thread: None,
            flume: None,
            interaction: create.interaction.clone(),
            ephemeral,
        };

        if !ephemeral {
            // insert message
            // PERF: avoid cloning, maybe move out of create
            // PERF: after media validation, have media registry store media ids instead of media so i can move embeds and components into DbMessageCreate
            txn.message_create(message_to_db(&message)).await?;

            // insert message links
            for media in registry.media() {
                txn.media_link_insert(media.id, *create.id, MediaLinkType::Message)
                    .await?;
                txn.media_link_insert(media.id, *create.id, MediaLinkType::MessageVersion)
                    .await?;
            }

            // upsert slowmode
            if let Some(delay) = channel.slowmode_message {
                let expires_at = Time::now_utc() + Duration::from_secs(delay);
                // TODO: rename to expires_at
                txn.channel_set_message_slowmode_expire_at(channel.id, create.user_id, expires_at)
                    .await?;
            }
        }

        txn.commit().await?;

        // 4. finalize

        let update_last_message_ids =
            srv.channels
                .update_last_message_ids(channel.id, create.id, (*create.id).into());

        let unarchive = async {
            if channel.is_archived() {
                // FIXME: unarchive channel
                // srv.channels
                //     .update(
                //         auth,
                //         op.channel.id,
                //         ChannelPatch {
                //             archived: Some(false),
                //             ..Default::default()
                //         },
                //     )
                //     .await?;
            }
        };

        let ensure_member = async {
            if channel.is_thread() {
                // FIXME: ensure thread membership

                // let mut txn = self.globals.begin().await?;
                // if txn.thread_member_get(thread_id, user_id).await.is_err() {
                //     txn.thread_member_put(thread_id, user_id, ThreadMemberPut::default())
                //         .await?;

                //     // NOTE: i need to commit this to see the update in next get
                //     txn.commit().await?;

                //     srv.channels.invalidate(thread_id).await; // NOTE: do i need this? presumably only member count is dirty

                //     let thread_member = self
                //         .globals
                //         .begin_read()
                //         .await?
                //         .thread_member_get(thread_id, user_id)
                //         .await?;
                //     let msg = MessageSync::ThreadMemberUpsert {
                //         room_id: op.channel.room_id,
                //         thread_id,
                //         added: vec![thread_member],
                //         removed: vec![],
                //     };
                //     self.globals
                //         .messaging()
                //         .broadcast_channel(thread_id, msg)
                //         .await?;
                // } else {
                //     txn.commit().await?;
                // }
            }
        };

        let unfurl = async {
            // TODO: inline spawn_unfurler_tasks
        };

        let send_notifs = srv.notifications.process_message(&channel, &message);

        let broadcast_sync = async {
            let sync = MessageSync::MessageCreate {
                message: message.clone(),
            };
            let broadcast = Broadcast::sync(sync).with_option_nonce(create.nonce.as_deref());

            self.globals
                .messaging()
                .broadcast_channel(channel.id, broadcast)
                .await
        };

        // TODO: spawn these tasks in the background
        let _ = futures::join!(
            update_last_message_ids,
            unarchive,
            ensure_member,
            unfurl,
            send_notifs,
            broadcast_sync
        );

        Ok(message)
    }

    pub async fn edit2(&self, edit: Edit) -> Result<Message> {
        let srv = self.globals.services();
        todo!()
    }

    async fn sanitize_mentions2(
        &self,
        mentions_ids: lamprey_backend_data_postgres::MentionsIds,
        room_id: Option<RoomId>,
        allow_external_emoji: bool,
    ) -> Result<Mentions> {
        let srv = self.globals.services();

        let users_and_roles = async {
            if let Some(room_id) = room_id {
                let room_handle = srv.rooms.load(room_id);
                let room = room_handle.ready(true).await?;

                let users = srv
                    .users
                    .get_many(&mentions_ids.users)
                    .await?
                    .into_iter()
                    .map(|user| {
                        let resolved_name = room
                            .members
                            .get(&user.id)
                            .and_then(|m| m.member.override_name.as_deref())
                            .unwrap_or(&user.name)
                            .to_owned();
                        MentionsUser {
                            id: user.id,
                            resolved_name,
                        }
                    })
                    .collect::<Vec<_>>();

                let roles = mentions_ids
                    .roles
                    .iter()
                    .cloned()
                    .filter(|role_id| room.roles.contains_key(role_id))
                    .map(|id| MentionsRole { id })
                    .collect::<Vec<_>>();

                Result::Ok((users, roles))
            } else {
                let users = srv
                    .users
                    .get_many(&mentions_ids.users)
                    .await?
                    .into_iter()
                    .map(|user| MentionsUser {
                        id: user.id,
                        resolved_name: user.name.clone(),
                    })
                    .collect::<Vec<_>>();
                Result::Ok((users, vec![]))
            }
        };

        let channels = srv
            .channels
            .get_many(&mentions_ids.channels, None)
            .map_ok(|chans| {
                chans
                    .into_iter()
                    .map(|c| MentionsChannel {
                        id: c.id,
                        room_id: c.room_id,
                        ty: c.ty,
                        name: c.name,
                    })
                    .collect()
            });

        let emojis = srv
            .cache
            .emoji_get_many(&mentions_ids.emojis)
            .map_ok(|emojis| {
                emojis
                    .into_iter()
                    .filter(|e| {
                        allow_external_emoji
                            || room_id.is_some_and(|room_id| {
                                e.owner == Some(EmojiOwner::Room { room_id })
                            })
                    })
                    .map(|e| MentionsEmoji {
                        id: e.id,
                        name: e.name,
                        animated: e.animated,
                    })
                    .collect()
            });

        let ((users, roles), channels, emojis) = try_join!(users_and_roles, channels, emojis)?;

        Ok(Mentions {
            users,
            roles,
            channels,
            emojis,
            everyone: mentions_ids.everyone,
        })
    }
}
