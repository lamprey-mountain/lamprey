use common::{
    v1::types::{
        Channel, MessageSync, PermissionBits, PermissionOverwrite, Role, User, util::Time,
    },
    v2::types::{ChannelId, RoleId, UserId},
};
use im::HashMap as ImMap;
use tracing::warn;

use crate::prelude::*;

#[derive(Debug, Clone)]
pub enum ChannelSnapshot {
    Available(Arc<LoadedChannel>),
    Loading,
    NotFound,
    Deleted,
    Backlogged,
    // etc...
}

#[derive(Clone, Debug)]
pub struct LoadedChannel {
    inner: Arc<Channel>,
    // PERF: unsure how ImMap compares to HashMap here
    overwrites_roles: ImMap<RoleId, PermSet>,
    overwrites_users: ImMap<UserId, PermSet>,
    members: ChannelMembers,
}

// TODO: move this to a common crate
/// a set of allowed and denied permissions
#[derive(Debug, Clone)]
pub struct PermSet {
    pub allow: PermissionBits,
    pub deny: PermissionBits,
}

impl From<&Role> for PermSet {
    fn from(value: &Role) -> Self {
        Self {
            allow: value.allow.as_slice().into(),
            deny: value.deny.as_slice().into(),
        }
    }
}

impl From<&PermissionOverwrite> for PermSet {
    fn from(value: &PermissionOverwrite) -> Self {
        Self {
            allow: value.allow.as_slice().into(),
            deny: value.deny.as_slice().into(),
        }
    }
}

/// the members for a channel
#[derive(Debug, Clone)]
pub enum ChannelMembers {
    /// all members are loaded
    Loaded {
        members: ImMap<UserId, ChannelMember>,
    },

    /// members are currently loading
    Loading,

    /// members dont exist for this channel type
    Unsupported,
}

impl ChannelMembers {
    pub fn is_loaded(&self) -> bool {
        match self {
            Self::Loaded { .. } => true,
            Self::Loading => false,
            Self::Unsupported => true, // NOTE: unsure if this is correct? i dont want code to hang if channel members will never load.
        }
    }

    pub fn is_supported(&self) -> bool {
        match self {
            Self::Loaded { .. } => true,
            Self::Loading => true, // NOTE: presumably if this channel doesnt have members we would never load it
            Self::Unsupported => false,
        }
    }

    pub fn get(&self, user_id: UserId) -> Option<&ChannelMember> {
        match self {
            Self::Loaded { members } => members.get(&user_id),
            Self::Loading | Self::Unsupported => None,
        }
    }

    pub fn insert(&mut self, user_id: UserId, member: ChannelMember) {
        match self {
            ChannelMembers::Loaded { members } => {
                members.insert(user_id, member);
            }
            ChannelMembers::Loading => {
                warn!("tried to insert() a member into ChannelMembers::Loading");
            }
            ChannelMembers::Unsupported => {
                warn!("tried to insert() a member into ChannelMembers::Unsupported");
            }
        };
    }

    pub fn remove(&mut self, user_id: UserId) -> Option<ChannelMember> {
        match self {
            ChannelMembers::Loaded { members } => members.remove(&user_id),
            ChannelMembers::Loading => {
                warn!("tried to remove() a member into ChannelMembers::Loading");
                None
            }
            ChannelMembers::Unsupported => {
                warn!("tried to remove() a member into ChannelMembers::Unsupported");
                None
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct ChannelMember {
    user: Arc<User>,
    joined_at: Time,
    // NOTE: maybe store room member here too?
}

impl ChannelSnapshot {
    // TODO: copy RoomSnapshot from kerosene-services/src/services/rooms/types.rs?
}

impl LoadedChannel {
    pub fn apply(self, msg: &MessageSync) -> Self {
        match msg {
            MessageSync::ChannelUpdate { channel } if channel.id == self.inner.id => {
                todo!()
            }
            // TODO: handle ThreadMemberUpsert
            // TODO: handle UserUpdate
            _ => {}
        }

        todo!()
    }
}

pub struct ChannelActor {
    globals: Globals,
    channel_id: ChannelId,
    // snapshot: Arc<ChannelSnapshot>,
    // snapshot_tx: watch::Sender<Arc<ChannelSnapshot>>,
    span: tracing::Span,
}

// #[kameo::messages]
impl ChannelActor {
    // TODO: impl these
    // fn get_snapshot
    // fn ensure_members
    // fn sync_message
}

/// a handle for interacting with a channel actor
pub struct ChannelHandle {
    channel_id: ChannelId,
    // actor_ref: ActorRef<ChannelActor>,
    // snapshot_rx: watch::Receiver<Arc<ChannelSnapshot>>,
}

pub struct ChannelWeak {
    // rooms own strong references to channels
    // all other channels are cached and evicted normally
}

impl ChannelWeak {
    pub fn upgrade(&self) -> Option<ChannelHandle> {
        todo!()
    }
}

impl ChannelHandle {
    pub fn channel_id(&self) -> ChannelId {
        self.channel_id
    }

    // what goes here?

    /// wait until the channel has successfully loaded
    ///
    /// `with_members` will wait until channel members are loaded
    pub async fn ready(&self, with_members: bool) -> Result<Arc<LoadedChannel>> {
        todo!()
    }

    /// get the current channel snapshot
    pub fn snapshot(&self) -> Arc<ChannelSnapshot> {
        todo!()
    }

    /// get the current channel data
    pub fn data(&self) -> Result<Arc<LoadedChannel>> {
        todo!()
    }

    // pub fn subscribe(&self) -> mpsc::Receiver<Arc<ChannelEvent>> {}

    // pub fn reload(&self) {
    //     todo!()
    // }

    // pub fn unload(&self) {
    //     todo!()
    // }

    // /// create a subscription to a member list
    // pub fn member_list(&self, conn_id: ConnectionId) -> MemberList {
    //     todo!()
    // }

    // pub(super) fn handle_sync(&self, sync: MessageSync) {
    //     todo!()
    // }
}

// pub struct ServiceChannels {
//     globals: Globals,
//     idempotency_keys: Cache<String, ChannelHandle>,
//     cache: Cache<ChannelId, ChannelHandle>,
//     // cache: Cache<ChannelId, Arc<Channel>>,
//     // cache_private: Cache<(ChannelId, UserId), DbChannelPrivate>,
//     // cache_recipients: Cache<ChannelId, Vec<UserId>>,
//     // // PERF: remove expired typing entries
//     // typing: Cache<(ChannelId, UserId), OffsetDateTime>,
// }

// impl ServiceChannels {
//     /// get a handle to a channel
//     pub fn load(&self, channel_id: ChannelId) -> ChannelHandle {
//         todo!()
//     }

//     // unload, reload
// }
