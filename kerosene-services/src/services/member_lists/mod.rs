//! Service for managing member lists

use common::v2::types::{ConnectionId, RoomId};
use dashmap::DashMap;

use crate::prelude::*;
use crate::services::member_lists::actor::MemberListHandle;
use crate::services::member_lists::{
    util::{MemberListKey, MemberListKey1},
    visibility::ListVisibility,
};
use crate::services::rooms::actor::MemberListSubscribeMsg;
use crate::services::rooms::{RoomActor, RoomHandle};

pub mod actor;
pub mod syncer;
pub mod util; // TODO: deprecate and remove; move any useful types to common/core
pub mod visibility;

/// Service for managing member lists
pub struct ServiceMemberLists {
    globals: Globals,

    // TODO: use this
    lists: DashMap<ListKey, MemberListHandle>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ListKey {
    room_id: RoomId,
    visibility: visibility::ListVisibility,
}

impl ServiceMemberLists {
    /// Create a new member lists service
    pub fn new(globals: Globals) -> Self {
        Self {
            globals,
            lists: DashMap::new(),
        }
    }

    /// Lookup a member list key from an API key
    async fn lookup_member_key(&self, key1: MemberListKey1) -> Result<MemberListKey> {
        let srv = self.globals.services();
        match key1 {
            MemberListKey1::Room(room_id) => Ok(MemberListKey::Room(room_id)),
            MemberListKey1::RoomChannel(room_id, channel_id) => {
                let chan = srv.channels.get(channel_id, None).await?;
                if chan.is_thread() && chan.ty.member_list_uses_thread_members() {
                    return Ok(MemberListKey::RoomThread(
                        room_id,
                        ListVisibility::default(),
                        channel_id,
                    ));
                }
                let overwrites = srv.channels.fetch_overwrite_ancestors(channel_id).await?;
                let visibility = ListVisibility::from_overwrites(room_id, overwrites);
                Ok(MemberListKey::RoomChannel(room_id, visibility))
            }
            MemberListKey1::DmChannel(channel_id) => Ok(MemberListKey::Dm(channel_id)),
        }
    }

    /// Ensure a member list exists and return its handle
    async fn ensure(&self, key: MemberListKey) -> Result<Arc<MemberListHandle>> {
        let room_id = key
            .room_id()
            .ok_or(Error::BadStatic("DM member lists not yet sharded"))?;

        let srv = self.globals.services();
        let room_handle = srv.rooms.load(room_id);

        // TODO: don't access actor_ref directly
        // Try to send the subscribe command; if it fails, the actor is dead
        // Evict the dead actor and retry once
        let result = room_handle
            .actor_ref
            .ask(MemberListSubscribeMsg { key: key.clone() })
            .send()
            .await;

        let actual_tx = match result {
            Ok(tx) => tx,
            _ => {
                // Actor is dead or failed, evict it
                self.globals.services().rooms.unload_cache(room_id).await;

                let room_handle = srv.rooms.load(room_id);
                room_handle
                    .actor_ref
                    .ask(MemberListSubscribeMsg { key: key.clone() })
                    .send()
                    .await
                    .map_err(|_| {
                        Error::Internal("failed to subscribe to member list".to_string())
                    })?
            }
        };

        Ok(Arc::new(MemberListHandle {
            actor_ref: room_handle.actor_ref.clone(),
            key,
            events_tx: actual_tx,
        }))
    }

    // TODO(?): replace fn ensure() with this?
    // pub fn ensure_handle(
    //     &self,
    //     room_id: RoomId,
    //     channel_id: Option<ChannelId>,
    // ) -> actor::ListHandle {
    //     todo!()
    // }

    /// Create a new syncer for a connection
    pub fn create_syncer(&self, connection_id: ConnectionId) -> syncer::MemberListSyncer {
        syncer::MemberListSyncer::new(self.globals.clone(), connection_id)
    }

    // TODO: remove this
    /// Start background tasks for the service
    pub fn start_background_tasks(&self) {
        // No longer needed as RoomActor handles its own events
    }
}
