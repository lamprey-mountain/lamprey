use axum::{
    body::Body,
    extract::State,
    http::{Request, Response},
    middleware::Next,
};
use common::{
    v1::types::{AuditLogEntry, AuditLogEntryStatus, AuditLogEntryType, RoomId, util::Time},
    v2::types::{ApplicationId, AuditLogEntryId, SessionId, UserId},
};
use kerosene_core::error::{LegacyErrorExt, ServerResult};
use lamprey_backend_services::globals::Globals;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::error;

/// metadata about the actor who caused an audit log entry to be appended
#[derive(Debug)]
pub struct ActorInfo {
    pub user_id: UserId,
    pub session_id: SessionId,
    pub ip_addr: Option<String>,
    pub user_agent: Option<String>,
    pub application_id: Option<ApplicationId>,
}

/// a pending audit log entry
#[derive(Debug)]
pub struct Entry {
    id: AuditLogEntryId,
    room_id: RoomId,
    ty: AuditLogEntryType,
    reason: Option<String>,
    status: Option<AuditLogEntryStatus>,
    started_at: Time,
    ended_at: Option<Time>,
    actor: Arc<ActorInfo>,
}

#[derive(Debug)]
pub struct AuditLoggerState {
    /// when this request started
    started_at: Time,

    /// currently pending entries
    ///
    /// these haven't been written yet
    pending_entries: Vec<Entry>,
}

pub type AuditLoggerSlot = Arc<Mutex<Option<AuditLoggerState>>>;

pub struct AuditLoggerHandle {
    actor: Arc<ActorInfo>,
    slot: AuditLoggerSlot,
}

impl AuditLoggerHandle {
    pub(super) fn new(actor: Arc<ActorInfo>, slot: AuditLoggerSlot) -> Self {
        Self { actor, slot }
    }

    pub async fn push(
        &self,
        room_id: RoomId,
        ty: AuditLogEntryType,
        reason: Option<String>,
    ) -> EntryHandle {
        let entry_id = AuditLogEntryId::new();
        let entry = Entry {
            id: entry_id,
            room_id,
            ty,
            reason,
            status: None,
            started_at: Time::now_utc(),
            ended_at: None,
            actor: Arc::clone(&self.actor),
        };

        let mut guard = self.slot.lock().await;
        if let Some(state) = guard.as_mut() {
            state.pending_entries.push(entry);
        }

        EntryHandle {
            id: entry_id,
            slot: self.slot.clone(),
        }
    }
}

/// A handle to a pending audit log entry.
///
/// This can be used to mark the entry as successful, unauthorized, or
/// unsuccessful. [`EntryHandle`]s can past the lifetime of a request.
pub struct EntryHandle {
    id: AuditLogEntryId,
    slot: AuditLoggerSlot,
}

impl EntryHandle {
    pub async fn set_status(&self, status: AuditLogEntryStatus) {
        let mut guard = self.slot.lock().await;
        if let Some(state) = guard.as_mut() {
            if let Some(entry) = state.pending_entries.iter_mut().find(|e| e.id == self.id) {
                entry.status = Some(status);
            }
        }
    }

    pub async fn success(self) {
        self.set_status(AuditLogEntryStatus::Success).await;
    }

    pub async fn unauthorized(self) {
        self.set_status(AuditLogEntryStatus::Unauthorized).await;
    }

    pub async fn failed(self) {
        self.set_status(AuditLogEntryStatus::Failed).await;
    }
}

/// middleware to initialize an audit log entry
pub async fn middleware(
    State(globals): State<Globals>,
    mut req: Request<Body>,
    next: Next,
) -> Response<Body> {
    // set up a new audit logger
    let slot: AuditLoggerSlot = Arc::new(Mutex::new(Some(AuditLoggerState {
        started_at: Time::now_utc(),
        pending_entries: Vec::new(),
    })));
    req.extensions_mut().insert(slot.clone());

    // continue running the request
    let res = next.run(req).await;

    // commit any pending entries after request completes
    let mut guard = slot.lock().await;
    if let Some(state) = guard.take() {
        let res = async {
            let mut txn = globals.begin().await.cast_internal()?;
            let ended_at = Time::now_utc();

            for pending in state.pending_entries {
                let entry = AuditLogEntry {
                    id: pending.id,
                    room_id: pending.room_id,
                    user_id: pending.actor.user_id,
                    session_id: Some(pending.actor.session_id),
                    reason: pending.reason,
                    ty: pending.ty,
                    status: pending.status.unwrap_or(AuditLogEntryStatus::Failed),
                    started_at: pending.started_at,
                    ended_at,
                    ip_addr: pending.actor.ip_addr.clone(),
                    user_agent: pending.actor.user_agent.clone(),
                    application_id: pending.actor.application_id,
                };

                txn.audit_logs_room_append(entry).await.cast_internal()?;
            }

            txn.commit().await.cast_internal()?;
            ServerResult::Ok(())
        };

        if let Err(err) = res.await {
            error!("failed to write audit logs: {err:?}");
        }
    }

    res
}
