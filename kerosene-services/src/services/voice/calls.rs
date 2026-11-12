use crate::Result;
use crate::services::voice::ServiceVoice;
use crate::services::voice::voice_state::VoiceStateHandle;
use common::v1::types::error::{ApiError, ErrorCode};
use common::v1::types::voice::CallMetadata;
use common::v1::types::{
    ChannelId, ChannelType, MessageId, SfuId, UserId,
    util::Time,
    voice::{Call, CallCreate, CallPatch},
};
use common::v1::types::{MessageCall, MessageSync, MessageType};
use dashmap::{DashMap, DashSet};
use lamprey_backend_data_postgres::{DbMessageCreate, DbMessageUpdate};
use std::{sync::Arc, time::Duration};
use tokio::time::Instant;
use tracing::error;

const CALL_CLEANUP_TIMEOUT: Duration = Duration::from_secs(300);
const AUDIENCE_UPDATE_INTERVAL: Duration = Duration::from_secs(30);

pub struct CallHandleInner {
    pub call: Call,
    pub sfus: DashSet<SfuId>,
    pub cleanup_task: Option<tokio::task::AbortHandle>,
    pub voice_states: DashMap<UserId, VoiceStateHandle>,
    pub message_id: Option<MessageId>,
}

pub type CallHandle = Arc<CallHandleInner>;

#[derive(Default)]
pub struct AudienceUpdate {
    last_broadcast_at: Option<Instant>,
    last_count: Option<u64>,
    pending: Option<tokio::task::AbortHandle>,
}

impl CallHandleInner {
    #[inline]
    pub fn id(&self) -> ChannelId {
        self.call.channel_id
    }

    #[inline]
    pub fn call(&self) -> &Call {
        &self.call
    }
}

// trait CallHandleExt {
//     pub fn update_voice_state(&self);
// }

// impl CallHandleExt for Arc<CallHandle> {
//     // TODO
// }

impl ServiceVoice {
    /// get a call
    pub fn call_get(&self, channel_id: ChannelId) -> Option<CallHandle> {
        self.calls.get(&channel_id).map(|c| c.value().clone())
    }

    /// mark call messages left active by a previous server instance as ended
    pub async fn cleanup_stale_calls(&self) -> Result<()> {
        let mut txn = self.state.begin().await?;
        let messages = txn.message_call_list_active().await?;
        let ended_at = Time::now_utc();

        for message in messages {
            let MessageType::Call(call) = &message.latest_version.message_type else {
                continue;
            };

            let update = DbMessageUpdate {
                attachments: vec![],
                author_id: message.author_id,
                embeds: vec![],
                components: vec![],
                message_type: MessageType::Call(MessageCall {
                    ended_at: Some(ended_at),
                    participants: call.participants.clone(),
                }),
                created_at: Some(message.created_at.into()),
                mentions: Default::default(),
            };

            txn.message_update_in_place(
                message.channel_id,
                message.latest_version.version_id,
                update,
            )
            .await?;
        }

        txn.commit().await?;
        Ok(())
    }

