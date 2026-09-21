use common::{
    v1::types::{Channel, ChannelCreate, util::Time},
    v2::types::{ChannelId, RoomId, SessionId, UserId},
};
use validator::Validate;

use crate::{prelude::*, services::channel::ServiceChannels};

// TODO: impl and use this

// TODO: pub struct ChannelHandle;

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

impl ServiceChannels {
    pub async fn create2(&self, create: Create) -> Result<Arc<Channel>> {
        // if let (Some(session_id), Some(nonce)) = (create.session_id, create.nonce.clone()) {
        //     self.idempotency_keys
        //         .try_get_with((session_id, nonce), self.create2_inner(create))
        //         .await
        //         .map_err(|err| err.fake_clone())
        // } else {
        //     self.create2_inner(create).await
        // }
        todo!()
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
