use crate::services::voice::calls::CallHandle;
use crate::services::voice::sfus::SfuHandle;
use crate::services::voice::voice_state::VoiceStateHandle;
use crate::ServerStateInner;
use common::v1::types::voice::internal::SfuStats;
use common::v1::types::voice::messages::SfuCommand;
use common::v1::types::voice::router::{VoiceRouter, VoiceRouterConfig};
use common::v1::types::{ChannelId, PeerId, SfuId};
use dashmap::DashMap;
use std::sync::Arc;

pub mod calls;
// pub mod ring;
pub mod sfus;
pub mod voice_state;

pub struct ServiceVoice {
    pub state: Arc<ServerStateInner>,
    pub voice_states: DashMap<PeerId, VoiceStateHandle>,
    pub calls: DashMap<ChannelId, CallHandle>,
    pub sfus: DashMap<SfuId, SfuHandle>,
    pub router: VoiceRouter,
}

impl ServiceVoice {
    pub fn new(state: Arc<ServerStateInner>) -> Self {
        Self {
            state,
            voice_states: DashMap::new(),
            calls: DashMap::new(),
            sfus: DashMap::new(),
            router: VoiceRouter::new(VoiceRouterConfig::default()),
        }
    }

    // TODO: ===== remove all code below =====

    // pub async fn state_put(&self, state: VoiceState) {
    //     self.voice_states.insert(state.user_id, state.clone());

    //     if !self.calls.contains_key(&state.channel_id) {
    //         let channel = self
    //             .state
    //             .services()
    //             .channels
    //             .get(state.channel_id, None)
    //             .await;
    //         if let Ok(channel) = channel {
    //             let room_id = channel.room_id;
    //             self.calls.insert(
    //                 state.channel_id,
    //                 Call {
    //                     room_id,
    //                     channel_id: state.channel_id,
    //                     topic: None,
    //                     created_at: Time::now_utc(),
    //                 },
    //             );
    //         }
    //     }
    // }

    // pub async fn state_remove(&self, user_id: &UserId) {
    //     if let Some((_, state)) = self.voice_states.remove(user_id) {
    //         let channel_id = state.channel_id;

    //         let still_connected: Vec<_> = self
    //             .voice_states
    //             .iter()
    //             .filter(|s| s.channel_id == channel_id)
    //             .collect();

    //         if still_connected.is_empty() {
    //             let channel = self.state.services().channels.get(channel_id, None).await;
    //             if let Ok(channel) = channel {
    //                 match channel.ty {
    //                     ChannelType::Voice => {
    //                         self.calls.remove(&channel_id);
    //                     }
    //                     ChannelType::Dm | ChannelType::Gdm | ChannelType::Broadcast => {
    //                         self.spawn_call_cleanup(channel_id);
    //                     }
    //                     _ => {}
    //                 }
    //             }
    //         }
    //     }
    // }

    // /// select the "best" sfu and pair it with this thread id. return the existing sfu id if it exists.
    // ///
    // /// currently "best" means the sfu with least load in terms of # of threads using it
    // pub async fn alloc_sfu(&self, channel_id: ChannelId) -> Result<SfuId> {
    //     if let Some(existing) = self.channel_to_sfu.get(&channel_id) {
    //         return Ok(*existing);
    //     }

    //     let sfu_channel_counts = DashMap::<SfuId, u64>::new();
    //     for i in &self.sfus {
    //         sfu_channel_counts.insert(*i.key(), 0);
    //     }
    //     for i in &self.channel_to_sfu {
    //         *sfu_channel_counts.get_mut(i.value()).unwrap() += 1;
    //     }
    //     let mut sorted: Vec<_> = sfu_channel_counts.into_iter().collect();
    //     sorted.sort_by_key(|(_, count)| *count);
    //     if let Some((chosen, _)) = sorted.first() {
    //         self.channel_to_sfu.insert(channel_id, *chosen);
    //         let channel = self.state.services().channels.get(channel_id, None).await?;
    //         self.state
    //             .broadcast_sfu(SfuCommand::Channel {
    //                 channel: channel.into(),
    //             })
    //             .unwrap();
    //         Ok(*chosen)
    //     } else {
    //         error!("no available sfu");
    //         Err(Error::BadStatic("no available sfu"))
    //     }
    // }

    // pub fn call_get(&self, channel_id: ChannelId) -> Result<Call> {
    //     self.calls
    //         .get(&channel_id)
    //         .map(|s| s.clone())
    //         .ok_or(Error::ApiError(ApiError::from_code(
    //             ErrorCode::UnknownVoiceChannel,
    //         )))
    // }

    // pub async fn call_create(&self, params: CallCreate) -> Result<()> {
    //     let channel = self
    //         .state
    //         .services()
    //         .channels
    //         .get(params.channel_id, None)
    //         .await?;

    //     let room_id = channel.room_id;
    //     let call = Call {
    //         room_id,
    //         channel_id: params.channel_id,
    //         topic: params.topic,
    //         created_at: Time::now_utc(),
    //     };
    //     self.calls.insert(params.channel_id, call.clone());

    //     let _ = self.state.broadcast(MessageSync::CallCreate { call });

    //     let has_voice_states = self
    //         .voice_states
    //         .iter()
    //         .any(|s| s.channel_id == params.channel_id);

    //     if !has_voice_states {
    //         self.spawn_call_cleanup(params.channel_id);
    //     }

    //     Ok(())
    // }

    // pub async fn call_delete(&self, channel_id: ChannelId, force: bool) {
    //     if force {
    //         let _ = self.disconnect_everyone(channel_id).await;
    //     }
    //     self.calls.remove(&channel_id);

    //     if let Some((_, handle)) = self.cleanup_tasks.remove(&channel_id) {
    //         handle.abort();
    //     }

    //     let _ = self.state.broadcast(MessageSync::CallDelete { channel_id });
    // }
}
