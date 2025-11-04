use std::sync::Arc;

use common::v1::types::voice::{SfuCommand, SfuPermissions, VoiceState};
use common::v1::types::UserId;
use common::v1::types::{ChannelId, SfuId};
use dashmap::DashMap;
use tracing::error;

use crate::{Error, Result, ServerStateInner};

pub struct ServiceVoice {
    state: Arc<ServerStateInner>,
    voice_states: DashMap<UserId, VoiceState>,

    // TODO: make this not public
    pub sfus: DashMap<SfuId, ()>,
    pub channel_to_sfu: DashMap<ChannelId, SfuId>,
}

impl ServiceVoice {
    pub fn new(state: Arc<ServerStateInner>) -> Self {
        Self {
            state,
            voice_states: DashMap::new(),
            sfus: DashMap::new(),
            channel_to_sfu: DashMap::new(),
        }
    }

    pub fn state_put(&self, state: VoiceState) {
        self.voice_states.insert(state.user_id, state);
    }

    pub fn state_remove(&self, user_id: &UserId) {
        self.voice_states.remove(user_id);
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

    pub fn disconnect_everyone(&self, channel_id: ChannelId) -> Result<()> {
        for s in &self.voice_states {
            if s.thread_id == channel_id {
                let r = self.state.sushi_sfu.send(SfuCommand::VoiceState {
                    user_id: s.user_id,
                    state: None,
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
        self.voice_states.retain(|_, s| s.thread_id != channel_id);
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
                .sushi_sfu
                .send(SfuCommand::Thread {
                    thread: channel.into(),
                })
                .unwrap();
            Ok(*chosen)
        } else {
            error!("no available sfu");
            Err(Error::BadStatic("no available sfu"))
        }
    }
}
