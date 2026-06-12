use std::sync::Arc;

use axum::{
    body::Body,
    extract::State,
    http::{Request, Response},
    middleware::Next,
};
use common::v1::types::{
    util::Time, ApplicationId, AuditLogEntry, AuditLogEntryId, AuditLogEntryStatus,
    AuditLogEntryType, MessageSync, RoomId,
};
use http::StatusCode;
use tokio::sync::Mutex;
use tracing::{error, warn};

use crate::{prelude::*, ServerState};

pub type AuditTxnSlot = Arc<Mutex<Option<AuditTxn>>>;

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

#[derive(Debug, Clone, PartialEq, Eq)]
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
    pub(super) fn begin(&mut self, context_id: RoomId) {
        self.state = AuditTxnState::Created {
            context_id,
            status: None,
        };
    }

    /// commit this audit log transaction
    ///
    /// this saves the audit log entry to the database and broadcasts an `AuditLogEntryCreate` sync message
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
        // FIXME: don't use blocking lock
        let mut txn = self.slot.blocking_lock();
        if let AuditTxnState::Created {
            status: ref mut current_status,
            ..
        } = txn.as_mut().unwrap().state
        {
            *current_status = Some(status);
        }
    }

    /// mark this audit log transaction as successful
    pub fn success(mut self) {
        self.set_status(AuditLogEntryStatus::Success);
    }

    pub fn unauthorized(mut self) {
        self.set_status(AuditLogEntryStatus::Unauthorized);
    }

    /// mark this audit log transaction as failed
    pub fn failed(mut self) {
        self.set_status(AuditLogEntryStatus::Failed);
    }
}

impl Drop for AuditTxnHandle {
    fn drop(&mut self) {
        // FIXME: don't use blocking lock
        let txn = self.slot.blocking_lock();
        if txn.as_ref().unwrap().state != AuditTxnState::Committed {
            warn!("AuditTxnHandle dropped without explicit commit. the audit log transaction will likely be marked as failed.");
        }
    }
}

// TODO: remove
impl super::Auth {
    /// begin an audit log transaction
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
pub async fn audit_log_middleware(
    State(s): State<Arc<ServerState>>,
    mut req: Request<Body>,
    next: Next,
) -> Response<Body> {
    let txn = AuditTxn {
        s: Arc::clone(&s),
        started_at: Time::now_utc(),
        state: AuditTxnState::Idle,
    };
    let slot = Arc::new(Mutex::new(Some(txn)));
    req.extensions_mut().insert(Arc::clone(&slot));

    let response = next.run(req).await;

    let mut guard = slot.lock().await;
    let mut txn = guard.take().unwrap();
    match &mut txn.state {
        AuditTxnState::Idle | AuditTxnState::Committed => {
            // unused or already committed; ignore
        }
        AuditTxnState::Created { status, .. } => {
            let task = match status {
                // explicitly set status
                Some(_) => txn.commit(),

                // guess the status
                None => {
                    let new_status = if matches!(
                        response.status(),
                        StatusCode::FORBIDDEN | StatusCode::UNAUTHORIZED
                    ) {
                        AuditLogEntryStatus::Unauthorized
                    } else {
                        AuditLogEntryStatus::Failed
                    };
                    *status = Some(new_status);
                    txn.commit()
                }
            };

            tokio::spawn(async move {
                if let Err(err) = task.await {
                    error!("failed to save audit log: {err:?}");
                }
            });
        }
    }

    response
}
