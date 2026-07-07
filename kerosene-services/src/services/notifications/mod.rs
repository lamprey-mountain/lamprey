use common::{
    v1::types::{
        Channel, Message,
        ack::{AckBulk, AckCreate},
        notifications::Notification,
    },
    v2::types::{ChannelId, UserId},
};

use crate::prelude::*;

mod ack;
mod calculator;
mod push;

pub struct Service {
    globals: Globals,
}

impl Service {
    pub fn new(_globals: Globals) -> Self {
        todo!()
    }

    pub fn start_background_tasks(&self) {
        // TODO: spawn push task
        // TODO: spawn sync event handler task?
    }

    /// process a channel ack
    pub async fn process_ack(&self, _user_id: UserId, _channel_id: ChannelId, _ack: AckCreate) {
        todo!()
    }

    /// process a batch of acks
    pub async fn process_acks(&self, _user_id: UserId, _acks: AckBulk) {
        todo!()
    }

    /// process notifications/acks for a new message
    pub async fn process_message(&self, _channel: &Channel, _message: &Message) {
        todo!()
    }

    /// process notifications/acks for a new thread
    pub async fn process_thread(&self, _parent_channel: &Channel, _thread: &Channel) {
        todo!()
    }

    /// process a new notification
    pub async fn process_notification(
        &self,
        _user_id: UserId,
        _notification: &Notification,
    ) -> Result<()> {
        todo!()
    }
}
