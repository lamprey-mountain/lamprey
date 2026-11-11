use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use common::v1::types::ack::{AckBulk, AckBulkItem, AckCreate, AckType};
use common::v1::types::util::Time;
use common::v1::types::{Channel, MessageSync};
use common::v2::types::{ChannelId, MessageId, RoomId, UserId};
use dashmap::DashMap;
use lamprey_backend_core::data::DbReadState;
use moka::future::Cache;
use tokio::sync::{Mutex, Notify};
use tracing::error;

use crate::prelude::*;

const FLUSH_INTERVAL: Duration = Duration::from_secs(5);
const FLUSH_DEBOUNCE: Duration = Duration::from_millis(500);

/// what a read state is for
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReadStateTarget {
    Channel(ChannelId),
    // TODO: documents, notifications, calendar events, etc
}

/// identifies a user's read state for a resource
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ReadStateKey {
    pub user_id: UserId,
    pub target: ReadStateTarget,
}

impl ReadStateKey {
    pub const fn channel(user_id: UserId, channel_id: ChannelId) -> Self {
        Self {
            user_id,
            target: ReadStateTarget::Channel(channel_id),
        }
    }
}

/// a user's read state for a resource
#[derive(Debug, Default, Clone, Copy)]
pub struct ReadState {
    // PERF: ensure MessageId is non-zero (niche)
    last_read_id: Option<MessageId>,
    last_read_at: Option<Time>,
    mention_count: u64,
    unread: bool,
    pins_read_at: Option<Time>,
}

impl ReadState {
    /// apply this read state to a channel
    pub fn apply(&self, channel: &mut Channel) {
        channel.is_unread = Some(self.unread);
        channel.last_read_id = self.last_read_id;
        channel.mention_count = Some(self.mention_count);
        channel.pins_read_at = self.pins_read_at;
    }
}

#[derive(Debug, Clone, Copy)]
struct PendingReadState {
    state: ReadState,
    revision: u64,
}

pub struct ServiceReadStates {
    globals: Globals,
    states: Cache<ReadStateKey, Arc<ReadState>>,
    pending: DashMap<ReadStateKey, PendingReadState>,
    flush_lock: Mutex<()>,
    notify: Arc<Notify>,
    revision: AtomicU64,
}

// PERF(future): store unreads in nats jetstream
// im not sure if im going to continue using nats or not though

impl ServiceReadStates {
    pub fn new(globals: Globals) -> Self {
        // NOTE: do i need support_invalidation_closures()?
        Self {
            globals,
            states: Cache::builder().max_capacity(100_000).build(),
            pending: DashMap::new(),
            flush_lock: Mutex::new(()),
            notify: Arc::new(Notify::new()),
            revision: AtomicU64::new(0),
        }
    }

