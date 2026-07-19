use std::marker::PhantomData;

use common::v1::types::components::{self, ComponentThin, Components};
use common::v1::types::error::{ApiError, ErrorCode};
use common::v1::types::message::MessageAttachmentCreateType;
use common::v1::types::misc::Time;
use common::v1::types::{
    Channel, ChannelId, ChannelPatch, Mentions, Message, MessageAttachmentType, MessageCreate,
    MessageId, MessagePatch, MessageSync, MessageType, MessageVersion, ParseMentions, Permission,
    ThreadMemberPut, User, UserId,
};
use common::v2::types::media::MediaReference;
use common::v2::types::{MediaId, SERVER_USER_ID};
use http::StatusCode;
use tracing::error;
use uuid::Uuid;
use validator::Validate;

use crate::routes::util::auth::Auth4;
use crate::services::messages::util::MediaRegistry;
use crate::services::messages::{links, markdown};
use crate::types::MediaLinkType;
use crate::{Error, Result, services::messages::ServiceMessages};

struct MessageOperation<'a, S> {
    channel: Channel,
    message_id: MessageId,
    auth: AuthProvider,
    kind: MessageOperationKind,
    stage: S,
    nonce: Option<String>,

    // TODO: remove 'a once im sure i dont need it anymore
    _ph: PhantomData<&'a ()>,
}

enum AuthProvider {
    Auth(Auth4),
    Webhook { user: User },
    Server,
}

impl AuthProvider {
    pub fn user_id(&self) -> Option<UserId> {
        match self {
            Self::Auth(a) => a.user().map(|u| u.id),
            Self::Webhook { user } => Some(user.id),
            Self::Server => None,
        }
    }
}

struct MessageCreateOperation {
    json: MessageCreate,
}

struct MessageEditOperation {
    json: MessagePatch,
    original: Message,
}

enum MessageOperationKind {
    MessageCreate(MessageCreateOperation),
    MessageEdit(MessageEditOperation),
}

struct New {
    header_timestamp: Option<Time>,
}

/// Stage 1: We know the user is allowed to perform this action.
// "is this allowed?"
struct Authorized {
    // content, mentions, etc
    permissions: MessagePermissions,
    created_at: Option<Time>,
    removed_at: Option<Time>,
}

/// Stage 2: The "Point of No Return".
/// Everything is transformed and resources (media) are reserved.
// "what's the final payload?"
struct Prepared {
    permissions: MessagePermissions,
    sanitized: MessageSanitized,
    all_media_ids: MediaRegistry,
    embeds: Vec<common::v2::types::embed::Embed>,
    components: Vec<components::Component<components::Thin>>,
    created_at: Option<Time>,
    removed_at: Option<Time>,
}

/// Stage 3: The data is in the database.
// "what needs to be done now that its been inserted?"
//
// run these after switching to this state:
struct Committed {
    // preflight, media, etc
    /// the fully hydrated message from the database
    message: Message,
    permissions: MessagePermissions,
    sanitized: MessageSanitized,
}

pub(super) struct MessagePermissions {
    allow_external_emoji: bool,
    generate_embeds: bool,
}

struct MessageSanitized {
    content: Option<String>,
    mentions: Mentions,
}

impl<'a, S> MessageOperation<'a, S> {
    pub fn user_id(&self) -> Option<UserId> {
        self.auth.user_id()
    }

    /// get the id of the user who should be credited as the message's author
    pub fn author_id(&self) -> UserId {
        self.auth.user_id().unwrap_or(SERVER_USER_ID)
    }

    pub fn transition<NewS, F: FnOnce(S) -> NewS>(
        self,
        new_stage: F,
    ) -> MessageOperation<'a, NewS> {
        MessageOperation {
            channel: self.channel,
            message_id: self.message_id,
            auth: self.auth,
            kind: self.kind,
            nonce: self.nonce,
            stage: new_stage(self.stage),
            _ph: PhantomData,
        }
    }
}

impl MessageOperationKind {
    pub fn validate(&self) -> Result<()> {
        match &self {
            Self::MessageCreate(o) => o.json.validate()?,
            Self::MessageEdit(o) => o.json.validate()?,
        }
        Ok(())
    }

    /// will the resulting message will have "standard" content? (text, attachments, and/or embeds)
    pub fn will_have_content(&self) -> bool {
        match self {
            Self::MessageCreate(o) => {
                o.json.content.is_some()
                    || !o.json.attachments.is_empty()
                    || !o.json.embeds.is_empty()
            }
            Self::MessageEdit(o) => {
                let patch_has_content = o.json.content.is_some()
                    || o.json.attachments.as_ref().is_some_and(|a| !a.is_empty())
                    || o.json.embeds.as_ref().is_some_and(|a| !a.is_empty());

                if o.json.content.is_none()
                    && o.json.attachments.is_none()
                    && o.json.embeds.is_none()
                {
                    // we aren't setting any content, but the resulting message
                    // could still have stuff if the original mesage had content
                    match &o.original.latest_version.message_type {
                        MessageType::DefaultMarkdown(m) => {
                            m.content.is_some() || !m.attachments.is_empty() || !m.embeds.is_empty()
                        }
                        _ => false,
                    }
                } else {
                    patch_has_content
                }
            }
        }
    }

