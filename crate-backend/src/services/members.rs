//! Service for managing member lists
//!
//! ## Member list logic
//!
//! In threads, the active member set is all members who are have an associated
//! thread_member object. In other channels, a member is active if they can view
//! the channel.
//!
//! A group is formed for each hoisted role, online members, and offline members.
//! Role groups are returned first (ordered by position), followed by online
//! members, then finally by offline members. A member is part of group formed by
//! their highest hoisted role. Role groups only contain online members, offline
//! members are always part of the offline group regardless of roles. If a group
//! has no members, it is not returned.
//!
//! After the member sets are filtered and grouped, they are ordered by their
//! display name. The display name uses the room override_name, falling back to
//! user name.

use std::sync::Arc;

use common::v1::types::{MemberListGroup, MemberListOp, MessageSync, UserId};
use dashmap::DashMap;
use tokio::sync::{broadcast, Mutex};
use tracing::{debug, warn};

use crate::{
    services::members::{
        lists::MemberList,
        util::{MemberListKey, MemberListVisibility},
    },
    Error, Result, ServerState, ServerStateInner,
};

/// helpful utilities for member lists
mod util;

/// member list implementation
mod lists;

pub use util::MemberListTarget;

pub struct ServiceMembers {
    s: Arc<ServerStateInner>,
    lists: DashMap<MemberListKey, Arc<MemberListHandler>>,
}

/// one syncer exists for each connected session
pub struct MemberListSyncer {
    s: Arc<ServerStateInner>,
    user_id: Mutex<Option<UserId>>,
    query_tx: tokio::sync::watch::Sender<Option<MemberListQuery>>,
    query_rx: tokio::sync::watch::Receiver<Option<MemberListQuery>>,
    ops_rx: Mutex<Option<broadcast::Receiver<MemberListSync>>>,
}

/// a member list query/subscription
#[derive(Debug)]
pub struct MemberListQuery {
    key: MemberListKey,
    ranges: Vec<(u64, u64)>,
}

/// one handler exists for each member list
pub struct MemberListHandler {
    s: Arc<ServerState>,
    list: MemberList,
    tx: broadcast::Receiver<MemberListSync>,
}

/// minimal member list sync payload for broadcasting
#[derive(Debug, Clone)]
pub struct MemberListSync {
    pub key: MemberListKey,
    pub ops: Vec<MemberListOp>,
    pub groups: Vec<MemberListGroup>,
}

impl MemberListSync {
    pub fn into_sync_message(self, user_id: UserId) -> MessageSync {
        MessageSync::MemberListSync {
            user_id,
            room_id: self.key.room_id,
            channel_id: self.key.channel_id,
            ops: self.ops,
            groups: self.groups,
        }
    }
}

impl ServiceMembers {
    pub fn new(state: Arc<ServerStateInner>) -> Self {
        Self {
            s: state,
            lists: DashMap::new(),
        }
    }

    // pub fn new(state: Arc<ServerStateInner>) -> Self {
    //     let inner = Arc::new(ServiceMembersInner {
    //         state: state.clone(),
    //         member_lists: DashMap::new(),
    //         cache_room_member: Cache::builder().max_capacity(1_000_000).build(),
    //         cache_thread_member: Cache::builder().max_capacity(1_000_000).build(),
    //     });

    //     let inner2 = inner.clone();
    //     let mut sub = state.sushi.subscribe();
    //     tokio::spawn(async move {
    //         while let Ok(msg) = sub.recv().await {
    //             if let Err(err) = inner2.handle_event(&msg).await {
    //                 error!("service members error: {err}");
    //             }
    //         }
    //     });

    //     Self { inner }
    // }

    /// create a new MemberListSyncer for a session
    pub fn create_syncer(&self) -> MemberListSyncer {
        let (query_tx, query_rx) = tokio::sync::watch::channel(None);
        MemberListSyncer {
            s: self.s.clone(),
            user_id: Mutex::new(None),
            query_tx,
            query_rx,
            ops_rx: Mutex::new(None),
        }
    }

