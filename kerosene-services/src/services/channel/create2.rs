use common::{
    v1::types::{Channel, ChannelCreate, ChannelPatch, util::Time},
    v2::types::{ChannelId, RoomId, SessionId, UserId},
};
use lamprey_backend_data_postgres::DbChannelCreate;
use validator::Validate;

use crate::{
    prelude::*,
    services::{channel::ServiceChannels, rooms::LoadedRoom},
};

// TODO: impl and use this

/// A request to create a new channel.
#[derive(Debug)]
pub struct Create {
    channel_id: ChannelId,
    room_id: Option<RoomId>,
    user_id: UserId,
    session_id: Option<SessionId>,
    payload: CreateType,
    nonce: Option<String>,
    timestamp: Option<Time>,
}

/// What kind of channel are we creating?
#[derive(Debug)]
pub enum CreateType {
    Default(Box<ChannelCreate>),
    // TODO(?): more dedicated CreateTypes?
    // Room(RoomId, Box<ChannelCreate>),
    // room channel
    // dm channel
    // thread from message
    // thread in text channel
    // thread in forum channel
}

impl CreateType {
    pub fn validate(&self) -> Result<()> {
        match self {
            CreateType::Default(c) => c.validate()?,
        }

        Ok(())
    }

    pub fn parent_id(&self) -> Option<ChannelId> {
        match self {
            CreateType::Default(c) => c.parent_id,
        }
    }
}

impl Create {
    /// create a new channel in a room
    pub fn new_room<C: Into<Box<ChannelCreate>>>(
        body: C,
        room_id: RoomId,
        user_id: UserId,
    ) -> Self {
        Self::new(CreateType::Default(body.into()), room_id, user_id)
    }

    fn new(payload: CreateType, room_id: RoomId, user_id: UserId) -> Self {
        Self {
            channel_id: ChannelId::new(),
            room_id: Some(room_id),
            user_id,
            session_id: None,
            payload,
            nonce: None,
            timestamp: None,
        }
    }

    /// explicitly set an id for this channel
    pub fn id(mut self, id: ChannelId) -> Self {
        self.channel_id = id;
        self
    }

    /// set the session id
    pub fn session(mut self, session_id: Option<SessionId>) -> Self {
        self.session_id = session_id;
        self
    }

    /// set the nonce (idempotency-key)
    ///
    /// session id must also be set to deduplicate/coalesce requests
    pub fn nonce(mut self, nonce: Option<String>) -> Self {
        self.nonce = nonce;
        self
    }

    /// override the `created_at` timestamp for the message
    pub fn timestamp(mut self, timestamp: Option<Time>) -> Self {
        self.timestamp = timestamp;
        self
    }
}

pub struct Draft<'a> {
    chan: Channel,
    parent: Option<&'a Channel>,
    room: Option<&'a LoadedRoom>,
}

impl Draft<'_> {
    pub fn from_create(c: ChannelCreate) -> Self {
        todo!()
    }

    pub fn from_update(c: Channel, update: ChannelPatch) -> Self {
        // patch.apply(chan_old);
        // archived_at needs special handling
        todo!()
    }

    pub fn validate(&self) -> Result<()> {
        let c = &self.chan;

        // general validation
        c.validate()?;

        // channel type specific validation
        if c.bitrate.is_some() {
            c.ensure_has_voice()?;
        }
        if c.user_limit.is_some() {
            c.ensure_has_voice()?;
        }
        if c.url.is_some() {
            c.ensure_has_url()?;
        }
        if c.default_auto_archive_duration.is_some() {
            c.ensure_has_threads()?;
        }
        if c.auto_archive_duration.is_some() {
            c.ensure_is_thread()?;
        }
        if c.slowmode_thread.is_some() {
            c.ensure_has_threads()?;
        }
        if c.slowmode_message.is_some() {
            c.ensure_has_text()?;
        }
        if c.default_slowmode_message.is_some() {
            c.ensure_has_threads()?;
        }
        if c.icon.is_some() {
            c.ensure_has_icon()?;
        }

        Ok(())
    }

    pub fn to_db_create(&self) -> DbChannelCreate {
        todo!()
    }

    // NOTE: there isn't any special database struct for channel patches
    pub fn to_db_update(&self) -> ChannelPatch {
        todo!()
    }
}

impl ServiceChannels {
    // TODO: also impl fn create3 similarly to messages

    pub async fn create2(&self, create: Create) -> Result<Arc<Channel>> {
        if let Some(nonce) = create.nonce.clone() {
            self.idempotency_keys
                .try_get_with(nonce, Box::pin(self.create2_inner(create)))
                .await
                .map_err(|err| err.fake_clone())
        } else {
            Box::pin(self.create2_inner(create)).await
        }
    }

    async fn create2_inner(&self, create: Create) -> Result<Arc<Channel>> {
        let srv = self.globals.services();

        // 1. validate/authorize
        create.payload.validate()?;
        // fetch parent channel
        // enforce permission checks
        // do other validation that requires room/parent/etc
        // thread slowmode
        // enforce channel components are valid for channel type
        // validate and check perms for tags
        // scan with automod

        // 2. prepare
        // the line between 1 and 2 is kind of blurry?

        // 3. commit
        // let mut txn = self.globals.begin().await?;
        // txn.channel_create
        // MediaLinker
        // room_template_mark_dirty
        // permission_overwrite_upsert
        // thread_member_put
        // txn.commit().await?;

        // 4. finalize
        // dispatch syncs (ChannelCreate, ThreadCreate(?), ThreadMemberUpsert(?))
        // send starter message (MessageCreate)
        // send thread created message (MessageCreate)

        todo!()
    }
}