    /// will the resulting message have components?
    pub fn will_have_components(&self) -> bool {
        // if you don't set any components, treat it like the user's trying to remove components from the message
        match self {
            Self::MessageCreate(o) => o.json.components.is_some(),
            Self::MessageEdit(o) => o.json.components.is_some(),
        }
    }

    pub fn attachment_ids(&self) -> Vec<MediaId> {
        match &self {
            Self::MessageCreate(o) => {
                let mut ids = Vec::new();
                for attachment in &o.json.attachments {
                    let MessageAttachmentCreateType::Media { media, .. } = &attachment.ty;
                    if let Some(media_id) = media.media_id() {
                        ids.push(media_id);
                    }
                }
                ids
            }
            Self::MessageEdit(o) => {
                let mut ids = Vec::new();
                if let Some(attachments) = &o.json.attachments {
                    for attachment in attachments {
                        let MessageAttachmentCreateType::Media { media, .. } = &attachment.ty;
                        if let Some(media_id) = media.media_id() {
                            ids.push(media_id);
                        }
                    }
                } else {
                    match &o.original.latest_version.message_type {
                        MessageType::DefaultMarkdown(m) => {
                            ids.extend(m.attachments.iter().filter_map(|a| match &a.ty {
                                MessageAttachmentType::Media { media } => Some(media.id),
                            }))
                        }
                        _ => {}
                    }
                }
                ids
            }
        }
    }
}

impl<S> MessageOperation<'_, S> {
    pub fn old_message_version(&self) -> Option<&MessageVersion> {
        match &self.kind {
            MessageOperationKind::MessageCreate(_) => None,
            MessageOperationKind::MessageEdit(m) => Some(&m.original.latest_version),
        }
    }
}

impl ServiceMessages {
    /// create a new message
    pub async fn create(
        &self,
        channel_id: ChannelId,
        auth: &Auth4,
        nonce: Option<String>,
        json: MessageCreate,
        header_timestamp: Option<Time>,
    ) -> Result<Message> {
        if let Some(nonce) = nonce {
            // FIXME: this won't work with federation
            let session = auth.ensure_session()?;
            self.idempotency_keys
                .try_get_with(
                    (session.id, nonce.clone()),
                    self.create_inner(channel_id, auth, Some(nonce), json, header_timestamp),
                )
                .await
                .map_err(|err| err.fake_clone())
        } else {
            self.create_inner(channel_id, auth, nonce, json, header_timestamp)
                .await
        }
    }

    /// create a new system message
    pub async fn create_system(
        &self,
        channel_id: ChannelId,
        json: MessageCreate,
    ) -> Result<Message> {
        let srv = self.state.services();
        let channel = srv.channels.get(channel_id, None).await?;

        let op = MessageOperation {
            channel,
            message_id: MessageId::new(),
            auth: AuthProvider::Server,
            kind: MessageOperationKind::MessageCreate(MessageCreateOperation { json }),
            nonce: None,
            stage: New {
                header_timestamp: None,
            },
            _ph: PhantomData,
        };

        let op = self.authorize(op).await?;
        let op = self.prepare(op).await?;
        let op = self.commit(op).await?;
        let op = self.finalize(op).await?;
        Ok(op.stage.message)
    }

    async fn create_inner(
        &self,
        channel_id: ChannelId,
        auth: &Auth4,
        nonce: Option<String>,
        json: MessageCreate,
        header_timestamp: Option<Time>,
    ) -> Result<Message> {
        let srv = self.state.services();
        let channel = srv.channels.get(channel_id, None).await?;

        let op = MessageOperation {
            channel,
            message_id: MessageId::new(),
            auth: AuthProvider::Auth(auth.clone()),
            kind: MessageOperationKind::MessageCreate(MessageCreateOperation { json }),
            nonce,
            stage: New { header_timestamp },
            _ph: PhantomData,
        };

        let op = self.authorize(op).await?;
        let op = self.prepare(op).await?;
        let op = self.commit(op).await?;
        let op = self.finalize(op).await?;
        Ok(op.stage.message)
    }

    /// edit a message
    pub async fn edit(
        &self,
        channel_id: ChannelId,
        message_id: MessageId,
        auth: &Auth4,
        json: MessagePatch,
        header_timestamp: Option<Time>,
    ) -> Result<(StatusCode, Message)> {
        self.edit_inner(channel_id, message_id, auth, json, header_timestamp)
            .await

        // TODO: add nonce support for edits
        // TODO: add if-match support for edits (PATCH routes in general?)
        // if let Some(nonce) = nonce {
        //     self.idempotency_keys
        //         .try_get_with(
        //             (auth.session.id, nonce.clone()),
        //             self.edit_inner(channel_id, message_id, user_id, json, header_timestamp),
        //         )
        //         .await
        //         .map_err(|err| err.fake_clone())
        // } else {
        //     self.edit_inner(channel_id, message_id, user_id, json, header_timestamp)
        //         .await
        // }
    }

