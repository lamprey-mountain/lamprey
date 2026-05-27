use common::v1::types::voice::messages::{SfuCommand, SignallingCommand};
use lamprey_backend_core::Error;
use std::sync::Arc;

use crate::services::voice::ServiceVoice;
use crate::Result;
use common::v1::types::util::Time;
use common::v1::types::voice::{
    CallCreate, VoiceState, VoiceStateScreenshareUpdate, VoiceStateUpdate,
};
use common::v1::types::{ChannelId, MessageSync, SfuId, UserId};

pub struct VoiceStateHandleInner {
    pub inner: VoiceState,
    pub state: VoiceStateState,
    pub sfu_id: SfuId,
}

pub type VoiceStateHandle = Arc<VoiceStateHandleInner>;

pub enum VoiceStateState {
    Connecting,
    Connected,
}

impl VoiceStateHandleInner {
    /// get the underlying voice state
    pub fn inner(&self) -> &VoiceState {
        &self.inner
    }

    /// get the current id of the sfu this user is on
    pub fn sfu_id(&self) -> SfuId {
        self.sfu_id
    }
}

impl ServiceVoice {
    /// create a new voice state
    pub async fn state_create(
        &self,
        user_id: UserId,
        update: VoiceStateUpdate,
    ) -> Result<VoiceStateHandle> {
        let srv = self.state.services();

        // permission checks
        let user = srv.users.get(user_id, Some(user_id)).await?;
        user.ensure_unsuspended()?;

        srv.perms
            .for_channel3(Some(user_id), update.channel_id)
            .await?
            .ensure_view()?
            .needs_unlocked()
            .check()?;

        let chan = srv.channels.get(update.channel_id, Some(user_id)).await?;
        chan.ensure_unarchived()?;
        chan.ensure_unremoved()?;

        // TODO: handle existing state
        // let old_state = srv.voice.state_get(user_id);

        // create call if it doesn't exist
        let call = if let Some(call) = self.call_get(update.channel_id) {
            call
        } else {
            self.call_create(CallCreate {
                channel_id: update.channel_id,
                topic: None,
            })
            .await?
        };

        // find sfu
        let sfu = self.sfu_alloc(update.channel_id, user_id)?;
        let sfu_id = sfu.id();

        // build voice state
        let state = VoiceState {
            user_id,
            channel_id: update.channel_id,
            session_id: None,    // TODO: populate
            connection_id: None, // TODO: populate
            joined_at: Time::now_utc(),
            mute: false, // TODO: populate from room member
            deaf: false, // TODO: populate from room member
            self_mute: update.self_mute,
            self_deaf: update.self_deaf,
            self_video: update.self_video,
            screenshare: None,
            suppress: false, // TODO: suppress by default in broadcast channels or the afk channel
            requested_to_speak_at: None,
        };

        let handle = Arc::new(VoiceStateHandleInner {
            inner: state,
            state: VoiceStateState::Connected,
            sfu_id,
        });
        call.voice_states.insert(user_id, Arc::clone(&handle));

        sfu.send(SfuCommand::Signalling {
            user_id,
            channel_id: todo!(),
            inner: SignallingCommand::VoiceState { state: update },
        });

        self.state.broadcast(MessageSync::VoiceState {
            user_id,
            state: Some(handle.inner.clone()),
            old_state: None,
        })?;

        Ok(handle)
    }

    /// update a voice state by user_id
    pub fn state_update(&self, user_id: UserId, update: VoiceStateUpdate) -> Result<()> {
        let call = self
            .call_get(update.channel_id)
            .ok_or_else(|| Error::BadStatic("call not found"))?;

        let mut entry = call
            .voice_states
            .get_mut(&user_id)
            .ok_or_else(|| Error::BadStatic("voice state not found"))?;

        let handle = entry.value();
        let old_state = handle.inner.clone();
        let sfu_id = handle.sfu_id;

        let mut new_inner = handle.inner.clone();
        new_inner.self_mute = update.self_mute;
        new_inner.self_deaf = update.self_deaf;
        new_inner.self_video = update.self_video;

        let new_handle = Arc::new(VoiceStateHandleInner {
            inner: new_inner,
            state: VoiceStateState::Connected,
            sfu_id: handle.sfu_id,
        });

        // replace in map
        *entry.value_mut() = Arc::clone(&new_handle);

        // notify sfu
        if let Some(sfu) = self.sfu_get(sfu_id) {
            sfu.send(SfuCommand::Signalling {
                user_id,
                channel_id: todo!(),
                inner: SignallingCommand::VoiceState { state: update },
            });
        }

        // broadcast sync
        self.state.broadcast(MessageSync::VoiceState {
            user_id: new_handle.inner.user_id,
            state: Some(new_handle.inner.clone()),
            old_state: Some(old_state),
        })?;

        Ok(())
    }

