use lamprey::v1::types::SERVER_ROOM_ID;
use std::collections::{HashMap, HashSet};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use lamprey::v1::types::error::{ApiError, ErrorCode};
use lamprey::v1::types::{
    ChannelId, ConnectionId, DocumentBranchId, MessageSync, Permission, RedexId, UserId,
    sync::SyncSubscription,
};

use crate::error::{Error, Result};
use crate::services::member_lists::util::MemberListTarget;
use crate::state::Globals;

/// manager for all the subscriptions for a connection
pub struct ConnectionSubscriptions {
    globals: Globals,
    conn_id: ConnectionId,

    // multiplex all events into a single stream
    event_tx: mpsc::UnboundedSender<Result<MessageSync>>,
    event_rx: mpsc::UnboundedReceiver<Result<MessageSync>>,

    documents: HashMap<(ChannelId, DocumentBranchId), JoinHandle<()>>,
    scripts: HashMap<(ChannelId, RedexId), JoinHandle<()>>,
    member_lists: HashMap<String, (JoinHandle<()>, Vec<(u64, u64)>)>, // store ranges to detect when ranges are updated
}

impl ConnectionSubscriptions {
    pub fn new(globals: Globals, conn_id: ConnectionId) -> Self {
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        Self {
            globals,
            conn_id,
            event_tx,
            event_rx,
            documents: HashMap::new(),
            scripts: HashMap::new(),
            member_lists: HashMap::new(),
        }
    }

    pub fn is_document_subscribed(
        &self,
        channel_id: ChannelId,
        branch_id: DocumentBranchId,
    ) -> bool {
        self.documents.contains_key(&(channel_id, branch_id))
    }

    pub async fn disconnect(&mut self, user_id: UserId) {
        let srv = self.globals.services();

        for (key, _) in self.documents.iter() {
            let _ = srv
                .documents
                .remove_presence(*key, user_id, self.conn_id)
                .await;
        }

        for handle in self.documents.values() {
            handle.abort();
        }
        for handle in self.scripts.values() {
            handle.abort();
        }
        for (handle, _) in self.member_lists.values() {
            handle.abort();
        }

        self.documents.clear();
        self.scripts.clear();
        self.member_lists.clear();
    }

    pub async fn set_subscription(
        &mut self,
        subscription: SyncSubscription,
        user_id: UserId,
    ) -> Result<()> {
        let srv = self.globals.services();

        // document subscriptions
        if let Some(docs) = subscription.documents {
            let mut new_keys = HashSet::new();

            for doc in docs {
                let key = (doc.channel_id, doc.branch_id);
                new_keys.insert(key);

                if !self.documents.contains_key(&key) {
                    let perms = srv.perms.for_channel(user_id, doc.channel_id).await?;
                    perms.ensure(Permission::ChannelView)?;

                    let branch = self
                        .globals
                        .begin_read()
                        .await?
                        .document_branch_get(doc.channel_id, doc.branch_id)
                        .await;
                    match branch {
                        Ok(branch) => {
                            if branch.private && branch.creator_id != user_id {
                                return Err(Error::ApiError(ApiError::from_code(
                                    ErrorCode::UnknownDocumentBranch,
                                )));
                            }
                        }
                        Err(_) if *doc.branch_id == *doc.channel_id => {
                            // Default branch fallback
                        }
                        Err(_) => {
                            return Err(Error::ApiError(ApiError::from_code(
                                ErrorCode::UnknownDocumentBranch,
                            )));
                        }
                    }

                    let mut syncer = srv.documents.create_syncer(self.conn_id);
                    syncer.set_user_id(Some(user_id)).await;
                    syncer.set_context_id(key, doc.state_vector).await?;

                    let tx = self.event_tx.clone();
                    let handle = tokio::spawn(async move {
                        loop {
                            match syncer.poll().await {
                                Ok(msg) => {
                                    if tx.send(Ok(msg)).is_err() {
                                        // connection closed
                                        break;
                                    }
                                }
                                Err(e) => {
                                    let _ = tx.send(Err(e));
                                    break;
                                }
                            }
                        }
                    });
                    self.documents.insert(key, handle);
                }
            }

            self.documents.retain(|k, handle| {
                if new_keys.contains(k) {
                    true
                } else {
                    handle.abort();
                    // TODO: remove presence
                    // srv.documents.remove_presence(*k, user_id, self.conn_id).await;
                    false
                }
            });
        }

        // script subscriptions
        if let Some(scripts) = subscription.scripts {
            let mut new_keys = HashSet::new();

            for script in scripts {
                let key = (script.channel_id, script.script_id);
                new_keys.insert(key);

                if !self.scripts.contains_key(&key) {
                    let perms = srv
                        .perms
                        .for_channel2(Some(user_id), script.channel_id)
                        .await?;
                    perms.ensure(Permission::ChannelView)?;

                    let mut syncer = srv.scripts.create_syncer(self.conn_id);
                    syncer.set_user_id(Some(user_id)).await;
                    syncer
                        .set_context_id(script.channel_id, script.script_id)
                        .await?;

                    let tx = self.event_tx.clone();
                    let handle = tokio::spawn(async move {
                        loop {
                            match syncer.poll().await {
                                Ok(msg) => {
                                    if tx.send(Ok(msg)).is_err() {
                                        break;
                                    }
                                }
                                Err(e) => {
                                    let _ = tx.send(Err(e));
                                    break;
                                }
                            }
                        }
                    });
                    self.scripts.insert(key, handle);
                }
            }

            self.scripts.retain(|k, handle| {
                if new_keys.contains(k) {
                    true
                } else {
                    handle.abort();
                    false
                }
            });
        }

        // member list subscriptions
        if let Some(member_lists) = subscription.member_lists {
            let mut new_keys = HashSet::new();

            for ml in member_lists {
                // PERF: don't use strings for keys
                let key = if let Some(room_id) = ml.room_id {
                    format!("room:{}", room_id)
                } else if let Some(channel_id) = ml.channel_id {
                    format!("channel:{}", channel_id)
                } else {
                    continue;
                };

                new_keys.insert(key.clone());

                // PERF: see if i can reuse subscriptions when ranges change
                if let Some((handle, old_ranges)) = self.member_lists.get(&key) {
                    if old_ranges != &ml.ranges {
                        handle.abort();
                        self.member_lists.remove(&key);
                    }
                }

                if !self.member_lists.contains_key(&key) {
                    let target = if let Some(room_id) = ml.room_id {
                        let perms = srv.perms.for_room2(Some(user_id), room_id).await?;
                        if room_id == SERVER_ROOM_ID {
                            perms.ensure(Permission::ServerOversee)?;
                        }
                        Some(MemberListTarget::Room(room_id))
                    } else if let Some(channel_id) = ml.channel_id {
                        let perms = srv.perms.for_channel2(Some(user_id), channel_id).await?;
                        perms.ensure(Permission::ChannelView)?;
                        Some(MemberListTarget::Channel(channel_id))
                    } else {
                        None
                    };

                    if let Some(t) = target {
                        let mut syncer = srv.member_lists.create_syncer(self.conn_id.into());
                        syncer.set_user_id(Some(user_id)).await;
                        syncer.set_query(t, &ml.ranges).await?;

                        let tx = self.event_tx.clone();
                        let handle = tokio::spawn(async move {
                            loop {
                                match syncer.poll().await {
                                    Ok(msg) => {
                                        if tx.send(Ok(msg)).is_err() {
                                            break;
                                        }
                                    }
                                    Err(e) => {
                                        let _ = tx.send(Err(e));
                                        break;
                                    }
                                }
                            }
                        });
                        self.member_lists.insert(key, (handle, ml.ranges));
                    }
                }
            }

            self.member_lists.retain(|k, (handle, _)| {
                if new_keys.contains(k) {
                    true
                } else {
                    handle.abort();
                    false
                }
            });
        }

        Ok(())
    }