    async fn edit_inner(
        &self,
        channel_id: ChannelId,
        message_id: MessageId,
        auth: &Auth4,
        json: MessagePatch,
        header_timestamp: Option<Time>,
    ) -> Result<(StatusCode, Message)> {
        let srv = self.state.services();
        let user_id = auth.user().map(|u| u.id);
        let channel = srv.channels.get(channel_id, user_id).await?;

        let original = self.get(channel_id, message_id, user_id).await?;

        let op = MessageOperation {
            channel,
            message_id,
            auth: AuthProvider::Auth(auth.clone()),
            kind: MessageOperationKind::MessageEdit(MessageEditOperation { json, original }),
            nonce: None,
            stage: New { header_timestamp },
            _ph: PhantomData,
        };

        let op = self.authorize(op).await?;
        let op = self.prepare(op).await?;
        let op = self.commit(op).await?;
        let op = self.finalize(op).await?;

        Ok((StatusCode::OK, op.stage.message))
    }

    pub async fn create_as_webhook(
        &self,
        channel_id: ChannelId,
        webhook_user_id: UserId,
        json: MessageCreate,
    ) -> Result<Message> {
        let srv = self.state.services();
        let channel = srv.channels.get(channel_id, None).await?;
        let user = srv.users.get(webhook_user_id, None).await?;

        let op = MessageOperation {
            channel,
            message_id: MessageId::new(),
            auth: AuthProvider::Webhook { user },
            kind: MessageOperationKind::MessageCreate(MessageCreateOperation { json }),
            nonce: None,
            stage: New {
                header_timestamp: None,
            },
            _ph: PhantomData,
        };

        let op = self.authorize(op).await?;
        let op = self.prepare(op).await?;
        let op = self.commit(op).await?;
        let op = self.finalize(op).await?;
        Ok(op.stage.message)
    }

    pub async fn edit_as_webhook(
        &self,
        channel_id: ChannelId,
        message_id: MessageId,
        webhook_user_id: UserId,
        json: MessagePatch,
    ) -> Result<(StatusCode, Message)> {
        let srv = self.state.services();
        let channel = srv.channels.get(channel_id, None).await?;
        let user = srv.users.get(webhook_user_id, None).await?;

        let original = self.get(channel_id, message_id, None).await?;

        if original.author_id != webhook_user_id {
            return Err(Error::ApiError(ApiError::from_code(
                ErrorCode::UnknownMessage,
            )));
        }

        let op = MessageOperation {
            channel,
            message_id,
            auth: AuthProvider::Webhook { user },
            kind: MessageOperationKind::MessageEdit(MessageEditOperation { json, original }),
            nonce: None,
            stage: New {
                header_timestamp: None,
            },
            _ph: PhantomData,
        };

        let op = self.authorize(op).await?;
        let op = self.prepare(op).await?;
        let op = self.commit(op).await?;
        let op = self.finalize(op).await?;
        Ok((StatusCode::OK, op.stage.message))
    }

