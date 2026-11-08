use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use common::v1::types::error::{ApiError, ErrorCode};
use common::v1::types::oauth::Scopes;
use common::v1::types::{ApplicationId, UserId};
use dashmap::DashMap;
use uuid::Uuid;

use crate::prelude::*;

pub const DEVICE_CODE_EXPIRATION: Duration = Duration::from_secs(10 * 60);
pub const DEVICE_CODE_INTERVAL: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceAuthorizationStatus {
    Pending,
    Approved {
        user_id: UserId,
    },
    /// approved and currently being exchanged for a token
    Consuming {
        user_id: UserId,
    },
    Denied,
}

#[derive(Debug, Clone)]
pub struct DeviceAuthorization {
    pub application_id: ApplicationId,
    pub scopes: Scopes,
    pub user_code: String,
    pub created_at: Instant,
    pub status: DeviceAuthorizationStatus,
    pub interval: Duration,
    pub last_polled_at: Option<Instant>,
}

impl DeviceAuthorization {
    fn is_expired(&self, now: Instant) -> bool {
        now.duration_since(self.created_at) >= DEVICE_CODE_EXPIRATION
    }
}

#[derive(Debug, Clone)]
pub enum DeviceAuthorizationPoll {
    Pending,
    SlowDown,
    Denied,
    Expired,
    Invalid,
    Approved {
        application_id: ApplicationId,
        user_id: UserId,
        scopes: Scopes,
    },
}

#[derive(Clone)]
pub struct DeviceAuthorizations {
    entries: Arc<DashMap<String, DeviceAuthorization>>,
}

impl DeviceAuthorizations {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(DashMap::new()),
        }
    }

    pub fn cleanup_expired(&self) {
        let now = Instant::now();
        self.entries.retain(|_, state| !state.is_expired(now));
    }

    /// start a device authorization flow
    pub fn create(
        &self,
        application_id: ApplicationId,
        scopes: Scopes,
    ) -> (String, DeviceAuthorization) {
        let device_code = Uuid::new_v4().to_string();
        let user_code = self.generate_unique_user_code();
        let authorization = DeviceAuthorization {
            application_id,
            scopes,
            user_code,
            created_at: Instant::now(),
            status: DeviceAuthorizationStatus::Pending,
            interval: DEVICE_CODE_INTERVAL,
            last_polled_at: None,
        };
        self.entries
            .insert(device_code.clone(), authorization.clone());
        (device_code, authorization)
    }

    /// look up a pending device authorization by its user code
    pub fn get_by_user_code(&self, user_code: &str) -> Option<DeviceAuthorization> {
        let device_code = self.find_device_code(user_code, Instant::now())?;
        self.entries
            .get(&device_code)
            .map(|entry| entry.value().clone())
    }

    /// approve or deny a pending device authorization
    pub fn authorize(&self, user_code: &str, user_id: UserId, approve: bool) -> Result<()> {
        let device_code = self
            .find_device_code(user_code, Instant::now())
            .ok_or_else(invalid_user_code)?;

        let mut authorization = self
            .entries
            .get_mut(&device_code)
            .ok_or_else(invalid_user_code)?;
        if authorization.status != DeviceAuthorizationStatus::Pending {
            return Err(ApiError::with_message(
                ErrorCode::InvalidData,
                "device authorization has already been handled".to_string(),
            )
            .into());
        }
        authorization.status = if approve {
            DeviceAuthorizationStatus::Approved { user_id }
        } else {
            DeviceAuthorizationStatus::Denied
        };
        Ok(())
    }

    /// poll a device authorization from the token endpoint
    pub fn poll(&self, device_code: &str) -> DeviceAuthorizationPoll {
        let now = Instant::now();
        let Some(mut authorization) = self.entries.get_mut(device_code) else {
            return DeviceAuthorizationPoll::Invalid;
        };
        if authorization.is_expired(now) {
            drop(authorization);
            self.entries.remove(device_code);
            return DeviceAuthorizationPoll::Expired;
        }
        match authorization.status {
            DeviceAuthorizationStatus::Pending => {
                if let Some(last_polled_at) = authorization.last_polled_at {
                    if now.saturating_duration_since(last_polled_at) < authorization.interval {
                        authorization.interval = authorization.interval + Duration::from_secs(5);
                        authorization.last_polled_at = Some(now);
                        return DeviceAuthorizationPoll::SlowDown;
                    }
                }
                authorization.last_polled_at = Some(now);
                DeviceAuthorizationPoll::Pending
            }
            DeviceAuthorizationStatus::Approved { user_id } => {
                let application_id = authorization.application_id;
                let scopes = authorization.scopes.clone();
                // claim the authorization until the token has been issued
                authorization.status = DeviceAuthorizationStatus::Consuming { user_id };
                DeviceAuthorizationPoll::Approved {
                    application_id,
                    user_id,
                    scopes,
                }
            }
            DeviceAuthorizationStatus::Consuming { .. } => DeviceAuthorizationPoll::SlowDown,
            DeviceAuthorizationStatus::Denied => {
                drop(authorization);
                self.entries.remove(device_code);
                DeviceAuthorizationPoll::Denied
            }
        }
    }

    /// finish a claimed device authorization after its token has been issued
    pub fn consume(&self, device_code: &str) {
        self.entries.remove(device_code);
    }

    /// release a claimed device authorization if token issuance failed
    pub fn release(&self, device_code: &str) {
        if let Some(mut authorization) = self.entries.get_mut(device_code) {
            if let DeviceAuthorizationStatus::Consuming { user_id } = authorization.status {
                authorization.status = DeviceAuthorizationStatus::Approved { user_id };
            }
        }
    }

    fn generate_unique_user_code(&self) -> String {
        loop {
            let code = generate_user_code();
            let normalized = normalize_user_code(&code);
            if !self
                .entries
                .iter()
                .any(|entry| normalize_user_code(&entry.value().user_code) == normalized)
            {
                break code;
            }
        }
    }

    fn find_device_code(&self, user_code: &str, now: Instant) -> Option<String> {
        let normalized = normalize_user_code(user_code);
        self.entries
            .iter()
            .find(|entry| {
                normalize_user_code(&entry.value().user_code) == normalized
                    && !entry.value().is_expired(now)
            })
            .map(|entry| entry.key().clone())
    }
}

fn invalid_user_code() -> ApiError {
    ApiError::with_message(
        ErrorCode::InvalidData,
        "invalid or expired user code".to_string(),
    )
}

fn normalize_user_code(user_code: &str) -> String {
    user_code
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_uppercase()
}

fn generate_user_code() -> String {
    const CHARSET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
    let mut bytes = [0u8; 8];
    rand::fill(&mut bytes);
    let code: String = bytes
        .iter()
        .map(|byte| CHARSET[*byte as usize % CHARSET.len()] as char)
        .collect();
    format!("{}-{}", &code[..4], &code[4..])
}
