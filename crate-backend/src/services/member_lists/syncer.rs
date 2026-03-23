use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;

use common::v1::types::{ChannelId, MemberListOp, MessageSync, RoomId, UserId};
use tokio_stream::{wrappers::BroadcastStream, StreamExt, StreamMap, StreamNotifyClose};
use uuid::Uuid;

use crate::{
    error::Result,
    services::member_lists::{
        actor::{MemberListCommand, MemberListEvent},
        util::{MemberListKey, MemberListKey1, MemberListTarget},
    },
    services::rooms::MemberListCommandMsg,
    Error, ServerStateInner,
};

/// Syncer for member list events
pub struct MemberListSyncer {
    pub(super) s: Arc<ServerStateInner>,
    pub(super) conn_id: Uuid,
    pub(super) outbox: VecDeque<MessageSync>,
    pub(super) subscriptions: HashMap<MemberListKey, HashSet<MemberListKey1>>,
    pub(super) streams:
        StreamMap<MemberListKey, StreamNotifyClose<BroadcastStream<MemberListEvent>>>,
    pub(super) known_users: HashSet<UserId>,
    pub(super) known_room_members: HashSet<(RoomId, UserId)>,
    pub(super) known_thread_members: HashSet<(ChannelId, UserId)>,
    pub(super) user_id: Option<UserId>,
    pub(super) current_key: Option<MemberListKey1>,
}

impl MemberListSyncer {
    /// Create a new member list syncer
    pub(super) fn new(s: Arc<ServerStateInner>, conn_id: Uuid) -> Self {
        Self {
            s,
            conn_id,
            outbox: VecDeque::new(),
            subscriptions: HashMap::new(),
            streams: StreamMap::new(),
            known_users: HashSet::new(),
            known_room_members: HashSet::new(),
            known_thread_members: HashSet::new(),
            user_id: None,
            current_key: None,
        }
    }

    /// Set the user ID for this syncer
    pub async fn set_user_id(&mut self, user_id: Option<UserId>) {
        self.user_id = user_id;
    }

    /// Set the member list query
    pub async fn set_query(
        &mut self,
        target: MemberListTarget,
        ranges: &[(u64, u64)],
    ) -> Result<()> {
        if let Some(key) = self.current_key.take() {
            let _ = self.unsubscribe(key).await;
        }

        let srv = self.s.services();
        let key1 = match target {
            MemberListTarget::Room(room_id) => MemberListKey1::Room(room_id),
            MemberListTarget::Channel(channel_id) => {
                let channel = srv.channels.get(channel_id, None).await?;
                if let Some(room_id) = channel.room_id {
                    MemberListKey1::RoomChannel(room_id, channel_id)
                } else {
                    MemberListKey1::DmChannel(channel_id)
                }
            }
        };

        self.subscribe(key1, ranges.to_vec()).await?;
        self.current_key = Some(key1);
        Ok(())
    }

    /// Clear the current member list query
    pub async fn clear_query(&mut self) {
        if let Some(key) = self.current_key.take() {
            let _ = self.unsubscribe(key).await;
        }
    }

    /// Subscribe to a member list
    pub async fn subscribe(&mut self, key1: MemberListKey1, ranges: Vec<(u64, u64)>) -> Result<()> {
        let srv = self.s.services();
        let key = srv.member_lists.lookup_member_key(key1).await?;

        self.subscriptions
            .entry(key.clone())
            .or_default()
            .insert(key1);

        let list = srv.member_lists.ensure(key.clone()).await?;

        // Try to send the command; if it fails, evict the dead actor and retry
        let conn_id = self.conn_id;
        let result = list
            .actor_ref
            .ask(MemberListCommandMsg {
                key: key.clone(),
                cmd: MemberListCommand::GetInitialRanges {
                    ranges: ranges.clone(),
                    conn_id,
                },
            })
            .send()
            .await;

        if result.is_err() {
            // Actor is dead, evict it and get a fresh one
            let room_id = key
                .room_id()
                .ok_or_else(|| Error::Internal("no room id for member list key".to_string()))?;

            srv.rooms.unload_cache(room_id).await;

            let list = srv.member_lists.ensure(key.clone()).await?;

            let reply_result = list
                .actor_ref
                .ask(MemberListCommandMsg {
                    key: key.clone(),
                    cmd: MemberListCommand::GetInitialRanges {
                        ranges,
                        conn_id: self.conn_id,
                    },
                })
                .send()
                .await;

            let reply = match reply_result {
                Ok(r) => r,
                Err(e) => {
                    return Err(Error::Internal(format!(
                        "failed to send member list command: {e}"
                    )))
                }
            };

            let mut initial_sync = match reply {
                Some(sync) => sync,
                None => {
                    return Err(Error::Internal(
                        "member list command returned None".to_string(),
                    ))
                }
            };

            self.patch_msg_key(&mut initial_sync, &key1);
            self.outbox.push_back(initial_sync);

            if !self.streams.contains_key(&key) {
                let stream = StreamNotifyClose::new(BroadcastStream::new(list.subscribe()));
                self.streams.insert(key, stream);
            }

            return Ok(());
        }

        let reply_result = result;

        let reply = match reply_result {
            Ok(r) => r,
            Err(e) => {
                return Err(Error::Internal(format!(
                    "failed to receive initial ranges: {e}"
                )))
            }
        };

        let mut initial_sync = match reply {
            Some(sync) => sync,
            None => {
                return Err(Error::Internal(
                    "member list command returned None".to_string(),
                ))
            }
        };

        self.patch_msg_key(&mut initial_sync, &key1);
        self.outbox.push_back(initial_sync);

        if !self.streams.contains_key(&key) {
            let stream = StreamNotifyClose::new(BroadcastStream::new(list.subscribe()));
            self.streams.insert(key, stream);
        }

        Ok(())
    }