    pub async fn poll(&mut self) -> Result<MessageSync> {
        match self.event_rx.recv().await {
            Some(res) => res,
            None => std::future::pending().await,
        }
    }
}

impl Drop for ConnectionSubscriptions {
    fn drop(&mut self) {
        for handle in self.documents.values() {
            handle.abort();
        }
        for handle in self.scripts.values() {
            handle.abort();
        }
        for (handle, _) in self.member_lists.values() {
            handle.abort();
        }
    }
}

#[cfg(any())]
mod next {
    /// manager for all the subscriptions for a connection
    pub struct ConnectionSubscriptions2 {
        // // multiplex all events into a single stream
        // event_tx: mpsc::UnboundedSender<Result<MessageSync>>,
        // event_rx: mpsc::UnboundedReceiver<Result<MessageSync>>,

        // documents: HashMap<(ChannelId, DocumentBranchId), JoinHandle<()>>,
        // scripts: HashMap<(ChannelId, RedexId), JoinHandle<()>>,
        // member_lists: HashMap<String, (JoinHandle<()>, Vec<(u64, u64)>)>, // store ranges to detect when ranges are updated
    }

    pub trait Syncer {
        // fn set_subscription(&mut self, subscription: SyncSubscription);
        fn poll(&mut self) -> impl Future<Output = Result<MessageSync>> + Send;
    }

    // pub trait ServiceFoo {
    //     fn create(&self, connection_id: ConnectionId, user_id: Option<UserId>) -> Syncer;
    // }

    // pub fn create_syncer(&self, conn_id: uuid::Uuid) -> syncer::MemberListSyncer {
    // pub trait Syncer {
    //     pub async fn set_user_id(&mut self, user_id: Option<UserId>) {
    //     pub async fn poll(&mut self) -> Result<MessageSync> {

    //     // member list
    //     pub async fn set_query( &mut self, target: MemberListTarget, ranges: &[(u64, u64)], ) -> Result<()> {
    //     pub async fn clear_query(&mut self) {
    //     pub async fn subscribe(&mut self, key1: MemberListKey1, ranges: Vec<(u64, u64)>) -> Result<()> {
    //     pub async fn unsubscribe(&mut self, key1: MemberListKey1) -> Result<()> {

    // // document
    //     pub async fn set_context_id( &self, context_id: EditContextId, state_vector: Option<DocumentStateVector>, ) -> Result<()> {
    //     pub fn is_subscribed(&self, context_id: &EditContextId) -> bool {
    //     pub async fn handle_disconnect(&self, user_id: UserId) -> Result<()> {

    //    // script
    //     pub async fn set_context_id(&self, channel_id: ChannelId, script_id: RedexId) -> Result<()> {
    //     pub fn is_subscribed(&self, channel_id: &ChannelId, script_id: &RedexId) -> bool {
    // }

    // unsure if i can impl these if i make ConnectionSubscriptions2 minimal
    // impl ConnectionSubscriptions2 {
    //     pub async fn disconnect(&mut self) {
    //         todo!()
    //     }
    //
    //     pub async fn set_subscription(
    //         &mut self,
    //         subscription: SyncSubscription,
    //         user_id: UserId,
    //     ) -> Result<()> {
    //         todo!()
    //     }
    //
    //     pub async fn poll(&mut self) -> Result<MessageSync> {
    //         todo!()
    //     }
    // }
}