    /// create a call
    // TODO: make sure that calls are automatically created when a voice state is created/updated for a channel for the first time
    pub async fn call_create(
        &self,
        channel_id: ChannelId,
        user_id: UserId,
        params: CallCreate,
    ) -> Result<CallHandle> {
        // 1. return (and bump) existing call if it exists
        if self.calls.contains_key(&channel_id) {
            self.call_bump(channel_id);
            return Ok(self.call_get(channel_id).unwrap());
        }

        // 2. fetch room_id for new call
        let srv = self.state.services();
        let channel = srv.channels.get(channel_id, None).await?;
        let call = Call {
            room_id: channel.room_id,
            channel_id: channel_id,
            inner: CallMetadata {
                topic: params.topic,
                created_at: Time::now_utc(),
                audience_count: (channel.ty == ChannelType::Broadcast).then_some(0),
                ringing: Vec::new(),
            },
        };

        // 3. insert handle
        let cleanup_task = if channel.ty == ChannelType::Broadcast {
            Some(self.spawn_cleanup_task(channel_id))
        } else {
            None
        };
        let handle = Arc::new(CallHandleInner {
            call: call.clone(),
            sfus: DashSet::new(),
            cleanup_task,
            voice_states: DashMap::new(),
            message_id: None,
        });

        self.calls.insert(channel_id, Arc::clone(&handle));

        // 4. broadcast CallCreate
        self.state
            .messaging()
            .broadcast_channel(channel_id, MessageSync::CallCreate { call })
            .await?;

        // 5. send call message, if needed
        if channel.is_dm() {
            // TODO: split out this code
            let globals = self.state.clone();
            tokio::spawn(async move {
                let _srv = globals.services();
                // TODO(?): move this logic to messages service
                let mut txn = globals.begin().await?;
                let message_id = txn
                    .message_create(DbMessageCreate {
                        id: None,
                        channel_id,
                        attachments: vec![],
                        author_id: user_id,
                        embeds: vec![],
                        components: vec![],
                        message_type: MessageType::Call(MessageCall {
                            ended_at: None,
                            participants: vec![user_id],
                        })
                        .into(),
                        created_at: None,
                        removed_at: None,
                        flume: None,
                        mentions: Default::default(),
                        interaction: None,
                        ephemeral: false,
                    })
                    .await?;
                let message = txn.message_get(channel_id, message_id).await?;
                txn.commit().await?;

                let event = MessageSync::MessageCreate { message };

                globals
                    .messaging()
                    .broadcast_channel(channel_id, event)
                    .await?;

                if let Some(mut entry) = globals.services().voice.calls.get_mut(&channel_id) {
                    let handle = entry.value();
                    let updated_handle = Arc::new(CallHandleInner {
                        call: handle.call.clone(),
                        sfus: handle.sfus.clone(),
                        cleanup_task: handle.cleanup_task.clone(),
                        voice_states: handle.voice_states.clone(),
                        message_id: Some(message_id),
                    });
                    *entry.value_mut() = updated_handle;
                }

                Result::Ok(())
            });
        }

        Ok(handle)
    }

    /// delete a call
    ///
    /// by default, this will not delete calls with members still in it. pass `force = true` to disconnect everyone first.
    ///
    /// returns true if this call was deleted
    pub async fn call_delete(&self, channel_id: ChannelId, force: bool) -> bool {
        if force {
            let _ = self.call_disconnect_all(channel_id).await;
        }

        if let Some(entry) = self.calls.get(&channel_id) {
            let handle = entry.value();
            if !handle.voice_states.is_empty() && !force {
                return false;
            }
        }

        if let Some((_, handle)) = self.calls.remove(&channel_id) {
            if let Some(task) = &handle.cleanup_task {
                task.abort();
            }

            if let Some((_, audience)) = self.audience_updates.remove(&channel_id) {
                if let Some(task) = audience.pending {
                    task.abort();
                }
            }

            if let Some(message_id) = handle.message_id {
                // TODO: split out this code
                let globals = self.state.clone();
                tokio::spawn(async move {
                    let result = async move {
                        let mut txn = globals.begin().await?;

                        // PERF: don't read message from db, store metadata in memory
                        let message = txn.message_get(channel_id, message_id).await?;

                        let MessageType::Call(message_call) = message.latest_version.message_type
                        else {
                            return Ok(());
                        };

                        let update = DbMessageUpdate {
                            attachments: vec![],
                            author_id: message.author_id,
                            embeds: vec![],
                            components: vec![],
                            message_type: MessageType::Call(MessageCall {
                                ended_at: Some(Time::now_utc()),
                                participants: message_call.participants,
                            }),
                            created_at: Some(message.created_at.into()),
                            mentions: Default::default(),
                        };

                        txn.message_update_in_place(channel_id, (*message_id).into(), update)
                            .await?;
                        let message = txn.message_get(channel_id, message_id).await?;
                        txn.commit().await?;

                        globals
                            .messaging()
                            .broadcast_channel(channel_id, MessageSync::MessageUpdate { message })
                            .await?;

                        Result::Ok(())
                    }
                    .await;
                    if let Err(err) = result {
                        error!("couldn't update message: {err}");
                    }
                });
            }

            let _ = self
                .state
                .messaging()
                .broadcast_channel(channel_id, MessageSync::CallDelete { channel_id })
                .await;

            return true;
        }

        false
    }