    /// Unsubscribe from a member list
    pub async fn unsubscribe(&mut self, key1: MemberListKey1) -> Result<()> {
        let srv = self.s.services();
        let key = srv.member_lists.lookup_member_key(key1).await?;
        if let Some(subs) = self.subscriptions.get_mut(&key) {
            subs.remove(&key1);
            if subs.is_empty() {
                self.subscriptions.remove(&key);
                self.streams.remove(&key);
            }
        }
        Ok(())
    }

    /// Poll for new events
    pub async fn poll(&mut self) -> Result<MessageSync> {
        let user_id = match self.user_id {
            Some(uid) => uid,
            None => std::future::pending().await,
        };

        loop {
            if let Some(mut msg) = self.outbox.pop_front() {
                self.patch_msg(&mut msg, user_id);
                return Ok(msg);
            }

            tokio::select! {
                Some((key, val)) = self.streams.next() => {
                    let msg = match val {
                        Some(Ok(MemberListEvent::Broadcast(msg))) => msg,
                        Some(Ok(MemberListEvent::Unicast(conn_id, msg))) if conn_id == self.conn_id => msg,
                        Some(Ok(_)) => continue, // skip other unicasts
                        Some(Err(e)) => return Err(Error::Internal(format!("member list stream error: {e}"))),
                        None => continue, // stream closed, try next
                    };

                    // PERF: maybe don't clone msg multiple times?
                    if let Some(subs) = self.subscriptions.get(&key) {
                        for key1 in subs {
                            let mut m = msg.clone();
                            self.patch_msg_key(&mut m, key1);
                            self.outbox.push_back(m);
                        }
                    }
                }
                else => std::future::pending().await,
            }
        }
    }

    fn patch_msg_key(&self, msg: &mut MessageSync, key1: &MemberListKey1) {
        if let MessageSync::MemberListSync { channel_id, .. } = msg {
            *channel_id = key1.channel_id();
        }
    }

    fn patch_msg(&mut self, msg: &mut MessageSync, user_id: UserId) {
        if let MessageSync::MemberListSync {
            user_id: ref mut uid,
            room_id,
            channel_id,
            ops,
            ..
        } = msg
        {
            *uid = user_id;

            for op in ops {
                match op {
                    MemberListOp::Sync {
                        room_members,
                        thread_members,
                        users,
                        ..
                    } => {
                        if let Some(users_vec) = users {
                            users_vec.retain(|u| self.known_users.insert(u.id));
                            if users_vec.is_empty() {
                                *users = None;
                            }
                        }
                        if let (Some(rid), Some(rm_vec)) = (room_id.as_ref(), room_members.as_mut())
                        {
                            rm_vec.retain(|m| self.known_room_members.insert((*rid, m.user_id)));
                            if rm_vec.is_empty() {
                                *room_members = None;
                            }
                        }
                        if let (Some(tid), Some(tm_vec)) =
                            (channel_id.as_ref(), thread_members.as_mut())
                        {
                            tm_vec.retain(|m| self.known_thread_members.insert((*tid, m.user_id)));
                            if tm_vec.is_empty() {
                                *thread_members = None;
                            }
                        }
                    }
                    MemberListOp::Insert {
                        user_id,
                        room_member,
                        thread_member,
                        user,
                        ..
                    } => {
                        if !self.known_users.insert(*user_id) {
                            *user = None;
                        }
                        if let (Some(rid), Some(m)) = (room_id.as_ref(), room_member.as_ref()) {
                            if !self.known_room_members.insert((*rid, m.user_id)) {
                                *room_member = None;
                            }
                        }
                        if let (Some(tid), Some(m)) = (channel_id.as_ref(), thread_member.as_ref())
                        {
                            if !self.known_thread_members.insert((*tid, m.user_id)) {
                                *thread_member = None;
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}
