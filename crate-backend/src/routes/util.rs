use std::{
    sync::{Arc, Mutex},
    time::{Duration, SystemTime},
};

use axum::{
    body::Body,
    extract::FromRequestParts,
    http::{request::Parts, Request, StatusCode},
    middleware::Next,
    response::Response,
};
use common::v1::types::{
    application::{Scope, Scopes},
    util::Time,
    ApplicationId, AuditLogEntry, AuditLogEntryId, AuditLogEntryStatus, AuditLogEntryType,
    MessageSync, RoomId, SessionToken, User, UserId,
};
use headers::{authorization::Bearer, Authorization, HeaderMapExt};
use http::{HeaderMap, HeaderName, HeaderValue};
use tracing::{debug, error};
use uuid::Uuid;

use crate::{
    error::Error,
    types::{Session, SessionStatus, SessionType},
    ServerState,
};

/// extract authentication info for a request
#[derive(Clone)]
pub struct Auth {
    /// the effective user making this request
    pub user: User,

    /// the real user making this request
    pub real_user: Option<User>,

    /// the session for this request
    pub session: Session,

    /// the oauth scopes this session has
    pub scopes: Scopes,

    /// the audit log reason for this request
    ///
    /// extracted from HeaderReason
    reason: Option<String>,

    /// the audit log slot for this request
    pub audit_log_slot: Option<AuditLogSlot>,

    /// a reference to the server state
    s: Arc<ServerState>,
}

impl Auth {
    pub fn ensure_scopes(&self, scopes: &[Scope]) -> Result<(), Error> {
        self.scopes.ensure_all(scopes).map_err(Into::into)
    }

    pub fn ensure_sudo(&self) -> Result<(), Error> {
        match &self.session.status {
            SessionStatus::Unauthorized => Err(Error::UnauthSession),
            SessionStatus::Authorized { .. } => Err(Error::BadStatic("needs sudo")),
            SessionStatus::Sudo {
                sudo_expires_at, ..
            } => {
                if *sudo_expires_at < Time::now_utc() {
                    Err(Error::BadStatic("sudo session expired"))
                } else {
                    Ok(())
                }
            }
        }
    }
}

/// extract the client's Session
// TODO: remove?
pub struct AuthRelaxed(pub Session);

/// extract the X-Reason header
// TODO: remove?
pub struct HeaderReason(pub Option<String>);

/// extract the Idempotency-Key header
pub struct HeaderIdempotencyKey(pub Option<String>);

/// extract the X-Puppet-Id header
pub struct HeaderPuppetId(pub Option<UserId>);

/// extract caching http headers
pub struct HeaderCache {
    if_none_match: Option<HeaderValue>,
    if_modified_since: Option<HeaderValue>,
}

impl FromRequestParts<Arc<ServerState>> for AuthRelaxed {
    type Rejection = Error;

    async fn from_request_parts(
        parts: &mut Parts,
        s: &Arc<ServerState>,
    ) -> Result<Self, Self::Rejection> {
        let auth: Authorization<Bearer> = parts
            .headers
            .typed_get()
            .ok_or_else(|| Error::MissingAuth)?;
        let srv = s.services();
        let session = srv
            .sessions
            .get_by_token(SessionToken(auth.token().to_string()))
            .await
            .map_err(|err| match err {
                Error::NotFound => Error::MissingAuth,
                other => other,
            })?;
        if session.expires_at.is_some_and(|t| t < Time::now_utc()) {
            return Err(Error::MissingAuth);
        }
        if session.last_seen_at < Time::now_utc() - Duration::from_secs(60) {
            s.data().session_set_last_seen_at(session.id).await?;
            srv.sessions.invalidate(session.id).await;
        }
        Ok(Self(session))
    }
}

impl FromRequestParts<Arc<ServerState>> for HeaderReason {
    type Rejection = Error;

    async fn from_request_parts(
        parts: &mut Parts,
        _s: &Arc<ServerState>,
    ) -> Result<Self, Self::Rejection> {
        let header = parts
            .headers
            .get("X-Reason")
            .and_then(|h| h.to_str().ok())
            .map(|h| h.to_string());

        if let Some(ref reason) = header {
            if reason.chars().count() > 1024 {
                return Err(Error::BadRequest(
                    "X-Audit-Reason must be 1024 characters or less".to_string(),
                ));
            }
        }

        Ok(Self(header))
    }
}