    /// update a broadcast call's audience count
    ///
    /// recomputes the count from active voice states and broadcasts `CallUpdate`,
    /// throttled to at most once every 30 seconds per channel.
    pub async fn call_update_audience(&self, channel_id: ChannelId) -> Result<()> {
        let audience_count = {
            let Some(handle) = self.call_get(channel_id) else {
                return Ok(());
            };

            handle
                .voice_states
                .iter()
                .filter(|state| state.value().inner().suppress)
                .count() as u64
        };

        let channel = self.state.services().channels.get(channel_id, None).await?;
        if channel.ty != ChannelType::Broadcast {
            return Ok(());
        }

        self.call_set_audience_count(channel_id, audience_count);
        self.call_throttle_audience_update(channel_id, audience_count)
            .await
    }

    fn call_set_audience_count(&self, channel_id: ChannelId, audience_count: u64) {
        let Some(mut entry) = self.calls.get_mut(&channel_id) else {
            return;
        };

        let handle = entry.value();
        if handle.call.inner.audience_count == Some(audience_count) {
            return;
        }

        let mut call = handle.call.clone();
        call.inner.audience_count = Some(audience_count);

        *entry.value_mut() = Arc::new(CallHandleInner {
            call,
            sfus: handle.sfus.clone(),
            cleanup_task: handle.cleanup_task.clone(),
            voice_states: handle.voice_states.clone(),
            message_id: handle.message_id,
        });
    }

    async fn call_throttle_audience_update(
        &self,
        channel_id: ChannelId,
        audience_count: u64,
    ) -> Result<()> {
        let now = Instant::now();
        let mut scheduled = false;

        {
            let mut entry = self.audience_updates.entry(channel_id).or_default();

            if entry.last_count == Some(audience_count) {
                if let Some(task) = entry.pending.take() {
                    task.abort();
                }
                return Ok(());
            }

            if entry.pending.is_some() {
                return Ok(());
            }

            let can_broadcast = entry
                .last_broadcast_at
                .is_none_or(|last| now.duration_since(last) >= AUDIENCE_UPDATE_INTERVAL);

            if can_broadcast {
                if let Some(task) = entry.pending.take() {
                    task.abort();
                }
                entry.last_broadcast_at = Some(now);
                entry.last_count = Some(audience_count);
            } else if entry.pending.is_none() {
                let last = entry.last_broadcast_at.expect("call must be broadcast");
                let delay = AUDIENCE_UPDATE_INTERVAL - now.duration_since(last);
                let state = self.state.clone();

                let task = tokio::spawn(async move {
                    tokio::time::sleep(delay).await;

                    let voice = state.services();
                    if let Err(err) = voice.voice.call_flush_audience_update(channel_id).await {
                        error!("couldn't update call audience: {err}");
                    }
                })
                .abort_handle();

                entry.pending = Some(task);
                scheduled = true;
            }

            drop(entry);
        }

        if scheduled {
            return Ok(());
        }

        self.call_broadcast_audience_update(channel_id).await
    }

    async fn call_flush_audience_update(&self, channel_id: ChannelId) -> Result<()> {
        let Some(handle) = self.call_get(channel_id) else {
            self.audience_updates.remove(&channel_id);
            return Ok(());
        };

        let audience_count = handle.call.inner.audience_count.unwrap_or_default();

        let should_broadcast = {
            let mut entry = self.audience_updates.entry(channel_id).or_default();
            entry.pending = None;

            if entry.last_count == Some(audience_count) {
                false
            } else {
                entry.last_broadcast_at = Some(Instant::now());
                entry.last_count = Some(audience_count);
                true
            }
        };

        if should_broadcast {
            self.call_broadcast_audience_update(channel_id).await?;
        }

        Ok(())
    }

