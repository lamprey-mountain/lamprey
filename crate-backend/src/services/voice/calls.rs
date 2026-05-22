use crate::services::voice::ServiceVoice;
use crate::Result;
use common::v1::types::{
    voice::{Call, CallCreate, CallPatch},
    ChannelId, SfuId, UserId,
};
use dashmap::DashSet;
use std::{sync::Arc, time::Duration};

pub struct CallHandleInner {
    pub call: Call,
    pub sfus: DashSet<SfuId>,
    pub cleanup_task: tokio::task::AbortHandle,
}

pub type CallHandle = Arc<CallHandleInner>;

impl CallHandleInner {
    pub fn id(&self) -> ChannelId {
        self.call.channel_id
    }

    pub fn call(&self) -> &Call {
        &self.call
    }
}

impl ServiceVoice {
    /// get a call
    pub fn call_get(&self, channel_id: ChannelId) -> Option<CallHandle> {
        self.calls.get(&channel_id).map(|c| c.value().clone())
    }

    /// create a call
    pub fn call_create(&self, params: CallCreate) -> Result<CallHandle> {
        // 1. return (and bump) existing call if it exists
        // 2. create a call
        // 3. insert handle
        // 4. start cleanup task
        todo!()
    }

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
    }

    /// update a call
    pub fn call_update(&self, channel_id: ChannelId, patch: CallPatch) -> Result<CallHandle> {
        // 1. update call topic
        // 2. update callhandle
        // 3. send sync event
        todo!()
    }

    /// disconnect everyone in a call
    ///
    /// returns number of voice states disconnected
    pub async fn call_disconnect_all(&self, channel_id: ChannelId) -> Result<u64> {
        todo!()
    }

    /// disconnect all voice states belonging to a user
    ///
    /// returns number of voice states disconnected
    pub async fn call_disconnect_all_user(
        &self,
        channel_id: ChannelId,
        user_id: UserId,
    ) -> Result<u64> {
        todo!()
    }

    /// restart a call's cleanup task timer
    pub fn call_bump(&self, channel_id: ChannelId) {
        if let Some(mut entry) = self.calls.get_mut(&channel_id) {
            let handle = entry.value();
            handle.cleanup_task.abort();

            let new_cleanup_task = self.spawn_cleanup_task(channel_id);

            let updated_handle = Arc::new(CallHandleInner {
                call: handle.call.clone(),
                sfus: handle.sfus.clone(),
                cleanup_task: new_cleanup_task,
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