    /// replace a voice state
    // TODO: remove this? and force all updates to go through state_update? having two update functions duplicates logic.
    pub fn state_replace(&self, state: VoiceState) -> Result<()> {
        let call = self
            .call_get(state.channel_id)
            .ok_or_else(|| Error::BadStatic("call not found"))?;

        let mut entry = call
            .voice_states
            .get_mut(&state.user_id)
            .ok_or_else(|| Error::BadStatic("voice state not found"))?;

        let handle = entry.value();
        let old_state = handle.inner.clone();
        let sfu_id = handle.sfu_id;
        let user_id = handle.inner.user_id;

        let new_handle = Arc::new(VoiceStateHandleInner {
            inner: state.clone(),
            state: VoiceStateState::Connected,
            sfu_id: handle.sfu_id,
        });

        *entry.value_mut() = Arc::clone(&new_handle);

        if let Some(sfu) = self.sfu_get(sfu_id) {
            sfu.send(SfuCommand::Signalling {
                user_id,
                channel_id: todo!(),
                inner: SignallingCommand::VoiceState {
                    state: VoiceStateUpdate {
                        channel_id: state.channel_id,
                        self_deaf: state.self_deaf,
                        self_mute: state.self_mute,
                        self_video: state.self_video,
                        screenshare: Some(state.screenshare.as_ref().map(|s| {
                            VoiceStateScreenshareUpdate {
                                thumbnail: s.thumbnail,
                            }
                        })),
                    },
                },
            });
        }

        self.state.broadcast(MessageSync::VoiceState {
            user_id: state.user_id,
            state: Some(state.clone()),
            old_state: Some(old_state),
        })?;

        Ok(())
    }

    /// destroy (disconnect) a voice state
    pub fn state_destroy(&self, channel_id: ChannelId, user_id: UserId) -> Result<()> {
        let call = self
            .call_get(channel_id)
            .ok_or_else(|| Error::BadStatic("call state not found"))?;

        let Some((_, handle)) = call.voice_states.remove(&user_id) else {
            return Ok(());
        };

        // notify sfu
        if let Some(sfu) = self.sfu_get(handle.sfu_id) {
            sfu.send(SfuCommand::Signalling {
                user_id,
                channel_id,
                inner: SignallingCommand::Disconnect,
            });
        }

        // broadcast removal
        self.state.broadcast(MessageSync::VoiceState {
            user_id: handle.inner.user_id,
            state: None,
            old_state: Some(handle.inner.clone()),
        })?;

        Ok(())
    }

    /// get a voice state by channel_id and user_id
    pub fn state_get(&self, channel_id: ChannelId, user_id: UserId) -> Option<VoiceStateHandle> {
        self.call_get(channel_id).and_then(|call| {
            call.voice_states
                .get(&user_id)
                .map(|s| Arc::clone(s.value()))
        })
    }

    /// temp(?): list **all** known voice states
    pub fn state_list(&self) -> Vec<VoiceStateHandle> {
        self.calls
            .iter()
            .flat_map(|entry| {
                entry
                    .value()
                    .voice_states
                    .iter()
                    .map(|s| Arc::clone(s.value()))
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    /// list all voice states by user
    pub fn state_list_by_user(&self, user_id: UserId) -> Vec<VoiceStateHandle> {
        self.calls
            .iter()
            .flat_map(|call| {
                call.value()
                    .voice_states
                    .get(&user_id)
                    .map(|s| Arc::clone(s.value()))
            })
            .collect()
    }

    /// list all voice states by channel
    pub fn state_list_by_channel(&self, channel_id: ChannelId) -> Vec<VoiceStateHandle> {
        self.call_get(channel_id)
            .map(|call| {
                call.voice_states
                    .iter()
                    .map(|s| Arc::clone(s.value()))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// list all voice states by sfu
    pub fn state_list_by_sfu(&self, sfu_id: SfuId) -> Vec<VoiceStateHandle> {
        self.calls
            .iter()
            .flat_map(|entry| {
                entry
                    .value()
                    .voice_states
                    .iter()
                    .filter(|s| s.value().sfu_id == sfu_id)
                    .map(|s| Arc::clone(s.value()))
                    .collect::<Vec<_>>()
            })
            .collect()
    }
}