impl FromRequestParts<Arc<ServerState>> for HeaderIdempotencyKey {
    type Rejection = Error;

    async fn from_request_parts(
        parts: &mut Parts,
        _s: &Arc<ServerState>,
    ) -> Result<Self, Self::Rejection> {
        let header = parts
            .headers
            .get("Idempotency-Key")
            .and_then(|h| h.to_str().ok())
            .map(|h| h.to_string());
        Ok(Self(header))
    }
}

impl FromRequestParts<Arc<ServerState>> for HeaderPuppetId {
    type Rejection = Error;

    async fn from_request_parts(
        parts: &mut Parts,
        _s: &Arc<ServerState>,
    ) -> Result<Self, Self::Rejection> {
        let puppet_id = parts
            .headers
            .get("X-Puppet-Id")
            .and_then(|h| h.to_str().ok())
            .and_then(|h| h.parse().ok());
        Ok(Self(puppet_id))
    }
}

impl FromRequestParts<Arc<ServerState>> for Auth {
    type Rejection = Error;

    async fn from_request_parts(
        parts: &mut Parts,
        s: &Arc<ServerState>,
    ) -> Result<Self, Self::Rejection> {
        // load existing session
        let auth: Authorization<Bearer> = parts
            .headers
            .typed_get()
            .ok_or_else(|| Error::MissingAuth)?;
        let token = auth.token();
        let srv = s.services();

        // check admin token
        if srv.admin.verify_admin_token(token).await {
            let user = srv.users.get(crate::types::SERVER_USER_ID, None).await?;
            let session = Session {
                id: crate::types::SERVER_TOKEN_SESSION_ID,
                status: SessionStatus::Sudo {
                    user_id: crate::types::SERVER_USER_ID,
                    sudo_expires_at: Time::now_utc() + Duration::from_secs(3600),
                },
                name: Some("admin token".to_string()),
                ty: SessionType::User,
                expires_at: None,
                app_id: None,
                last_seen_at: Time::now_utc(),
                ip_addr: None,
                user_agent: None,
                authorized_at: Some(Time::now_utc()),
                deauthorized_at: None,
            };

            return Ok(Auth {
                user,
                real_user: None,
                session,
                scopes: Scopes(vec![Scope::Full]),
                reason: HeaderReason::from_request_parts(parts, s).await?.0,
                audit_log_slot: parts.extensions.get::<AuditLogSlot>().cloned(),
                s: s.clone(),
            });
        }

        let reason = HeaderReason::from_request_parts(parts, s).await?;
        let session = srv
            .sessions
            .get_by_token(SessionToken(auth.token().to_string()))
            .await
            .map_err(|err| match err {
                Error::NotFound => Error::MissingAuth,
                other => other,
            })?;
        if session.expires_at.is_some_and(|t| t < Time::now_utc()) {
            return Err(Error::MissingAuth);
        }
        if session.last_seen_at < Time::now_utc() - Duration::from_secs(60) {
            s.data().session_set_last_seen_at(session.id).await?;
            srv.sessions.invalidate(session.id).await;
        }

        let user_id = session.user_id().ok_or(Error::UnauthSession)?;

        let HeaderPuppetId(puppet_id) = HeaderPuppetId::from_request_parts(parts, s).await?;
        let real_user = srv.users.get(user_id, None).await?;

        // load the real user if this is for puppeting
        let mut effective_user = if let Some(puppet_id) = puppet_id {
            let puppet = srv.users.get(puppet_id, None).await?;

            if puppet.bot {
                // check if we own this application
                let app = s
                    .data()
                    .application_get(puppet.id.into_inner().into())
                    .await?;
                if app.owner_id == real_user.id {
                    puppet
                } else {
                    return Err(Error::BadStatic("not bot owner"));
                }
            } else {
                if !real_user.bot {
                    return Err(Error::BadStatic("user is not a bot"));
                }

                // check if we are a bridge
                let app = s
                    .data()
                    .application_get(real_user.id.into_inner().into())
                    .await?;
                if app.bridge.is_none() {
                    return Err(Error::BadStatic("bot is not a bridge"));
                }

                let Some(p) = &puppet.puppet else {
                    return Err(Error::BadStatic("can only puppet users of type Puppet"));
                };

                if p.owner_id.into_inner() != *real_user.id {
                    return Err(Error::BadStatic("can only puppet your own puppets"));
                }

                puppet
            }
        } else {
            real_user.clone()
        };

        // propagate suspension
        if effective_user.id != real_user.id && real_user.is_suspended() {
            effective_user.suspended = real_user.suspended.clone();
        }

        if effective_user.suspended.is_none() {
            if let Some(puppet) = &effective_user.puppet {
                let bot_app_id = puppet.owner_id;
                let bot_user = srv.users.get(bot_app_id.into_inner().into(), None).await?;
                if bot_user.is_suspended() {
                    effective_user.suspended = bot_user.suspended.clone();
                } else if bot_user.bot {
                    // check the owner of the bot
                    if let Ok(app) = s.data().application_get(bot_app_id).await {
                        let owner = srv.users.get(app.owner_id, None).await?;
                        if owner.is_suspended() {
                            effective_user.suspended = owner.suspended.clone();
                        }
                    }
                }
            } else if effective_user.bot {
                if let Ok(app) = s
                    .data()
                    .application_get(effective_user.id.into_inner().into())
                    .await
                {
                    let owner = srv.users.get(app.owner_id, None).await?;
                    if owner.is_suspended() {
                        effective_user.suspended = owner.suspended.clone();
                    }
                }
            }
        }

        let scopes = if session.ty == SessionType::User {
            Scopes(vec![Scope::Auth])
        } else if let Some(app_id) = session.app_id {
            s.data()
                .connection_get(user_id, app_id)
                .await
                .map(|c| c.scopes)
                .unwrap_or_default()
        } else {
            Scopes::default()
        };

        let audit_log_slot = parts.extensions.get::<AuditLogSlot>().cloned();

        Ok(Auth {
            user: effective_user,
            real_user: if puppet_id.is_some() {
                Some(real_user)
            } else {
                None
            },
            session,
            scopes,
            reason: reason.0,
            audit_log_slot,
            s: s.clone(),
        })
    }
}

