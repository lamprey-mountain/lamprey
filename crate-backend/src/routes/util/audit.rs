use std::sync::Arc;

use axum::{
    body::Body,
    http::{Request, Response},
    middleware::Next,
};
use common::v1::types::{
    util::Time, ApplicationId, AuditLogEntry, AuditLogEntryId, AuditLogEntryStatus,
    AuditLogEntryType, MessageSync, RoomId,
};
use tokio::sync::Mutex;
use tracing::warn;

use crate::{prelude::*, ServerState};

pub type AuditTxnSlot = Arc<Mutex<AuditTxn>>;

/// an active audit log transaction
pub struct AuditTxn {
    s: Arc<ServerState>,
    started_at: Time,
    state: AuditTxnState,
}

// TODO: remove
/// an in-progress audit log
#[derive(Clone)]
pub struct AuditLoggerTransaction {
    pub context_id: RoomId,
    pub auth: super::Auth,
    pub reason: Option<String>,
    pub started_at: Time,
    pub application_id: Option<ApplicationId>,
    pub ty: Option<AuditLogEntryType>,
    pub status: Option<AuditLogEntryStatus>,
}

enum AuditTxnState {
    Idle,

    Created {
        context_id: RoomId,
        // reason: Option<String>,
        status: Option<AuditLogEntryStatus>,
        // pub auth: Auth,
        // pub application_id: Option<ApplicationId>,
        // pub ty: Option<AuditLogEntryType>,
    },

    Committed,
}

/// a handle to an [`AuditTxn`]
pub struct AuditTxnHandle {
    // local_txn: Option<AuditLoggerTransaction>,
    pub(super) slot: AuditTxnSlot,
}

impl AuditTxn {
    pub(super) fn begin(&mut self, context_id: RoomId) -> Self {
        self.state = AuditTxnState::Created {
            context_id,
            status: None,
        };
    }

    async fn commit(self) -> Result<()> {
        let entry = AuditLogEntry { ..todo!() };
        self.s.data().audit_logs_room_append(entry.clone()).await?;
        self.s
            .broadcast_room(
                entry.room_id,
                entry.user_id,
                MessageSync::AuditLogEntryCreate { entry },
            )
            .await?;
        Ok(())
    }
}

impl AuditTxnHandle {
    fn set_status(&mut self, status: AuditLogEntryStatus) {
        if let Ok(txn) = self.slot.lock() {
            txn.status = Some(status);
        }
    }

    /// mark this audit log transaction as successful
    pub fn success(mut self) {
        todo!()
    }

    pub fn unauthorized(mut self) {
        todo!()
    }

    /// mark this audit log transaction as failed
    pub fn failed(mut self) {
        self.set_status(AuditLogEntryStatus::Failed);
    }
}

impl Drop for AuditTxnHandle {
    fn drop(&mut self) {
        if let Ok(txn) = self.slot.lock() {
            if txn.status.is_none() {
                warn!("AuditTxnHandle dropped without explicit commit; marking as failed");
            }
        }
    }
}

impl super::Auth {
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
}

// TODO: remove
impl AuditLoggerTransaction {
    /// save an audit log entry with the success status
    pub async fn commit_success(self, ty: AuditLogEntryType) -> Result<()> {
        self.commit(AuditLogEntryStatus::Success, ty).await
    }

    /// save an audit log entry
    pub async fn commit(
        mut self,
        status: AuditLogEntryStatus,
        ty: AuditLogEntryType,
    ) -> Result<()> {
        self.commit_inner(status, ty).await
    }

    async fn commit_inner(
        &mut self,
        status: AuditLogEntryStatus,
        ty: AuditLogEntryType,
    ) -> Result<()> {
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

/// middleware to initialize an audit log entry
pub async fn audit_log_middleware(mut req: Request<Body>, next: Next) -> Response<Body> {
    let txn = AuditTxn {
        s: todo!(),
        started_at: Time::now(),
        state: AuditTxnState::Idle,
    };

    let slot = Arc::new(Mutex::new(txn));
    req.extensions_mut().insert(Arc::clone(&slot));

    let response = next.run(req).await;

    if let Ok(mut guard) = slot.lock_owned() {
        // TODO: commit audit log
        // guard
        // if let Some(mut txn) = guard.take() {
        //     let status = if let Some(s) = txn.status.clone() {
        //         s
        //     } else if response.status().is_success() {
        //         AuditLogEntryStatus::Success
        //     } else if response.status() == StatusCode::FORBIDDEN
        //         || response.status() == StatusCode::UNAUTHORIZED
        //     {
        //         AuditLogEntryStatus::Unauthorized
        //     } else {
        //         AuditLogEntryStatus::Failed
        //     };

        //     tokio::spawn(async move {
        //         if let Err(err) = txn.commit_inner(status, txn.ty.clone().unwrap()).await {
        //             error!("failed to save audit log: {err:?}");
        //         }
        //     });
        // }

        todo!()
    }

    response
}
