use crate::Result;
use crate::services::voice::ServiceVoice;
use crate::services::voice::calls::CallHandleInner;
use common::v1::types::error::{ApiError, ErrorCode};
use common::v1::types::{
    ChannelId, MessageSync, UserId,
    voice::{Call, RingEligibility},
};
use std::sync::Arc;

/// maximum number of users that can be rung at once
const MAX_RINGING_USERS: usize = 100;

impl ServiceVoice {
    /// check if this channel can be rung
    pub async fn ring_eligible(
        &self,
        channel_id: ChannelId,
        user_id: UserId,
    ) -> Result<RingEligibility> {
        let channel = self
            .state
            .services()
            .channels
            .get(channel_id, Some(user_id))
            .await?;

        Ok(RingEligibility {
            ringable: channel.ty.is_dm(),
        })
    }

    /// start ringing users in this channel
    pub async fn ring_start(&self, channel_id: ChannelId, user_ids: &[UserId]) -> Result<()> {
        let channel = self.state.services().channels.get(channel_id, None).await?;
        if !channel.ty.is_dm() {
            return Err(ApiError::from_code(ErrorCode::InvalidData).into());
        }

        if user_ids.len() > MAX_RINGING_USERS
            || user_ids
                .iter()
                .any(|user_id| !channel.recipients.iter().any(|user| user.id == *user_id))
        {
            return Err(ApiError::with_message(
                ErrorCode::InvalidData,
                "can only ring members of this dm or gdm".to_string(),
            )
            .into());
        }

        let call = self.call_update_ringing(channel_id, |ringing| {
            for user_id in user_ids {
                if !ringing.contains(user_id) {
                    ringing.push(*user_id);
                }
            }
        })?;

        if let Some(call) = call {
            self.state
                .messaging()
                .broadcast_channel(channel_id, MessageSync::CallUpdate { call })
                .await?;
        }

        Ok(())
    }

    /// stop ringing users in this channel
    pub async fn ring_stop(&self, channel_id: ChannelId, user_ids: &[UserId]) -> Result<()> {
        let channel = self.state.services().channels.get(channel_id, None).await?;
        if !channel.ty.is_dm() {
            return Err(ApiError::from_code(ErrorCode::InvalidData).into());
        }

        let call = self.call_update_ringing(channel_id, |ringing| {
            ringing.retain(|user_id| !user_ids.contains(user_id));
        })?;

        if let Some(call) = call {
            self.state
                .messaging()
                .broadcast_channel(channel_id, MessageSync::CallUpdate { call })
                .await?;
        }

        Ok(())
    }

    fn call_update_ringing(
        &self,
        channel_id: ChannelId,
        update: impl FnOnce(&mut Vec<UserId>),
    ) -> Result<Option<Call>> {
        let Some(mut entry) = self.calls.get_mut(&channel_id) else {
            return Err(ApiError::from_code(ErrorCode::UnknownCall).into());
        };

        let handle = entry.value();
        let mut ringing = handle.call.inner.ringing.clone();
        update(&mut ringing);

        if handle.call.inner.ringing == ringing {
            return Ok(None);
        }

        let mut call = handle.call.clone();
        call.inner.ringing = ringing;

        *entry.value_mut() = Arc::new(CallHandleInner {
            call: call.clone(),
            sfus: handle.sfus.clone(),
            cleanup_task: handle.cleanup_task.clone(),
            voice_states: handle.voice_states.clone(),
            message_id: handle.message_id,
        });

        Ok(Some(call))
    }
}