impl HeaderCache {
    /// compare the etag of the request with the current etag
    fn compare_etag(&self, etag: &str) -> Result<(), Error> {
        if let Some(val) = &self.if_none_match {
            if val == etag {
                return Err(Error::NotModified);
            }
        }

        Ok(())
    }

    /// compare the last-modified-time of the request with the current mtime
    fn compare_mtime(&self, last_modified: &Time) -> Result<(), Error> {
        if let Some(val) = &self.if_modified_since {
            if let Ok(s) = val.to_str() {
                if let Ok(parsed_time) = httpdate::parse_http_date(s) {
                    let last_modified_st = SystemTime::UNIX_EPOCH
                        + Duration::from_secs(last_modified.unix_timestamp() as u64);

                    if last_modified_st <= parsed_time {
                        return Err(Error::NotModified);
                    }
                }
            }
        }
        Ok(())
    }

    /// compare version ids. returns the new caching headers
    pub fn compare_uuid(&self, uuid: &Uuid) -> Result<HeaderMap, Error> {
        let ts: Time = uuid
            .get_timestamp()
            .expect("this is a uuid v7")
            .try_into()
            .expect("uuids are always valid timestamps");
        let etag = format!(r#"W/"{}""#, uuid);
        self.compare_etag(&etag)?;
        self.compare_mtime(&ts)?;
        let headers = HeaderMap::from_iter([
            (
                HeaderName::from_static("last-modified"),
                HeaderValue::from_str(&httpdate::fmt_http_date(
                    (SystemTime::UNIX_EPOCH
                        + Duration::from_nanos(ts.unix_timestamp_nanos().try_into().unwrap_or(0)))
                    .into(),
                ))
                .unwrap(),
            ),
            (
                HeaderName::from_static("etag"),
                HeaderValue::from_str(&etag).unwrap(),
            ),
        ]);
        Ok(headers)
    }
}

impl FromRequestParts<Arc<ServerState>> for HeaderCache {
    type Rejection = Error;

    async fn from_request_parts(
        parts: &mut Parts,
        _s: &Arc<ServerState>,
    ) -> Result<Self, Self::Rejection> {
        let if_none_match = parts.headers.get("if-none-match").cloned();
        let if_modified_since = parts.headers.get("if-modified-since").cloned();
        Ok(Self {
            if_none_match,
            if_modified_since,
        })
    }
}

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
            ip_addr: self.auth.session.ip_addr.clone(),
            user_agent: self.auth.session.user_agent.clone(),
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
