use crate::prelude::*;
use crate::services::voice::calls::CallHandle;
use crate::services::voice::sfus::SfuHandle;
use common::v1::types::voice::messages::SfuCommand;
use common::v1::types::voice::router::{VoiceRouter, VoiceRouterConfig};
use common::v1::types::{ChannelId, SfuId};
use dashmap::DashMap;
use tokio::sync::RwLock;

pub mod calls;
// pub mod ring;
pub mod sfus;
pub mod sync;
pub mod voice_state;

pub struct ServiceVoice {
    pub state: Globals,
    pub calls: DashMap<ChannelId, CallHandle>,
    pub sfus: DashMap<SfuId, SfuHandle>,
    pub router: RwLock<VoiceRouter>,
}

impl ServiceVoice {
    pub fn new(state: Globals) -> Self {
        let router = VoiceRouter::new(VoiceRouterConfig::default());
        Self {
            state,
            calls: DashMap::new(),
            sfus: DashMap::new(),
            router: RwLock::new(router),
        }
    }
}
