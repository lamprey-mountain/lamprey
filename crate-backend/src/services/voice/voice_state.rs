use common::v1::types::voice::messages::{SfuCommand, SignallingCommand, SignallingEvent};
use lamprey_backend_core::Error;
use std::sync::Arc;

use crate::services::voice::ServiceVoice;
use crate::Result;
use common::v1::types::util::Time;
use common::v1::types::voice::{CallCreate, VoiceState, VoiceStateUpdate};
use common::v1::types::{
    ChannelId, ChannelType, ConnectionId, MessageSync, SessionId, SfuId, UserId,
};

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
        session_id: Option<SessionId>,
        connection_id: Option<ConnectionId>,
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

        let old_states = srv.voice.state_list_by_user(user_id);
        if user.bot {
            // TODO: remove existing voice state for channel if it exists
        } else {
            // TODO: remove all existing voice states
            // self.state_destroy();
        }

        // create call if it doesn't exist
        let call = if let Some(call) = self.call_get(update.channel_id) {
            call
        } else {
            self.call_create(update.channel_id, CallCreate { topic: None })
                .await?
        };

        // find sfu
        let sfu = self.sfu_alloc(update.channel_id, user_id).await?;
        let sfu_id = sfu.id();

        let mut mute = false;
        let mut deaf = false;
        let mut suppress = false;

        if let Some(room_id) = chan.room_id {
            let room = srv.rooms.load_room(room_id, true).await?;
            if let Some(member) = room.get_member(&user_id) {
                mute = member.member.mute;
                deaf = member.member.deaf;
            }

            if let Some(afk_id) = room.get_data().unwrap().room.afk_channel_id {
                if update.channel_id == afk_id {
                    suppress = true;
                }
            }

            if chan.ty == ChannelType::Broadcast {
                suppress = true;
            }
        }

        // build voice state
        let state = VoiceState {
            user_id,
            channel_id: update.channel_id,
            session_id,
            connection_id,
            joined_at: Time::now_utc(),
            mute,
            deaf,
            self_mute: update.self_mute,
            self_deaf: update.self_deaf,
            self_video: update.self_video,
            screenshare: None,
            suppress,
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
            channel_id: update.channel_id,
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

        if new_inner.channel_id != update.channel_id {
            // TODO: handle this
        }

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
                channel_id: update.channel_id,
                inner: SignallingCommand::VoiceState { state: update },
            });
        }

        // broadcast sync
        self.state.broadcast(MessageSync::VoiceState {
            user_id: new_handle.inner.user_id,
            state: Some(new_handle.inner.clone()),
            old_state: Some(old_state),
        })?;

        // TODO: if nobody is connected to the old channel anymore, spawn timeout task to clean up the call?

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
        self.state.broadcast(MessageSync::VoiceDispatch {
            user_id: handle.inner.user_id,
            channel_id,
            payload: SignallingEvent::Disconnected,
        })?;
        self.state.broadcast(MessageSync::VoiceState {
            user_id: handle.inner.user_id,
            state: None,
            old_state: Some(handle.inner.clone()),
        })?;

        // TODO: if nobody is connected anymore, spawn timeout task to clean up the call?

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
