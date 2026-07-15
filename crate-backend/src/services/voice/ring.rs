use crate::Result;
use crate::services::voice::ServiceVoice;
use common::v1::types::{ChannelId, UserId, voice::RingEligibility};

// TODO: implement ringing (for Dm/Gdm channels)

impl ServiceVoice {
    /// check if this channel can be rung
    pub fn ring_eligible(
        &self,
        _channel_id: ChannelId,
        _user_id: UserId,
    ) -> Result<RingEligibility> {
        todo!()
    }

    /// start ringing users in this channel
    pub fn ring_start(&self, _channel_id: ChannelId, _user_ids: &[UserId]) -> Result<()> {
        todo!()
    }

    /// stop ringing users in this channel
    pub fn ring_stop(&self, _channel_id: ChannelId, _user_ids: &[UserId]) -> Result<()> {
        todo!()
    }
}
