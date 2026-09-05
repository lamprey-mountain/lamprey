use common::{
    v1::types::{
        Channel, EmbedCreate, Message, MessageAttachmentCreate, MessageCreate, MessageInteraction,
        MessagePatch, MessageType, Permission, User, util::Time,
    },
    v2::types::{AUTOMOD_USER_ID, ChannelId, MessageId, SERVER_USER_ID, UserId},
};
use kerosene_core::types::permission::requirements::Requirements;
use validator::Validate;

use crate::{
    prelude::*,
    services::{automod::AutomodContext, messages::ServiceMessages},
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

        // <A: Auth5> auth: &mut A,
        // TODO: somehow enforce requirements?
        // srv.perms.enforce(re, auth).await?

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
        // let sanitized = self.process_mentions_and_emojis(&mut op).await?;
        // let embeds = self.process_embeds(&mut op).await?;
        // let components = self.process_components(&mut op, &mut all_media_ids).await?;
        // collect media ids

        // 3. commit
        // self.validate_media(&op.stage.all_media_ids, message_id, author_id)
        //     .await?;
        // // TODO: skip all of these for ephemeral messages
        // let message = self.persist_to_database(&mut op).await?;
        // let version_id = *message.latest_version.version_id;
        // self.claim_media(&mut op.stage.all_media_ids, message_id, version_id)
        //     .await?;
        // self.update_slowmode_timeout(&mut op).await?;

        // 4. finalize
        // .update_last_message_ids(
        // self.ensure_thread_unarchived(&mut op).await?;
        // self.ensure_thread_membership(&mut op).await?;
        // self.spawn_unfurler_tasks(&mut op).await?;
        // self.spawn_notification_tasks(&mut op).await?;
        // broadcast sync event

        todo!()
    }

    pub async fn edit2(&self, edit: Edit) -> Result<Message> {
        let srv = self.globals.services();
        todo!()
    }
}
