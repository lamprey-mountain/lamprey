use crate::Result;
use crate::services::voice::ServiceVoice;
use crate::services::voice::voice_state::VoiceStateHandle;
use common::v1::types::MessageSync;
use common::v1::types::error::{ApiError, ErrorCode};
use common::v1::types::voice::CallMetadata;
use common::v1::types::{
    ChannelId, SfuId, UserId,
    util::Time,
    voice::{Call, CallCreate, CallPatch},
};
use dashmap::{DashMap, DashSet};
use std::{sync::Arc, time::Duration};

pub struct CallHandleInner {
    pub call: Call,
    pub sfus: DashSet<SfuId>,
    pub cleanup_task: tokio::task::AbortHandle,
    pub voice_states: DashMap<UserId, VoiceStateHandle>,
}

pub type CallHandle = Arc<CallHandleInner>;

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

    /// create a call
    // TODO: make sure that calls are automatically created when a voice state is created/updated for a channel for the first time
    pub async fn call_create(
        &self,
        channel_id: ChannelId,
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
                audience_count: Some(0),
            },
        };

        // 3. insert handle
        let handle = Arc::new(CallHandleInner {
            call,
            sfus: DashSet::new(),
            cleanup_task: self.spawn_cleanup_task(channel_id),
            voice_states: DashMap::new(),
        });

        self.calls.insert(channel_id, Arc::clone(&handle));

        Ok(handle)
    }

    // TEMP: old code for reference
    // pub async fn call_create(&self, params: CallCreate) -> Result<()> {
    //     let channel = self
    //         .state
    //         .services()
    //         .channels
    //         .get(params.channel_id, None)
    //         .await?;
    //
    //     let room_id = channel.room_id;
    //     let call = Call {
    //         room_id,
    //         channel_id: params.channel_id,
    //         topic: params.topic,
    //         created_at: Time::now_utc(),
    //     };
    //     self.calls.insert(params.channel_id, call.clone());
    //
    //     let _ = self.state.broadcast(MessageSync::CallCreate { call });
    //
    //     let has_voice_states = self
    //         .voice_states
    //         .iter()
    //         .any(|s| s.channel_id == params.channel_id);
    //
    //     if !has_voice_states {
    //         self.spawn_call_cleanup(params.channel_id);
    //     }
    //
    //     Ok(())
    // }

    /// delete a call
    ///
    /// by default, this will not delete calls with members still in it. pass `force = true` to disconnect everyone first.
    ///
    /// returns true if this call was deleted
    pub fn call_delete(&self, channel_id: ChannelId, _force: bool) -> bool {
        // FIXME: handle force = true (disconnect everyone)
        // FIXME: handle force = false (don't delete if there are members)

        if let Some((_, handle)) = self.calls.remove(&channel_id) {
            handle.cleanup_task.abort();
        }

        true

        // === old code===
        // if force {
        //     let _ = self.disconnect_everyone(channel_id).await;
        // }
        // self.calls.remove(&channel_id);
        //
        // if let Some((_, handle)) = self.cleanup_tasks.remove(&channel_id) {
        //     handle.abort();
        // }
        //
        // let _ = self.state.broadcast(MessageSync::CallDelete { channel_id });
    }

    /// update a call
    pub fn call_update(&self, channel_id: ChannelId, patch: CallPatch) -> Result<CallHandle> {
        let mut entry = self
            .calls
            .get_mut(&channel_id)
            .ok_or_else(|| ApiError::from_code(ErrorCode::UnknownVoiceChannel))?;

        let handle = entry.value();
        let mut new_call = handle.call.clone();
        new_call.apply_patch(patch);

        let updated_handle = Arc::new(CallHandleInner {
            call: new_call.clone(),
            sfus: handle.sfus.clone(),
            cleanup_task: handle.cleanup_task.clone(),
            voice_states: handle.voice_states.clone(),
        });

        *entry.value_mut() = Arc::clone(&updated_handle);

        let _ = self
            .state
            .broadcast(MessageSync::CallUpdate { call: new_call });

        Ok(updated_handle)
    }

    /// disconnect everyone in a call
    ///
    /// returns number of voice states disconnected
    pub async fn call_disconnect_all(&self, channel_id: ChannelId) -> Result<u64> {
        let srv = self.state.services();
        let states = srv.voice.state_list_by_channel(channel_id);
        let count = states.len() as u64;

        for handle in states {
            let user_id = handle.inner().user_id;
            srv.voice.state_destroy(channel_id, user_id)?;
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
                srv.voice.state_destroy(channel_id, user_id)?;
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
            handle.cleanup_task.abort();

            let new_cleanup_task = self.spawn_cleanup_task(channel_id);

            let updated_handle = Arc::new(CallHandleInner {
                call: handle.call.clone(),
                sfus: handle.sfus.clone(),
                cleanup_task: new_cleanup_task,
                voice_states: handle.voice_states.clone(),
            });

            *entry.value_mut() = updated_handle;
        }
    }

    fn spawn_cleanup_task(&self, channel_id: ChannelId) -> tokio::task::AbortHandle {
        let state = self.state.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(300)).await;

                // keep looping until there are no voice states
                if state.services().voice.call_delete(channel_id, false) {
                    break;
                }
            }
        })
        .abort_handle()
    }
}