    /// start the periodic read state flusher
    pub fn start_background_tasks(&self) {
        let globals = self.globals.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(FLUSH_INTERVAL);
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            loop {
                let srv = globals.services();
                tokio::select! {
                    _ = interval.tick() => {}
                    _ = srv.read_states.notify.notified() => {
                        // debounce bursts of writes
                        tokio::time::sleep(FLUSH_DEBOUNCE).await;
                    }
                }

                if let Err(err) = srv.read_states.flush().await {
                    error!("failed to flush read states: {err}");
                }
            }
        });
    }

    /// write all pending read state changes to the database
    pub async fn flush(&self) -> Result<()> {
        let _guard = self.flush_lock.lock().await;

        let pending: Vec<(ReadStateKey, PendingReadState)> = self
            .pending
            .iter()
            .map(|entry| (*entry.key(), *entry.value()))
            .collect();

        if pending.is_empty() {
            return Ok(());
        }

        let mut states: HashMap<UserId, Vec<(ChannelId, DbReadState)>> = HashMap::new();
        for (key, pending) in &pending {
            let channel_id = match key.target {
                ReadStateTarget::Channel(channel_id) => channel_id,
            };

            states
                .entry(key.user_id)
                .or_default()
                .push((channel_id, pending.state.into()));
        }

        let mut txn = self.globals.begin().await?;
        for (user_id, states) in &states {
            txn.unread_read_state_put(*user_id, states).await?;
        }
        txn.commit().await?;

        for (key, pending) in pending {
            self.pending
                .remove_if(&key, |_, current| current.revision == pending.revision);
        }

        Ok(())
    }

    /// update a user's read state for a channel
    pub async fn ack_channel(
        &self,
        user_id: UserId,
        channel_id: ChannelId,
        ack: AckCreate,
    ) -> Result<()> {
        let message_id = match ack.message_id {
            Some(message_id) => message_id,
            None => self
                .globals
                .services()
                .channels
                .read_metadata(channel_id)
                .await?
                .last_message_id
                .ok_or_else(|| Error::BadStatic("channel has no messages"))?,
        };

        let item = AckBulkItem {
            ty: AckType::Message {
                channel_id,
                message_id,
                mention_count: ack.mention_count,
            },
        };
        self.ack_bulk(user_id, &AckBulk { acks: vec![item] }).await
    }

    /// update a user's read state en masse
    pub async fn ack_bulk(&self, user_id: UserId, acks: &AckBulk) -> Result<()> {
        let now = Time::now_utc();

        for ack in &acks.acks {
            match &ack.ty {
                AckType::Message {
                    channel_id,
                    message_id,
                    mention_count,
                } => {
                    let key = ReadStateKey::channel(user_id, *channel_id);
                    self.update(key, |state| {
                        state.last_read_id = Some(*message_id);
                        state.last_read_at = Some(now);
                        state.mention_count = *mention_count;
                        state.unread = false;
                    })
                    .await?;
                }
                AckType::Pins { channel_id } => {
                    let key = ReadStateKey::channel(user_id, *channel_id);
                    self.update(key, |state| {
                        state.pins_read_at = Some(now);
                    })
                    .await?;
                }
            }
        }

        Ok(())
    }

    /// mark all channels in a room as read
    pub async fn ack_room(
        &self,
        user_id: UserId,
        room_id: RoomId,
    ) -> Result<Vec<(ChannelId, MessageId)>> {
        let acks = self
            .globals
            .begin_read()
            .await?
            .unread_room_last_messages(room_id)
            .await?;

        let now = Time::now_utc();
        for (channel_id, message_id) in &acks {
            let key = ReadStateKey::channel(user_id, *channel_id);
            self.update(key, |state| {
                state.last_read_id = Some(*message_id);
                state.last_read_at = Some(now);
                state.mention_count = 0;
                state.unread = false;
            })
            .await?;
        }

        Ok(acks)
    }

    /// get a user's read state from the cache, if it is cached
    pub async fn cached(&self, key: ReadStateKey) -> Option<ReadState> {
        self.states.get(&key).await.map(|state| *state)
    }

    /// get a user's read state
    pub async fn get(&self, key: ReadStateKey) -> Result<ReadState> {
        if let Some(state) = self.cached(key).await {
            return Ok(state);
        }

        let state = match key.target {
            ReadStateTarget::Channel(channel_id) => self
                .globals
                .begin_read()
                .await?
                .unread_read_state_get(key.user_id, channel_id)
                .await?
                .map(ReadState::from)
                .unwrap_or_default(),
        };

        self.cache(key, state).await;
        Ok(state)
    }

    /// update read state caches via a sync event
    pub async fn handle(&self, sync: &MessageSync) -> Result<()> {
        match sync {
            MessageSync::MessageCreate { message } => {
                self.globals
                    .services()
                    .channels
                    .update_read_metadata(message.channel_id, |metadata| {
                        metadata.last_message_id = Some(message.id);
                        metadata.last_version_id = Some(message.latest_version.version_id);
                    })
                    .await;

                // reset the author's read state if it is cached
                let key = ReadStateKey::channel(message.author_id, message.channel_id);
                if self.cached(key).await.is_some() {
                    self.update(key, |state| {
                        state.last_read_id = Some(message.id);
                        state.last_read_at = Some(Time::now_utc());
                        state.mention_count = 0;
                        state.unread = false;
                    })
                    .await?;
                }
            }
            MessageSync::MessageUpdate { message } => {
                if let Some(pinned) = &message.pinned {
                    self.globals
                        .services()
                        .channels
                        .update_read_metadata(message.channel_id, |metadata| {
                            metadata.pins_updated_at = Some(pinned.time);
                        })
                        .await;
                }
            }
            MessageSync::ChannelCreate { channel } => {
                if channel.ty.is_thread() {
                    // if parent.ty.is_thread_only()
                    // reset read state for channel.creator_id in channel.parent_id
                    // TODO: update last_message_id?
                }
            }
            _ => {}
        }

        Ok(())
    }

    /// increment the mention count for all of these users in a channel
    pub(crate) async fn increment_mentions(
        &self,
        channel_id: ChannelId,
        user_ids: &[UserId],
    ) -> Result<()> {
        for user_id in user_ids {
            let key = ReadStateKey::channel(*user_id, channel_id);
            self.update(key, |state| {
                state.mention_count = state.mention_count.saturating_add(1);
                state.unread = true;
            })
            .await?;
        }

        Ok(())
    }

    /// load, update, and store a read state
    async fn update(&self, key: ReadStateKey, update: impl FnOnce(&mut ReadState)) -> Result<()> {
        let mut state = self.get(key).await?;
        update(&mut state);
        self.store(key, state).await;
        Ok(())
    }

    /// update the cached read state without marking it as pending
    async fn cache(&self, key: ReadStateKey, state: ReadState) {
        self.states.insert(key, Arc::new(state)).await;
    }

    /// update the cached read state and mark it as pending
    async fn store(&self, key: ReadStateKey, state: ReadState) {
        self.cache(key, state).await;

        let revision = self.revision.fetch_add(1, Ordering::Relaxed) + 1;
        self.pending
            .entry(key)
            .and_modify(|pending| {
                pending.state = state;
                pending.revision = revision;
            })
            .or_insert(PendingReadState { state, revision });

        self.notify.notify_one();
    }
}

impl From<DbReadState> for ReadState {
    fn from(value: DbReadState) -> Self {
        Self {
            last_read_id: value.last_read_id,
            last_read_at: value.last_read_at,
            mention_count: value.mention_count,
            unread: value.unread,
            pins_read_at: value.pins_read_at,
        }
    }
}

impl From<ReadState> for DbReadState {
    fn from(value: ReadState) -> Self {
        Self {
            last_read_id: value.last_read_id,
            last_read_at: value.last_read_at,
            mention_count: value.mention_count,
            unread: value.unread,
            pins_read_at: value.pins_read_at,
        }
    }
}
