use common::v1::types::voice::messages::{SfuCommand, SignallingCommand};
use lamprey_backend_core::Error;
use std::sync::Arc;

use crate::services::voice::ServiceVoice;
use crate::Result;
use common::v1::types::util::Time;
use common::v1::types::voice::{VoiceState, VoiceStateStreamUpdate, VoiceStateUpdate};
use common::v1::types::{ChannelId, MessageSync, PeerId, SfuId, UserId};

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

        // TODO: create call if it doesn't exist

        // find sfu
        let peer_id = PeerId::new();
        let sfu = self.sfu_alloc(update.channel_id, peer_id)?;
        let sfu_id = sfu.id();

        // build voice state
        let state = VoiceState {
            peer_id,
            user_id,
            channel_id: update.channel_id,
            session_id: None, // TODO: populate
            joined_at: Time::now_utc(),
            mute: false,
            deaf: false,
            self_mute: update.self_mute,
            self_deaf: update.self_deaf,
            self_video: update.self_video,
            screenshare: None,
            suppress: false, // TODO: suppress by default in broadcast rooms
            requested_to_speak_at: None,
        };

        let handle = Arc::new(VoiceStateHandleInner {
            inner: state,
            state: VoiceStateState::Connected,
            sfu_id,
        });
        self.voice_states.insert(peer_id, Arc::clone(&handle));

        sfu.send(SfuCommand::Signalling {
            peer_id: Some(peer_id),
            inner: SignallingCommand::VoiceState { state: update },
        });

        self.state.broadcast(MessageSync::VoiceState {
            user_id,
            peer_id,
            state: Some(handle.inner.clone()),
            old_state: None,
        })?;

        Ok(handle)
    }

    /// update a voice state
    pub fn state_update(&self, peer_id: PeerId, update: VoiceStateUpdate) -> Result<()> {
        let mut entry = self
            .voice_states
            .get_mut(&peer_id)
            .ok_or_else(|| Error::BadStatic("voice state not found"))?;

        let handle = entry.value();
        let old_state = handle.inner.clone();
        let sfu_id = handle.sfu_id;

        let mut new_inner = handle.inner.clone();
        new_inner.self_mute = update.self_mute;
        new_inner.self_deaf = update.self_deaf;
        new_inner.self_video = update.self_video;
        // TODO: handle channel_id updates
        // TODO: create call if it doesn't exist

        let new_handle = Arc::new(VoiceStateHandleInner {
            inner: new_inner,
            state: VoiceStateState::Connected,
            sfu_id: handle.sfu_id,
        });

        // 3. replace in map
        *entry.value_mut() = Arc::clone(&new_handle);

        // 4. notify sfu
        if let Some(sfu) = self.sfu_get(sfu_id) {
            sfu.send(SfuCommand::Signalling {
                peer_id: Some(peer_id),
                inner: SignallingCommand::VoiceState { state: update },
            });
        }

        // 5. broadcast sync
        self.state.broadcast(MessageSync::VoiceState {
            user_id: new_handle.inner.user_id,
            peer_id,
            state: Some(new_handle.inner.clone()),
            old_state: Some(old_state),
        })?;

        Ok(())
    }

    /// replace a voice state
    // TODO: remove this? and force all updates to go through state_update? having two update functions duplicates logic.
    pub fn state_replace(&self, state: VoiceState) -> Result<()> {
        let mut entry = self
            .voice_states
            .get_mut(&state.peer_id)
            .ok_or_else(|| Error::BadStatic("voice state not found"))?;

        let handle = entry.value();
        let old_state = handle.inner.clone();
        let sfu_id = handle.sfu_id;

        let new_handle = Arc::new(VoiceStateHandleInner {
            inner: state.clone(),
            state: VoiceStateState::Connected,
            sfu_id: handle.sfu_id,
        });

        // TODO: handle channel_id updates
        // TODO: create call if it doesn't exist

        *entry.value_mut() = Arc::clone(&new_handle);

        if let Some(sfu) = self.sfu_get(sfu_id) {
            sfu.send(SfuCommand::Signalling {
                peer_id: Some(state.peer_id),
                inner: SignallingCommand::VoiceState {
                    state: VoiceStateUpdate {
                        channel_id: state.channel_id,
                        self_deaf: state.self_deaf,
                        self_mute: state.self_mute,
                        self_video: state.self_video,
                        screenshare: state.screenshare.as_ref().map(|s| VoiceStateStreamUpdate {
                            thumbnail: s.thumbnail,
                        }),
                    },
                },
            });
        }

        self.state.broadcast(MessageSync::VoiceState {
            user_id: state.user_id,
            peer_id: state.peer_id,
            state: Some(state.clone()),
            old_state: Some(old_state),
        })?;

        Ok(())
    }

    /// destroy (disconnect) a voice state
    pub fn state_destroy(&self, peer_id: PeerId) -> Result<()> {
        let Some((_, handle)) = self.voice_states.remove(&peer_id) else {
            return Ok(());
        };

        // notify sfu
        if let Some(sfu) = self.sfu_get(handle.sfu_id) {
            sfu.send(SfuCommand::Signalling {
                peer_id: Some(peer_id),
                inner: SignallingCommand::Disconnect,
            });
        }

        // broadcast removal
        self.state.broadcast(MessageSync::VoiceState {
            user_id: handle.inner.user_id,
            peer_id,
            state: None,
            old_state: Some(handle.inner.clone()),
        })?;

        Ok(())
    }

    /// get a voice state
    pub fn state_get(&self, peer_id: PeerId) -> Option<VoiceStateHandle> {
        self.voice_states
            .get(&peer_id)
            .map(|s| Arc::clone(s.value()))
    }

    /// temp(?): list **all** voice states
    pub fn state_list(&self) -> Vec<VoiceStateHandle> {
        self.voice_states
            .iter()
            .map(|s| Arc::clone(s.value()))
            .collect()
    }

    /// temp(?): list **all** voice states by user
    pub fn state_list_by_user(&self, user_id: UserId) -> Vec<VoiceStateHandle> {
        self.voice_states
            .iter()
            .filter(|s| s.inner.user_id == user_id)
            .map(|s| Arc::clone(s.value()))
            .collect()
    }

    /// temp(?): list **all** voice states by channel
    pub fn state_list_by_channel(&self, channel_id: ChannelId) -> Vec<VoiceStateHandle> {
        self.voice_states
            .iter()
            .filter(|s| s.inner.channel_id == channel_id)
            .map(|s| Arc::clone(s.value()))
            .collect()
    }

    /// temp(?): list **all** voice states by sfu
    pub fn state_list_by_sfu(&self, sfu_id: SfuId) -> Vec<VoiceStateHandle> {
        self.voice_states
            .iter()
            .filter(|s| s.sfu_id == sfu_id)
            .map(|s| Arc::clone(s.value()))
            .collect()
    }
}