    async fn authorize<'a>(
        &'a self,
        mut op: MessageOperation<'a, New>,
    ) -> Result<MessageOperation<'a, Authorized>> {
        // TODO: check idempotency keys
        self.validate_json(&mut op)?;
        let (permissions, created_at) = self.validate_permissions(&mut op).await?;
        let removed_at = self.enforce_automod(&mut op).await?;

        Ok(op.transition(|_| Authorized {
            permissions,
            created_at,
            removed_at,
        }))
    }

    async fn prepare<'a>(
        &self,
        mut op: MessageOperation<'a, Authorized>,
    ) -> Result<MessageOperation<'a, Prepared>> {
        let sanitized = self.process_mentions_and_emojis(&mut op).await?;
        let mut all_media_ids = MediaRegistry::default();
        let embeds = self.process_embeds(&mut op).await?;
        let components = self.process_components(&mut op, &mut all_media_ids).await?;

        match &op.kind {
            MessageOperationKind::MessageCreate(m) => {
                for m in &m.json.attachments {
                    let MessageAttachmentCreateType::Media { media, .. } = &m.ty;
                    all_media_ids.insert_ref(media)?;
                }
                for embed in &m.json.embeds {
                    if let Some(m) = &embed.media {
                        all_media_ids.insert_ref(m)?;
                    }
                    if let Some(m) = &embed.thumbnail {
                        all_media_ids.insert_ref(m)?;
                    }
                    if let Some(m) = &embed.author_avatar {
                        all_media_ids.insert_ref(m)?;
                    }
                }
            }
            MessageOperationKind::MessageEdit(m) => {
                if let Some(atts) = &m.json.attachments {
                    for m in atts {
                        let MessageAttachmentCreateType::Media { media, .. } = &m.ty;
                        all_media_ids.insert_ref(media)?;
                    }
                }
                if let Some(embeds) = &m.json.embeds {
                    for embed in embeds {
                        if let Some(m) = &embed.media {
                            all_media_ids.insert_ref(m)?;
                        }
                        if let Some(m) = &embed.thumbnail {
                            all_media_ids.insert_ref(m)?;
                        }
                        if let Some(m) = &embed.author_avatar {
                            all_media_ids.insert_ref(m)?;
                        }
                    }
                }
            }
        };

        Ok(op.transition(|old| Prepared {
            permissions: old.permissions,
            sanitized,
            all_media_ids,
            embeds,
            components,
            created_at: old.created_at,
            removed_at: old.removed_at,
        }))
    }

    async fn commit<'a>(
        &self,
        mut op: MessageOperation<'a, Prepared>,
    ) -> Result<MessageOperation<'a, Committed>> {
        let author_id = op.author_id();
        let message_id = op.message_id;

        // FIXME: run everything in a transaction
        // let mut txn = self.state.acquire_data().await?;
        // txn.media_link_select(media_id).await?;
        // txn.media_link_insert(media_id, target_id, link_type)
        //     .await?;
        // txn.commit().await?;

        self.validate_media(&op.stage.all_media_ids, message_id, author_id)
            .await?;

        // TODO: skip all of these for ephemeral messages
        let message = self.persist_to_database(&mut op).await?;
        let version_id = *message.latest_version.version_id;
        self.claim_media(&mut op.stage.all_media_ids, message_id, version_id)
            .await?;
        self.update_slowmode_timeout(&mut op).await?;

        Ok(op.transition(|old| Committed {
            permissions: old.permissions,
            message,
            sanitized: old.sanitized,
        }))
    }

    async fn finalize<'a>(
        &'a self,
        mut op: MessageOperation<'a, Committed>,
    ) -> Result<MessageOperation<'a, Committed>> {
        self.state
            .services()
            .channels
            .update_last_message_ids(
                op.channel.id,
                op.message_id,
                op.stage.message.latest_version.version_id,
            )
            .await;

        self.ensure_thread_unarchived(&mut op).await?;
        self.ensure_thread_membership(&mut op).await?;
        self.spawn_unfurler_tasks(&mut op).await?;
        self.spawn_notification_tasks(&mut op).await?;

        let sync = match &op.kind {
            MessageOperationKind::MessageCreate(_) => MessageSync::MessageCreate {
                message: op.stage.message.clone(),
            },
            MessageOperationKind::MessageEdit(_) => MessageSync::MessageUpdate {
                message: op.stage.message.clone(),
            },
        };

        self.state.broadcast_with_nonce(op.nonce.as_deref(), sync)?;

        Ok(op)
    }

    fn validate_json(&self, op: &mut MessageOperation<'_, New>) -> Result<()> {
        op.kind.validate()?;

        // NOTE: do i want to allow or deny switching between content/component messages?
        // currently i allow it freely because why not
        // if i need to enforce it strictly, i can introduce a new message type (DefaultComponents?)
        let has_content = op.kind.will_have_content();
        let has_components = op.kind.will_have_components();
        if has_content && has_components {
            return Err(Error::BadStatic(
                "cannot have both (content, attachments, or embeds) and components on the same message",
            ));
        }

        if !has_content && !has_components {
            return Err(Error::BadStatic(
                "at least one of (content, attachments, or embeds) or components must be defined",
            ));
        }

        Ok(())
    }

    /// Validates message creation/edit permissions for a session-authenticated user.
    /// Returns (allow_external_emoji, generate_embeds, created_at).
    pub(super) async fn validate_session_permissions(
        &self,
        auth: &Auth4,
        channel: &Channel,
        has_attachments: bool,
        has_embeds: bool,
        header_timestamp: Option<Time>,
    ) -> Result<(MessagePermissions, Option<Time>)> {
        let srv = self.state.services();
        let mut data = self.state.data();

        let user = auth.ensure_user()?;

        let mut perms = srv
            .perms
            .for_channel3(Some(user.id), channel.id)
            .await?
            .ensure_view()?;
        perms.needs_unlocked().needs_slowmode_message_bypass();
        perms.needs(if channel.is_thread() {
            Permission::MessageCreateThread
        } else {
            Permission::MessageCreate
        });

        if has_attachments {
            perms.needs(Permission::MessageAttachments);
        }
        if has_embeds {
            perms.needs(Permission::MessageEmbeds);
        }
        perms.check()?;

        let created_at = if let Some(ts) = header_timestamp {
            let owner_id = if let Some(puppet) = &user.puppet {
                (*puppet.owner_id).into()
            } else if user.bot {
                let app = data
                    .application_get(user.id.into_inner().into())
                    .await
                    .map_err(|_| {
                        Error::BadStatic("MemberBridge permission required to override timestamp")
                    })?;
                app.owner_id
            } else {
                return Err(Error::BadStatic(
                    "MemberBridge permission required to override timestamp",
                ));
            };

            srv.perms
                .for_channel3(Some(owner_id), channel.id)
                .await?
                .ensure_view()?
                .needs(Permission::IntegrationsBridge)
                .check()?;

            Some(ts)
        } else {
            None
        };

        Ok((
            MessagePermissions {
                allow_external_emoji: perms.has(Permission::EmojiUseExternal),
                generate_embeds: perms.has(Permission::MessageEmbeds),
            },
            created_at,
        ))
    }

    /// Validates permissions for a message operation (create/edit).
    async fn validate_permissions(
        &self,
        op: &mut MessageOperation<'_, New>,
    ) -> Result<(MessagePermissions, Option<Time>)> {
        // TODO: always allow ephemeral messages

        let _srv = self.state.services();

        // 0. you can only edit your own messages
        // (this *may* change in the future - dubious though)
        // TODO: think about it
        match &op.kind {
            MessageOperationKind::MessageCreate(_) => {}
            MessageOperationKind::MessageEdit(m) => {
                if op.user_id() != Some(m.original.author_id) {
                    // TODO: custom error code for this?
                    return Err(Error::ApiError(ApiError::with_message(
                        ErrorCode::MissingPermissions,
                        "cannot edit other people's messages".to_owned(),
                    )));
                }
            }
        }

        match &op.auth {
            AuthProvider::Auth(auth) => {
                let (has_attachments, has_embeds) = match &op.kind {
                    MessageOperationKind::MessageCreate(o) => {
                        (!o.json.attachments.is_empty(), !o.json.embeds.is_empty())
                    }
                    MessageOperationKind::MessageEdit(o) => (
                        o.json.attachments.as_ref().is_some_and(|a| !a.is_empty()),
                        o.json.embeds.as_ref().is_some_and(|a| !a.is_empty()),
                    ),
                };
                self.validate_session_permissions(
                    auth,
                    &op.channel,
                    has_attachments,
                    has_embeds,
                    op.stage.header_timestamp,
                )
                .await
            }
            AuthProvider::Webhook { .. } => {
                // 1. webhooks bypass permission checks (for now)
                Ok((
                    MessagePermissions {
                        allow_external_emoji: true,
                        generate_embeds: true,
                    },
                    op.stage.header_timestamp,
                ))
            }
            AuthProvider::Server => {
                // 2. system messages bypass permission checks
                Ok((
                    MessagePermissions {
                        allow_external_emoji: true,
                        generate_embeds: true,
                    },
                    op.stage.header_timestamp,
                ))
            }
        }
    }

    async fn enforce_automod(&self, op: &mut MessageOperation<'_, New>) -> Result<Option<Time>> {
        // TODO: skip for ephemeral messages

        let Some(room_id) = op.channel.room_id else {
            return Ok(None);
        };

        let Some(user_id) = op.user_id() else {
            return Ok(None);
        };

        let srv = self.state.services();
        let automod = srv.automod.load(room_id).await?;

        let scan = match &op.kind {
            MessageOperationKind::MessageCreate(o) => automod.scan_message_create(&o.json),
            MessageOperationKind::MessageEdit(o) => {
                automod.scan_message_update(&o.original, &o.json)
            }
        };

        if scan.is_triggered() {
            let removed = srv
                .automod
                .enforce_message_create(room_id, op.channel.id, op.message_id, user_id, &scan)
                .await?;
            if removed {
                return Ok(Some(Time::now_utc()));
            }
        }
        Ok(None)
    }

    async fn process_mentions_and_emojis(
        &self,
        op: &mut MessageOperation<'_, Authorized>,
    ) -> Result<MessageSanitized> {
        let content = match &op.kind {
            MessageOperationKind::MessageCreate(m) => m.json.content.as_deref(),
            MessageOperationKind::MessageEdit(m) => {
                if let Some(v) = &op.old_message_version() {
                    match (
                        m.json.content.as_ref().map(|c| c.as_deref()),
                        &v.message_type,
                    ) {
                        (Some(s), _) => s,
                        (None, MessageType::DefaultMarkdown(m)) => m.content.as_deref(),
                        (None, _) => None,
                    }
                } else {
                    None
                }
            }
        };

        let Some(content) = content else {
            return Ok(MessageSanitized {
                content: None,
                mentions: Mentions::default(),
            });
        };

        // TODO: ignore all mentions for ephemeral messages
        let parse_mentions = match &op.kind {
            MessageOperationKind::MessageCreate(m) => &m.json.mentions,
            MessageOperationKind::MessageEdit(_) => &ParseMentions::default(),
        };

        let mention_ids = markdown::parse(content, parse_mentions);
        let mentions = self
            .fetch_full_mentions_from_ids(mention_ids, op.channel.room_id)
            .await?;

        let final_content = if let Some(room_id) = op.channel.room_id {
            self.enforce_emoji_use_external(
                &mentions,
                room_id,
                op.stage.permissions.allow_external_emoji,
                content,
            )
            .await?
        } else {
            content.to_owned()
        };

        return Ok(MessageSanitized {
            content: Some(final_content),
            mentions,
        });
    }

    // TODO: don't process embeds during create/update
    // insert it into the database raw, and inflate the embeds into full embeds with media on read
    async fn process_embeds(
        &self,
        op: &mut MessageOperation<'_, Authorized>,
    ) -> Result<Vec<common::v2::types::embed::Embed>> {
        let embeds_create = match &op.kind {
            MessageOperationKind::MessageCreate(o) => o.json.embeds.clone(),
            MessageOperationKind::MessageEdit(o) => o.json.embeds.clone().unwrap_or_default(),
        };

        if embeds_create.is_empty() {
            return Ok(vec![]);
        }

        let user_id = op.author_id();
        let mut embed_futs = Vec::new();
        for embed_create in embeds_create {
            embed_futs.push(self.embed_from_create(embed_create, user_id));
        }

        futures_util::future::try_join_all(embed_futs).await
    }

    /// Parse components and collect media IDs into the registry.
    /// Used by both the operation flow and flume_create.
    pub(super) async fn process_components_with_media(
        &self,
        components_input: &Components<components::Create>,
        old_components: Option<&Components<components::Thin>>,
        all_media_ids: &mut MediaRegistry,
    ) -> Result<Components<components::Thin>> {
        let components = components_input
            .clone()
            .parse_thin(old_components, &|m| match m {
                MediaReference::Media { media_id } => Ok(media_id),
                _ => Err(ApiError::from_code(ErrorCode::Unimplemented)),
            })?;

        let mut media_ids = vec![];
        components.collect_media_refs(&mut media_ids);
        all_media_ids.extend(&media_ids);

        Ok(components)
    }

    async fn process_components(
        &self,
        op: &mut MessageOperation<'_, Authorized>,
        all_media_ids: &mut MediaRegistry,
    ) -> Result<Vec<ComponentThin>> {
        let (components_input, old_components) = match &op.kind {
            MessageOperationKind::MessageCreate(o) => (&o.json.components, None),
            MessageOperationKind::MessageEdit(o) => {
                let old = op
                    .old_message_version()
                    .as_ref()
                    .and_then(|v| match &v.message_type {
                        MessageType::DefaultMarkdown(m) => Some(m.components.clone().into_thin()),
                        _ => None,
                    });
                (&o.json.components, old)
            }
        };

        let Some(components_input) = components_input else {
            return Ok(vec![]);
        };

        let components = self
            .process_components_with_media(components_input, old_components.as_ref(), all_media_ids)
            .await?;

        Ok(components.inner)
    }

    async fn persist_to_database(
        &self,
        op: &mut MessageOperation<'_, Prepared>,
    ) -> Result<Message> {
        let mut data = self.state.data();
        let user_id = op.author_id();

        let attachment_ids = op.kind.attachment_ids();

        // NOTE: logic for constructing the MessageType payload should be shared
        let payload =
            MessageType::DefaultMarkdown(common::v1::types::message::MessageDefaultMarkdown {
                content: op.stage.sanitized.content.clone(),
                metadata: match &op.kind {
                    MessageOperationKind::MessageCreate(o) => o.json.metadata.clone(),
                    MessageOperationKind::MessageEdit(o) => {
                        o.json.metadata.clone().unwrap_or_else(|| {
                            op.old_message_version()
                                .as_ref()
                                .and_then(|v| match &v.message_type {
                                    MessageType::DefaultMarkdown(m) => m.metadata.clone(),
                                    _ => None,
                                })
                        })
                    }
                },
                reply_id: match &op.kind {
                    MessageOperationKind::MessageCreate(o) => o.json.reply_id,
                    MessageOperationKind::MessageEdit(o) => o.json.reply_id.unwrap_or_else(|| {
                        op.old_message_version()
                            .as_ref()
                            .and_then(|v| match &v.message_type {
                                MessageType::DefaultMarkdown(m) => m.reply_id,
                                _ => None,
                            })
                    }),
                },

                // these fields are handled in DbMessageCreate and ignored here
                attachments: vec![],
                embeds: vec![],
                components: Components::default(),
            });

        match &op.kind {
            MessageOperationKind::MessageCreate(_) => {
                // TODO: handle interactions
                data.message_create(crate::types::DbMessageCreate {
                    id: Some(op.message_id),
                    channel_id: op.channel.id,
                    attachment_ids,
                    author_id: user_id,
                    embeds: op.stage.embeds.clone(),
                    components: op.stage.components.clone(),
                    message_type: payload,
                    created_at: op.stage.created_at.map(|t| t.into()),
                    removed_at: op.stage.removed_at.map(|t| t.into()),
                    flume: None,
                    mentions: op.stage.sanitized.mentions.clone(),
                    interaction: None,
                    ephemeral: false,
                })
                .await?;
            }
            MessageOperationKind::MessageEdit(_) => {
                data.message_update(
                    op.channel.id,
                    op.message_id,
                    crate::types::DbMessageUpdate {
                        attachment_ids,
                        author_id: user_id,
                        embeds: op.stage.embeds.clone(),
                        components: op.stage.components.clone(),
                        message_type: payload,
                        created_at: op.stage.created_at.map(|t| t.into()),
                        mentions: op.stage.sanitized.mentions.clone(),
                    },
                )
                .await?;
            }
        }

        self.get(op.channel.id, op.message_id, None).await
    }

    /// Validate media ownership and reuse (no side effects).
    /// Called before persisting the message so failures don't leave ghost records.
    pub(super) async fn validate_media(
        &self,
        all_media_ids: &MediaRegistry,
        message_id: MessageId,
        author_id: UserId,
    ) -> Result<()> {
        all_media_ids.check()?;

        let mut data = self.state.data();
        for &id in &all_media_ids.known {
            // PERF: this should probably be batched
            let media = data.media_select(id).await?;

            // 1. ownership check: user must own the media they are trying to use
            if media.user_id != Some(author_id) {
                return Err(Error::MissingPermissions);
            }

            // 2. reuse check: media cannot be linked to a different message
            let existing = data.media_link_select(id).await?;
            let already_linked_to_this = existing.iter().any(|l| {
                l.link_type == MediaLinkType::Message && l.target_id == message_id.into_inner()
            });

            if !already_linked_to_this && !existing.is_empty() {
                return Err(Error::ApiError(ApiError::from_code(
                    ErrorCode::MediaAlreadyUsed,
                )));
            }
        }

        Ok(())
    }

    /// Insert media links for a message (assumes validation already passed).
    /// Used by both the operation flow and flume_create.
    pub(super) async fn claim_media(
        &self,
        all_media_ids: &mut MediaRegistry,
        message_id: MessageId,
        version_id: Uuid,
    ) -> Result<()> {
        let mut data = self.state.data();
        for &id in &all_media_ids.known {
            // 3. insert media links
            data.media_link_insert(id, message_id.into_inner(), MediaLinkType::Message)
                .await?;
            data.media_link_insert(id, version_id, MediaLinkType::MessageVersion)
                .await?;
        }

        Ok(())
    }

    async fn update_slowmode_timeout(&self, op: &mut MessageOperation<'_, Prepared>) -> Result<()> {
        let Some(user_id) = op.user_id() else {
            return Ok(());
        };

        if let Some(slowmode_delay) = op.channel.slowmode_message {
            let mut data = self.state.data();
            let next_message_time =
                Time::now_utc() + std::time::Duration::from_secs(slowmode_delay);
            data.channel_set_message_slowmode_expire_at(op.channel.id, user_id, next_message_time)
                .await?;
        }

        Ok(())
    }

    /// if the channel is a thread and it is archived, unarchive it
    async fn ensure_thread_unarchived(
        &self,
        op: &mut MessageOperation<'_, Committed>,
    ) -> Result<()> {
        // TODO: skip if message is ephemeral

        let srv = self.state.services();

        // TODO: unarchive thread for system, webhooks
        let AuthProvider::Auth(auth) = &op.auth else {
            return Ok(());
        };

        if op.channel.is_archived() {
            srv.channels
                .update(
                    auth,
                    op.channel.id,
                    ChannelPatch {
                        archived: Some(false),
                        ..Default::default()
                    },
                )
                .await?;
        }

        Ok(())
    }

    /// if the channel is a thread and the message author is not a member, add the author to the thread
    async fn ensure_thread_membership(
        &self,
        op: &mut MessageOperation<'_, Committed>,
    ) -> Result<()> {
        // TODO: skip if message is ephemeral

        let mut data = self.state.data();
        let srv = self.state.services();

        if !op.channel.is_thread() {
            return Ok(());
        }

        let Some(user_id) = op.user_id() else {
            return Ok(());
        };
        let thread_id = op.channel.id;

        if data.thread_member_get(thread_id, user_id).await.is_err() {
            data.thread_member_put(thread_id, user_id, ThreadMemberPut::default())
                .await?;
            srv.channels.invalidate(thread_id).await; // NOTE: do i need this? presumably only member count is dirty
            let thread_member = data.thread_member_get(thread_id, user_id).await?;
            let msg = MessageSync::ThreadMemberUpsert {
                room_id: op.channel.room_id,
                thread_id,
                added: vec![thread_member],
                removed: vec![],
            };
            self.state
                .broadcast_channel(thread_id, user_id, msg)
                .await?;
        }

        Ok(())
    }

    async fn spawn_unfurler_tasks(&self, op: &mut MessageOperation<'_, Committed>) -> Result<()> {
        if !op.stage.permissions.generate_embeds {
            return Ok(());
        }

        let Some(content) = op.stage.sanitized.content.clone() else {
            return Ok(());
        };

        // PERF: use less cloning
        let message = op.stage.message.clone();

        let s = self.state.clone();
        let srv = s.services();

        tokio::spawn(async move {
            for url in links::extract_links(&content) {
                if let Err(e) = srv
                    .embed
                    .queue(
                        Some(crate::types::MessageRef {
                            thread_id: message.channel_id,
                            message_id: message.id,
                            version_id: message.latest_version.version_id,
                        }),
                        message.latest_version.author_id,
                        url,
                    )
                    .await
                {
                    error!("Failed to queue embed generation: {:?}", e);
                }
            }
        });

        Ok(())
    }

    async fn spawn_notification_tasks(
        &self,
        op: &mut MessageOperation<'_, Committed>,
    ) -> Result<()> {
        let srv = self.state.services();
        let channel = op.channel.clone();
        let message = op.stage.message.clone();
        tokio::spawn(async move {
            srv.notifications.process_message(channel, message).await;
        });
        Ok(())
    }
}