    async fn call_broadcast_audience_update(&self, channel_id: ChannelId) -> Result<()> {
        let Some(handle) = self.call_get(channel_id) else {
            return Ok(());
        };

        self.state
            .messaging()
            .broadcast_channel(
                channel_id,
                MessageSync::CallUpdate {
                    call: handle.call.clone(),
                },
            )
            .await
    }

    /// update a call
    pub async fn call_update(&self, channel_id: ChannelId, patch: CallPatch) -> Result<CallHandle> {
        let mut entry = self
            .calls
            .get_mut(&channel_id)
            .ok_or_else(|| ApiError::from_code(ErrorCode::UnknownVoiceChannel))?;

        let handle = entry.value();
        let mut new_call = handle.call.clone();
        new_call.apply_patch(patch);

        // PERF: dont clone to update calls (arc-swap?)
        let updated_handle = Arc::new(CallHandleInner {
            call: new_call.clone(),
            sfus: handle.sfus.clone(),
            cleanup_task: handle.cleanup_task.clone(),
            voice_states: handle.voice_states.clone(),
            message_id: handle.message_id,
        });

        *entry.value_mut() = Arc::clone(&updated_handle);

        self.state
            .messaging()
            .broadcast_channel(channel_id, MessageSync::CallUpdate { call: new_call })
            .await?;

        Ok(updated_handle)
    }

    /// disconnect everyone in a call
    ///
    /// returns number of voice states disconnected
    // NOTE: maybe don't make this async?
    pub async fn call_disconnect_all(&self, channel_id: ChannelId) -> Result<u64> {
        let srv = self.state.services();
        let states = srv.voice.state_list_by_channel(channel_id);
        let count = states.len() as u64;

        for handle in states {
            let user_id = handle.inner().user_id;
            srv.voice.state_destroy(channel_id, user_id).await?;
        }

        Ok(count)
    }

    /// disconnect all voice states belonging to a user
    ///
    /// returns number of voice states disconnected
    pub async fn call_disconnect_all_user(
        &self,
        channel_id: ChannelId,
        user_id: UserId,
    ) -> Result<u64> {
        let srv = self.state.services();
        let states = srv.voice.state_list_by_user(user_id);
        let mut count = 0;

        for handle in states {
            if handle.inner().channel_id == channel_id {
                let user_id = handle.inner().user_id;
                srv.voice.state_destroy(channel_id, user_id).await?;
                count += 1;
            }
        }

        Ok(count)
    }

    /// restart a call's cleanup task timer
    pub fn call_bump(&self, channel_id: ChannelId) {
        // PERF: maybe `.remove()` instead to avoid cloning?
        if let Some(mut entry) = self.calls.get_mut(&channel_id) {
            let handle = entry.value();
            if let Some(task) = &handle.cleanup_task {
                task.abort();

                let new_cleanup_task = self.spawn_cleanup_task(channel_id);

                let updated_handle = Arc::new(CallHandleInner {
                    call: handle.call.clone(),
                    sfus: handle.sfus.clone(),
                    cleanup_task: Some(new_cleanup_task),
                    voice_states: handle.voice_states.clone(),
                    message_id: handle.message_id,
                });

                *entry.value_mut() = updated_handle;
            }
        }
    }

    // TODO: use this for broadcast channel calls
    // calls in all other channels should be deleted as soon as the last person disconnects
    fn spawn_cleanup_task(&self, channel_id: ChannelId) -> tokio::task::AbortHandle {
        let state = self.state.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(CALL_CLEANUP_TIMEOUT).await;

                // keep looping until there are no voice states
                if state.services().voice.call_delete(channel_id, false).await {
                    break;
                }
            }
        })
        .abort_handle()
    }
}
