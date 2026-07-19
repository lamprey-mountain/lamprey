use bytes::Bytes;
use common::{v1::types::notifications::bytes::NotificationBytes, v2::types::UserId};
use jsonwebtoken::EncodingKey;
use lamprey_backend_data_postgres::PushData;

use crate::prelude::*;

#[derive(Clone)]
pub struct VapidKeys {
    encoding: EncodingKey,
    public: String,
}

impl super::Service {
    /// Helper to fetch and parse VAPID keys from configuration
    pub async fn get_vapid_keys(&self) -> Result<VapidKeys> {
        todo!()
    }

    /// Create new VAPID keys if none exists yet
    pub async fn init_vapid_keys(&self) -> Result<VapidKeys> {
        todo!()
    }

    /// Create new VAPID keys
    pub async fn rotate_vapid_keys(&self) -> Result<VapidKeys> {
        todo!()
    }

    /// Push a notification to all of user's sessions via web push api
    ///
    /// pushes the notification to all sessions in parallel
    pub async fn push(&self, _user_id: UserId, mut _payload: NotificationBytes) -> Result<()> {
        todo!()
    }

    /// send a notification to a session via web push api
    async fn push_inner(_state: Globals, _sub: PushData, _payload: Bytes) -> Result<()> {
        todo!()
    }

    pub(super) async fn spawn_push_task(_state: Globals) {
        todo!()
    }
}