// TODO: port NotificationProcessor to ServiceNotifications
// TODO: remove commented out NotificationProcessor code
// struct NotificationProcessor {
//     state: Arc<ServerStateInner>,
//     channel: Arc<Channel>,
//     message: Message,
//     // in the future, i'll probably wrap a lot of stuff in `Arc`s
//     // but channel: Arc<Channel> is kinda useless for now
// }

// struct NotificationTargets {
//     notify: HashSet<UserId>,
//     add_to_thread: HashSet<UserId>,
// }

// impl NotificationProcessor {
//     async fn process(self) {
//         let targets = match self.get_mentioned_users().await {
//             Ok(t) => t,
//             Err(err) => {
//                 warn!("failed to get mention targets, skipping: {err:?}");
//                 return;
//             }
//         };

//         let mut thread_members = vec![];
//         for target in targets.notify {
//             match self.process_mention(target).await {
//                 Ok(Some(thread_member)) => thread_members.push(thread_member),
//                 Ok(None) => {}
//                 Err(err) => warn!("failed to process mention, skipping: {err:?}"),
//             }
//         }

//         let srv = self.state.services();

//         if !thread_members.is_empty() {
//             let thread_id = self.channel.id;

//             // NOTE: do i need this? presumably only member count is dirty (from thread members)
//             srv.channels.invalidate(thread_id).await;

