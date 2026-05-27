use std::sync::{Arc, Mutex};

use axum::{body::Body, extract::Request, middleware::Next, response::Response};
use common::v1::types::{
    util::Time, ApplicationId, AuditLogEntry, AuditLogEntryId, AuditLogEntryStatus,
    AuditLogEntryType, MessageSync, RoomId,
};
use http::StatusCode;
use tracing::{debug, error};

use crate::routes::util::auth::Auth;
use crate::Error;

pub type AuditLogSlot = Arc<Mutex<Option<AuditLoggerTransaction>>>;

/// an in-progress audit log
#[derive(Clone)]
pub struct AuditLoggerTransaction {
    pub context_id: RoomId,
    pub auth: Auth,
    pub reason: Option<String>,
    pub started_at: Time,
    pub application_id: Option<ApplicationId>,
    pub ty: Option<AuditLogEntryType>,
    pub status: Option<AuditLogEntryStatus>,
}

pub struct AuditLoggerTransaction2 {
    local_txn: Option<AuditLoggerTransaction>,
    slot: Option<AuditLogSlot>,
}

impl Auth {
    /// begin an audit log transaction
    // TODO: automatically save failed audit logs
    #[must_use = "must call commit() to save a successful audit log entry"]
    pub fn audit_log(&self, context_id: RoomId) -> AuditLoggerTransaction {
        AuditLoggerTransaction {
            context_id,
            auth: self.clone(),
            reason: self.reason.clone(),
            started_at: Time::now_utc(),
            application_id: self.session.app_id,
            ty: None,
            status: None,
        }
    }

    #[must_use = "must call commit() to save a successful audit log entry"]
    pub fn audit_log2(&self, context_id: RoomId, ty: AuditLogEntryType) -> AuditLoggerTransaction2 {
        let txn = AuditLoggerTransaction {
            context_id,
            auth: self.clone(),
            reason: self.reason.clone(),
            started_at: Time::now_utc(),
            application_id: self.session.app_id,
            ty: Some(ty),
            status: None,
        };

        if let Some(slot) = &self.audit_log_slot {
            let mut guard = slot.lock().unwrap();
            *guard = Some(txn);
            AuditLoggerTransaction2 {
                local_txn: None,
                slot: Some(slot.clone()),
            }
        } else {
            AuditLoggerTransaction2 {
                local_txn: Some(txn),
                slot: None,
            }
        }
    }
}

impl AuditLoggerTransaction {
    /// save an audit log entry with the success status
    pub async fn commit_success(self, ty: AuditLogEntryType) -> Result<(), Error> {
        self.commit(AuditLogEntryStatus::Success, ty).await
    }

    /// save an audit log entry
    pub async fn commit(
        mut self,
        status: AuditLogEntryStatus,
        ty: AuditLogEntryType,
    ) -> Result<(), Error> {
        self.commit_inner(status, ty).await
    }

    async fn commit_inner(
        &mut self,
        status: AuditLogEntryStatus,
        ty: AuditLogEntryType,
    ) -> Result<(), Error> {
        let entry = AuditLogEntry {
            id: AuditLogEntryId::new(),
            room_id: self.context_id,
            ty,
            user_id: self.auth.user.id,
            session_id: Some(self.auth.session.id),
            reason: self.reason.clone(),
            status,
            started_at: self.started_at,
            ended_at: Time::now_utc(),
            ip_addr: self.auth.session.imprint.ip_addr.clone(),
            user_agent: self.auth.session.imprint.user_agent.clone(),
            application_id: self.application_id,
        };
        self.auth
            .s
            .data()
            .audit_logs_room_append(entry.clone())
            .await?;
        self.auth
            .s
            .broadcast_room(
                entry.room_id,
                entry.user_id,
                MessageSync::AuditLogEntryCreate { entry },
            )
            .await?;
        Ok(())
    }
}

impl Drop for AuditLoggerTransaction2 {
    fn drop(&mut self) {
        if self.slot.is_some() {
            return;
        }
        if let Some(mut txn) = self.local_txn.take() {
            let status = if let Some(s) = &txn.status {
                s.to_owned()
            } else {
                debug!("implicitly failing audit log entry");
                AuditLogEntryStatus::Failed
            };
            tokio::spawn(async move {
                if let Err(err) = txn.commit_inner(status, txn.ty.clone().unwrap()).await {
                    error!("failed to save audit log: {err:?}");
                }
            });
        }
    }
}

impl AuditLoggerTransaction2 {
    pub fn set_status(&mut self, status: AuditLogEntryStatus) {
        if let Some(slot) = &self.slot {
            if let Ok(mut guard) = slot.lock() {
                if let Some(txn) = guard.as_mut() {
                    txn.status = Some(status);
                }
            }
        } else if let Some(txn) = self.local_txn.as_mut() {
            txn.status = Some(status);
        }
    }

    pub fn success(mut self) {
        self.set_status(AuditLogEntryStatus::Success);
    }

    pub fn unauthorized(mut self) {
        self.set_status(AuditLogEntryStatus::Unauthorized);
    }

    pub fn failed(mut self) {
        self.set_status(AuditLogEntryStatus::Failed);
    }
}

pub async fn audit_log_middleware(mut req: Request<Body>, next: Next) -> Response {
    let slot: AuditLogSlot = Arc::new(Mutex::new(None));
    req.extensions_mut().insert(slot.clone());

    let response = next.run(req).await;

    if let Ok(mut guard) = slot.lock() {
        if let Some(mut txn) = guard.take() {
            let status = if let Some(s) = txn.status.clone() {
                s
            } else if response.status().is_success() {
                AuditLogEntryStatus::Success
            } else if response.status() == StatusCode::FORBIDDEN
                || response.status() == StatusCode::UNAUTHORIZED
            {
                AuditLogEntryStatus::Unauthorized
            } else {
                AuditLogEntryStatus::Failed
            };

            tokio::spawn(async move {
                if let Err(err) = txn.commit_inner(status, txn.ty.clone().unwrap()).await {
                    error!("failed to save audit log: {err:?}");
                }
            });
        }
    }

    response
}
