use crate::services::voice::calls::CallHandle;
use crate::services::voice::sfus::SfuHandle;
use crate::ServerStateInner;
use common::v1::types::voice::internal::SfuStats;
use common::v1::types::voice::messages::SfuCommand;
use common::v1::types::voice::router::{VoiceRouter, VoiceRouterConfig};
use common::v1::types::{ChannelId, SfuId};
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub mod calls;
// pub mod ring;
pub mod sfus;
pub mod sync;
pub mod voice_state;

pub struct ServiceVoice {
    pub state: Arc<ServerStateInner>,
    pub calls: DashMap<ChannelId, CallHandle>,
    pub sfus: DashMap<SfuId, SfuHandle>,
    pub router: RwLock<VoiceRouter>,
}

impl ServiceVoice {
    pub fn new(state: Arc<ServerStateInner>) -> Self {
        let router = VoiceRouter::new(VoiceRouterConfig::default());
        Self {
            state,
            calls: DashMap::new(),
            sfus: DashMap::new(),
            router: RwLock::new(router),
        }
    }

    // TODO: ===== remove all code below =====

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