//             let msg = MessageSync::ThreadMemberUpsert {
//                 room_id: self.channel.room_id,
//                 thread_id,
//                 added: thread_members,
//                 removed: vec![],
//             };

//             if let Err(err) = self.state.broadcast(msg) {
//                 warn!("failed to broadcast message: {err:?}");
//             };
//         }
//     }

//     async fn get_mentioned_users(&self) -> Result<NotificationTargets> {
//         let mut users_to_notify = HashSet::new();
//         let is_thread = self.channel.ty.is_thread();
//         let mentions = &self.message.latest_version.mentions;
//         let mut data = self.state.data();
//         let author_id = self.message.author_id;
//         let channel_id = self.channel.id;

//         // 1. collect user mentions
//         for u in &mentions.users {
//             if u.id != author_id {
//                 users_to_notify.insert(u.id);
//             }
//         }

//         // 2. collect role mentions
//         for r in &mentions.roles {
//             if let Ok(members) = data.role_member_list(r.id, Default::default()).await {
//                 // Check if we should auto-add role members to the thread
//                 let should_add_to_thread = is_thread
//                     && members.items.len() < crate::consts::MAX_ROLE_MENTION_MEMBERS_ADD as usize;

//                 for member in members.items {
//                     if member.user_id != author_id {
//                         users_to_notify.insert(member.user_id);