    /// spawn a handler for a key if it doesnt exist
    // TODO: shutdown unused receivers after a period of time
    // TODO: better error handling (maybe return Result?)
    pub async fn ensure_handler(&self, key: MemberListKey) -> broadcast::Receiver<MemberListSync> {
        if let Some(list) = self.lists.get(&key) {
            return list.tx.resubscribe();
        }

        let (tx, rx) = broadcast::channel(100);
        let mut events = self.s.sushi.subscribe();
        let s = self.s.clone();

        tokio::spawn(async move {
            let mut list = match MemberList::new_from_server_inner(key.clone(), &s).await {
                Ok(l) => l,
                Err(e) => {
                    warn!("failed to init member list: {e:?}");
                    return;
                }
            };

            loop {
                let msg = events.recv().await.expect("error while receiving event");
                let ops = match msg {
                    MessageSync::ChannelUpdate { channel } => {
                        let srv = s.services();
                        let overwrites = srv
                            .channels
                            .fetch_overwrite_ancestors(channel.id)
                            .await
                            .unwrap();
                        let v = MemberListVisibility { overwrites };
                        list.set_visibility(v)
                    }
                    msg => list.process(&msg),
                };
                if !ops.is_empty() {
                    tx.send(MemberListSync {
                        key: key.clone(),
                        ops,
                        groups: list.groups(),
                    })
                    .unwrap();
                }
            }
        });

        // FIXME: create handler, insert into self.lists

        rx
    }
}

impl MemberListSyncer {
    /// set the user id for this syncer
    pub async fn set_user_id(&self, user_id: Option<UserId>) {
        debug!("set user_id to {user_id:?}");
        *self.user_id.lock().await = user_id;
    }

    /// set a new query
    pub async fn set_query(&self, target: MemberListTarget, ranges: &[(u64, u64)]) -> Result<()> {
        debug!("set query to {target:?}, {ranges:?}");
        let srv = self.s.services();
        let key = match target.clone() {
            MemberListTarget::Room(room_id) => MemberListKey {
                room_id: Some(room_id),
                channel_id: None,
            },
            MemberListTarget::Channel(channel_id) => {
                let channel = srv.channels.get(channel_id, None).await?;
                MemberListKey {
                    room_id: channel.room_id,
                    channel_id: Some(channel_id),
                }
            }
        };
        *self.ops_rx.lock().await = Some(srv.members.ensure_handler(key.clone()).await);
        self.query_tx
            .send(Some(MemberListQuery {
                key,
                ranges: ranges.to_vec(),
            }))
            .unwrap();
        Ok(())
    }

    // pub async fn clear_query(&self) {
    //     *self.ops_rx.lock().await = None;
    //     self.query_tx.send(None).unwrap();
    // }

    /// poll for the next member list sync message
    // TODO: better error handling for changed
    pub async fn poll(&self) -> Result<MessageSync> {
        let mut qrx = self.query_rx.clone();
        if let Some(ops_rx) = &mut *self.ops_rx.lock().await {
            tokio::select! {
                op = ops_rx.recv() => {
                    debug!("recv member list message");
                    let msg = op.map_err(|_| Error::BadStatic("member list handler closed"))?.into_sync_message(self.user_id().await);
                    Ok(msg)
                }
                changed = qrx.changed() => {
                    changed.unwrap();
                    debug!("query changed, getting initial ranges");
                    Ok(self.get_initial_ranges().await)
                }
            }
        } else {
            qrx.changed().await.unwrap();
            debug!("query changed, getting initial ranges");
            Ok(self.get_initial_ranges().await)
        }
    }

    async fn get_initial_ranges(&self) -> MessageSync {
        let user_id = self.user_id().await;
        debug!("get initial ranges for {user_id:?}");
        let q = self.query_rx.borrow();
        let q = q.as_ref().unwrap();
        let srv = self.s.services();
        let handler = srv.members.lists.get(&q.key).unwrap();
        let ops = handler.get_initial_ranges(&q.ranges);
        MessageSync::MemberListSync {
            user_id,
            room_id: q.key.room_id,
            channel_id: q.key.channel_id,
            ops,
            groups: handler.list().groups(),
        }
    }

    async fn user_id(&self) -> UserId {
        self.user_id.lock().await.unwrap()
    }
}

impl MemberListHandler {
    pub fn get_initial_ranges(&self, ranges: &[(u64, u64)]) -> Vec<MemberListOp> {
        self.list.get_initial_ranges(ranges)
    }

    pub fn list(&self) -> &MemberList {
        &self.list
    }
}
