use std::sync::Arc;
use std::time::Duration;

use common::v1::types::sync::MessageSync;
use common::v1::types::util::Time;
use common::v1::types::voice::{
    Call, CallCreate, CallPatch, SfuCommand, SfuPermissions, VoiceState,
};
use common::v1::types::{ChannelId, ChannelType, SfuId, UserId};
use dashmap::DashMap;
use tokio::time::sleep;
use tracing::error;

use crate::consts::EMPTY_CALL_TIMEOUT;
use crate::{Error, Result, ServerStateInner};

pub struct ServiceVoice {
    state: Arc<ServerStateInner>,
    voice_states: DashMap<UserId, VoiceState>,
    calls: DashMap<ChannelId, Call>,
    cleanup_tasks: DashMap<ChannelId, tokio::task::AbortHandle>,

    // TODO: make this not public
    pub sfus: DashMap<SfuId, ()>,
    pub channel_to_sfu: DashMap<ChannelId, SfuId>,
}

impl ServiceVoice {
    pub fn new(state: Arc<ServerStateInner>) -> Self {
        Self {
            state,
            voice_states: DashMap::new(),
            calls: DashMap::new(),
            cleanup_tasks: DashMap::new(),
            sfus: DashMap::new(),
            channel_to_sfu: DashMap::new(),
        }
    }

    pub async fn state_put(&self, state: VoiceState) {
        self.voice_states.insert(state.user_id, state.clone());

        if !self.calls.contains_key(&state.channel_id) {
            let channel = self
                .state
                .services()
                .channels
                .get(state.channel_id, None)
                .await;
            if let Ok(channel) = channel {
                let room_id = channel.room_id;
                self.calls.insert(
                    state.channel_id,
                    Call {
                        room_id,
                        channel_id: state.channel_id,
                        topic: None,
                        created_at: Time::now_utc(),
                    },
                );
            }
        }
    }

    pub async fn state_remove(&self, user_id: &UserId) {
        if let Some((_, state)) = self.voice_states.remove(user_id) {
            let channel_id = state.channel_id;

            let still_connected: Vec<_> = self
                .voice_states
                .iter()
                .filter(|s| s.channel_id == channel_id)
                .collect();

            if still_connected.is_empty() {
                let channel = self.state.services().channels.get(channel_id, None).await;
                if let Ok(channel) = channel {
                    match channel.ty {
                        ChannelType::Voice => {
                            self.calls.remove(&channel_id);
                        }
                        ChannelType::Dm | ChannelType::Gdm | ChannelType::Broadcast => {
                            self.spawn_call_cleanup(channel_id);
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    pub fn state_get(&self, user_id: UserId) -> Option<VoiceState> {
        self.voice_states.get(&user_id).map(|s| s.to_owned())
    }

    pub fn state_list(&self) -> Vec<VoiceState> {
        self.voice_states
            .iter()
            .map(|r| r.value().clone())
            .collect()
    }

    pub async fn disconnect_everyone(&self, channel_id: ChannelId) -> Result<()> {
        for s in &self.voice_states {
            if s.channel_id == channel_id {
                let r = self.state.broadcast_sfu(SfuCommand::VoiceState {
                    user_id: s.user_id,
                    state: None,
                    // FIXME: permissions
                    permissions: SfuPermissions {
                        speak: false,
                        video: false,
                        priority: false,
                    },
                });
                if let Err(err) = r {
                    error!("failed to disconnect user from thread: {err}");
                }
            }
        }
        self.voice_states.retain(|_, s| s.channel_id != channel_id);
        Ok(())
    }

    /// select the "best" sfu and pair it with this thread id. return the existing sfu id if it exists.
    ///
    /// currently "best" means the sfu with least load in terms of # of threads using it
    pub async fn alloc_sfu(&self, channel_id: ChannelId) -> Result<SfuId> {
        if let Some(existing) = self.channel_to_sfu.get(&channel_id) {
            return Ok(*existing);
        }

        let sfu_channel_counts = DashMap::<SfuId, u64>::new();
        for i in &self.sfus {
            sfu_channel_counts.insert(*i.key(), 0);
        }
        for i in &self.channel_to_sfu {
            *sfu_channel_counts.get_mut(i.value()).unwrap() += 1;
        }
        let mut sorted: Vec<_> = sfu_channel_counts.into_iter().collect();
        sorted.sort_by_key(|(_, count)| *count);
        if let Some((chosen, _)) = sorted.first() {
            self.channel_to_sfu.insert(channel_id, *chosen);
            let channel = self.state.services().channels.get(channel_id, None).await?;
            self.state
                .broadcast_sfu(SfuCommand::Channel {
                    channel: channel.into(),
                })
                .unwrap();
            Ok(*chosen)
        } else {
            error!("no available sfu");
            Err(Error::BadStatic("no available sfu"))
        }
    }

    pub fn call_get(&self, channel_id: ChannelId) -> Result<Call> {
        self.calls
            .get(&channel_id)
            .map(|s| s.clone())
            .ok_or(Error::NotFound)
    }

    pub async fn call_create(&self, params: CallCreate) -> Result<()> {
        let channel = self
            .state
            .services()
            .channels
            .get(params.channel_id, None)
            .await?;

        let room_id = channel.room_id;
        let call = Call {
            room_id,
            channel_id: params.channel_id,
            topic: params.topic,
            created_at: Time::now_utc(),
        };
        self.calls.insert(params.channel_id, call.clone());

        let _ = self.state.broadcast(MessageSync::CallCreate { call });

        let has_voice_states = self
            .voice_states
            .iter()
            .any(|s| s.channel_id == params.channel_id);

        if !has_voice_states {
            self.spawn_call_cleanup(params.channel_id);
        }

        Ok(())
    }

    pub async fn call_delete(&self, channel_id: ChannelId, force: bool) {
        if force {
            let _ = self.disconnect_everyone(channel_id).await;
        }
        self.calls.remove(&channel_id);

        if let Some((_, handle)) = self.cleanup_tasks.remove(&channel_id) {
            handle.abort();
        }

        let _ = self.state.broadcast(MessageSync::CallDelete { channel_id });
    }

    pub fn call_update(&self, channel_id: ChannelId, patch: CallPatch) -> Result<()> {
        let call_opt = self.calls.get(&channel_id).map(|c| c.clone());
        if let Some(call) = call_opt {
            let updated_call = Call {
                topic: patch.topic.and_then(|t| t),
                ..call.clone()
            };
            self.calls.insert(channel_id, updated_call.clone());
            let _ = self
                .state
                .broadcast(MessageSync::CallUpdate { call: updated_call });
        }

        Ok(())
    }

    pub fn spawn_call_cleanup(&self, channel_id: ChannelId) {
        if self.cleanup_tasks.contains_key(&channel_id) {
            return;
        }

        let state = self.state.clone();
        let channel_id_copy = channel_id;

        let handle = tokio::spawn(async move {
            sleep(Duration::from_secs(EMPTY_CALL_TIMEOUT)).await;

            state
                .services()
                .voice
                .call_delete(channel_id_copy, false)
                .await;
        })
        .abort_handle();

        self.cleanup_tasks.insert(channel_id, handle);
    }
}