//                         // Specific logic for bulk-adding role members to threads
//                         if should_add_to_thread {
//                             let _ = data
//                                 .thread_member_put(channel_id, member.user_id, Default::default())
//                                 .await;
//                         }
//                     }
//                 }
//             }
//         }

//         // 3. collect @everyone mentions
//         if mentions.everyone {
//             let everyone_ids = if is_thread {
//                 data.thread_member_list_all(channel_id)
//                     .await
//                     .ok()
//                     .map(|m| m.into_iter().map(|u| u.user_id).collect::<Vec<_>>())
//             } else if let Some(r_id) = self.channel.room_id {
//                 // use room cache for this? it also might be a good idea to see if i can avoid creating a vec of every user id in a room on an everyone mention.
//                 data.room_member_list_all(r_id)
//                     .await
//                     .ok()
//                     .map(|m| m.into_iter().map(|u| u.user_id).collect::<Vec<_>>())
//             } else {
//                 None
//             };

//             if let Some(ids) = everyone_ids {
//                 for id in ids {
//                     if id != author_id {
//                         users_to_notify.insert(id);
//                     }
//                 }
//             }
//         }

//         Ok(NotificationTargets {
//             notify: users_to_notify.clone(),
//             add_to_thread: users_to_notify.clone(),
//         })
//     }

