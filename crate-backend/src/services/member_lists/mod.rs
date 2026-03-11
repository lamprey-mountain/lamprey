//! Service for managing member lists

use std::sync::Arc;

use tokio::sync::{broadcast, mpsc};

use crate::services::rooms::{RoomActor, RoomCommand, RoomHandle};
use crate::{
    services::member_lists::{
        actor::{MemberListCommand, MemberListEvent},
        util::{MemberListKey, MemberListKey1},
        visibility::MemberListVisibility,
    },
    Result, ServerStateInner,
};

pub mod actor;
pub mod syncer;
pub mod util;
pub mod visibility;

/// Service for managing member lists
pub struct ServiceMemberLists {
    s: Arc<ServerStateInner>,
}

impl ServiceMemberLists {
    /// Create a new member lists service
    pub fn new(state: Arc<ServerStateInner>) -> Self {
        Self { s: state }
    }

    /// Lookup a member list key from an API key
    pub async fn lookup_member_key(&self, key1: MemberListKey1) -> Result<MemberListKey> {
        let srv = self.s.services();
        match key1 {
            MemberListKey1::Room(room_id) => Ok(MemberListKey::Room(room_id)),
            MemberListKey1::RoomChannel(room_id, channel_id) => {
                let chan = srv.channels.get(channel_id, None).await?;
                if chan.is_thread() && chan.ty.member_list_uses_thread_members() {
                    return Ok(MemberListKey::RoomThread(
                        room_id,
                        MemberListVisibility::default(),
                        channel_id,
                    ));
                }
                let overwrites = srv.channels.fetch_overwrite_ancestors(channel_id).await?;
                let visibility = MemberListVisibility::from_overwrites(room_id, overwrites);
                Ok(MemberListKey::RoomChannel(room_id, visibility))
            }
            MemberListKey1::DmChannel(channel_id) => Ok(MemberListKey::Dm(channel_id)),
        }
    }

    /// Ensure a member list exists and return its handle
    pub async fn ensure(&self, key: MemberListKey) -> Result<Arc<MemberListHandle>> {
        let room_id = key
            .room_id()
            .ok_or(crate::Error::BadStatic("DM member lists not yet sharded"))?;

        let room_handle = self
            .s
            .services()
            .rooms
            .actors
            .try_get_with(room_id, async {
                Ok::<RoomHandle, crate::Error>(RoomActor::spawn(room_id, self.s.clone()))
            })
            .await
            .map_err(|e| e.fake_clone())?;

        let (events_tx, _) = broadcast::channel(100);
        room_handle
            .tx
            .send(RoomCommand::MemberListSubscribe(
                key.clone(),
                events_tx.clone(),
            ))
            .await
            .map_err(|_| {
                crate::Error::Internal("failed to subscribe to member list".to_string())
            })?;

        Ok(Arc::new(MemberListHandle {
            room_tx: room_handle.tx.clone(),
            key,
            events_tx,
        }))
    }

    /// Create a new syncer for a connection
    pub fn create_syncer(&self, conn_id: uuid::Uuid) -> syncer::MemberListSyncer {
        syncer::MemberListSyncer::new(self.s.clone(), conn_id)
    }

    /// Start background tasks for the service
    pub fn start_background_tasks(&self) {
        // No longer needed as RoomActor handles its own events
    }
}

pub struct MemberListHandle {
    pub(super) room_tx: mpsc::Sender<RoomCommand>,
    pub(super) key: MemberListKey,
    pub(super) events_tx: broadcast::Sender<MemberListEvent>,
}

impl MemberListHandle {
    pub async fn send_command(&self, cmd: MemberListCommand) -> Result<()> {
        self.room_tx
            .send(RoomCommand::MemberList(self.key.clone(), cmd))
            .await
            .map_err(|_| crate::Error::Internal("failed to send member list command".to_string()))
    }

    pub fn subscribe(&self) -> broadcast::Receiver<MemberListEvent> {
        self.events_tx.subscribe()
    }
}