//     async fn process_mention(&self, user_id: UserId) -> Result<Option<ThreadMember>> {
//         let mut data = self.state.data();
//         let srv = self.state.services();
//         let is_thread = self.channel.ty.is_thread();
//         let channel_id = self.channel.id;

//         // 1. ensure thread membership
//         let thread_member = if is_thread {
//             if data.thread_member_get(channel_id, user_id).await.is_err() {
//                 // PERF: either make thread_member_put return ThreadMember or make it take a full struct
//                 data.thread_member_put(channel_id, user_id, ThreadMemberPut::default())
//                     .await?;
//                 let thread_member = data.thread_member_get(channel_id, user_id).await?;
//                 Some(thread_member)
//             } else {
//                 None
//             }
//         } else {
//             None
//         };

//         // 2. increment unread count
//         // PERF: this should probably be done in bulk
//         data.unread_increment_mentions(
//             user_id,
//             self.channel.id,
//             self.message.id,
//             self.message.latest_version.version_id,
//             1,
//         )
//         .await?;

//         // 3. notifiation action calculation
//         let notification = Notification {
//             id: NotificationId::new(),
//             ty: NotificationType::Message {
//                 room_id: self.channel.room_id,
//                 channel_id: self.channel.id,
//                 message_id: self.message.id,
//                 user_id: self.message.author_id,
//                 // FIXME: populate these fields
//                 mention_user: false,
//                 mention_everyone: false,
//                 mention_role: false,
//                 reply: false,
//             },
//             added_at: Time::now_utc(),
//             read_at: None,
//             note: None,
//         };

//         let action = srv
//             .notifications
//             .calculate_actions(user_id, &notification)
//             .await
//             .unwrap_or(NotificationAction::Skip);

//         if action.should_add_to_inbox() {
//             data.notification_add(user_id, notification).await?;
//         }

//         Ok(thread_member)
//     }
// }
