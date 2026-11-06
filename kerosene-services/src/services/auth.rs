//! authentication primitives: password hashing, TOTP, recovery codes, and email auth

use common::v1::types::AuditLogChange;
use common::v1::types::UserId;
use common::v1::types::auth::{AuthState, PasswordExecIdent, TotpRecoveryCode};
use common::v1::types::email::EmailAddr;
use common::v1::types::error::{ApiError, ErrorCode};
use common::v1::types::util::{Changes, Time};
use common::v1::types::{MessageSync, Session, SessionId, SessionStatus};
use lamprey_backend_data_postgres::types::EmailPurpose;
use time::Duration;
use totp_rs::{Algorithm as TotpAlgorithm, Secret as TotpSecret, TOTP as Totp};
use uuid::Uuid;

use crate::prelude::*;

/// number of recovery codes generated per rotation
const RECOVERY_CODE_COUNT: usize = 5;

/// length in bytes of a generated TOTP secret
const TOTP_SECRET_LEN: usize = 20;

/// how long a session stays in sudo mode
const SUDO_DURATION: Duration = Duration::minutes(5);

/// how long a session may wait for its second factor before a new login is required
const MFA_WINDOW: Duration = Duration::minutes(15);

/// session status for a user that still has to complete a second factor
fn bound_status(user_id: UserId) -> SessionStatus {
    SessionStatus::Bound {
        user_id,
        expires_at: Time::now_utc().saturating_add(MFA_WINDOW).into(),
    }
}

pub struct ServiceAuth {
    state: Globals,
}

impl ServiceAuth {
    pub fn new(state: Globals) -> Self {
        Self { state }
    }

    /// generate a new random TOTP secret, base32 encoded without padding
    pub fn generate_totp_secret(&self) -> String {
        let mut secret_bytes = [0u8; TOTP_SECRET_LEN];
        rand::fill(&mut secret_bytes);
        base32::encode(base32::Alphabet::Rfc4648 { padding: false }, &secret_bytes)
    }

    /// verify a TOTP code against a base32 encoded secret
    ///
    /// returns `Ok(())` if the code is valid, otherwise an
    /// [`ErrorCode::InvalidTotpCode`] api error. errors decoding or constructing
    /// the TOTP instance are internal errors.
    pub fn verify_totp_code(&self, secret: &str, code: &str) -> Result<()> {
        let secret_bytes = base32::decode(base32::Alphabet::Rfc4648 { padding: false }, secret)
            .ok_or_else(|| Error::Internal("failed to decode totp secret".to_owned()))?;

        let totp = Totp::new(
            TotpAlgorithm::SHA1,
            6,
            1,
            30,
            TotpSecret::Raw(secret_bytes).to_bytes().unwrap(),
        )
        .map_err(|e| {
            tracing::error!("failed to create totp: {e}");
            Error::Internal("failed to create totp".to_owned())
        })?;

        if totp.check_current(code).unwrap_or(false) {
            Ok(())
        } else {
            Err(ApiError::from_code(ErrorCode::InvalidTotpCode).into())
        }
    }

    /// hash a password with a fresh random salt, returning `(hash, salt)`
    pub fn hash_password(&self, password: &str) -> Result<(Vec<u8>, Vec<u8>)> {
        let salt: [u8; 32] = rand::random();
        let config = argon2::Config::default();
        let hash = argon2::hash_raw(password.as_bytes(), &salt, &config).map_err(|e| {
            tracing::error!("failed to hash password: {e}");
            Error::Internal("failed to hash password".to_owned())
        })?;
        Ok((hash, salt.to_vec()))
    }

    /// verify a password against a stored hash and salt
    pub fn verify_password(&self, password: &str, hash: &[u8], salt: &[u8]) -> bool {
        let config = argon2::Config::default();
        argon2::verify_raw(hash, salt, password.as_bytes(), &config).unwrap_or(false)
    }

    /// generate a new set of recovery codes
    pub fn generate_recovery_codes(&self) -> Vec<String> {
        (0..RECOVERY_CODE_COUNT)
            .map(|_| Uuid::new_v4().to_string())
            .collect()
    }

    /// collect the authentication state for a user
    pub async fn get_state(&self, user_id: UserId) -> Result<AuthState> {
        let mut txn = self.state.begin_read().await?;

        let (_secret, totp_enabled) = txn.auth_totp_get(user_id).await?.unwrap_or_default();
        let has_password = txn.auth_password_get(user_id).await?.is_some();
        let oauth_providers = txn.auth_oauth_get_all(user_id).await?;
        let has_email = !txn.user_email_list(user_id).await?.is_empty();

        Ok(AuthState {
            has_email,
            has_totp: totp_enabled,
            has_password,
            oauth_providers,
            authenticators: vec![], // TODO: implement webauthn
        })
    }

    /// check that a user can still log in after an auth method is removed
    async fn ensure_can_still_login_after_removal(
        &self,
        user_id: UserId,
        simulate: impl FnOnce(&mut AuthState),
    ) -> Result<()> {
        let mut auth_state = self.get_state(user_id).await?;
        simulate(&mut auth_state);

        if !auth_state.can_login() {
            return Err(ApiError::from_code(ErrorCode::CannotRemoveLastAuthMethod).into());
        }

        Ok(())
    }

    /// set a session's status, invalidating cached sessions and returning the
    /// updated session
    async fn set_session_status(
        &self,
        session_id: SessionId,
        status: SessionStatus,
    ) -> Result<Session> {
        let mut txn = self.state.begin().await?;
        txn.session_set_status(session_id, status).await?;
        txn.commit().await?;

        let srv = self.state.services();
        srv.sessions.invalidate(session_id).await;
        let session = srv.sessions.get(session_id).await?;
        Ok((*session).clone())
    }

    /// start totp setup, returning the new secret
    pub async fn init_totp(&self, user_id: UserId) -> Result<String> {
        let secret = self.generate_totp_secret();

        let mut txn = self.state.begin().await?;
        txn.auth_totp_recovery_delete_all(user_id).await?;
        txn.auth_totp_set(user_id, Some(secret.clone()), false)
            .await?;
        txn.commit().await?;

        Ok(secret)
    }

    /// enable totp after verifying a code from the new secret
    pub async fn enable_totp(&self, user_id: UserId, code: &str) -> Result<Vec<AuditLogChange>> {
        let mut txn = self.state.begin().await?;
        let (secret, enabled) = txn
            .auth_totp_get(user_id)
            .await?
            .ok_or_else(|| ApiError::from_code(ErrorCode::TotpNotInitialized))?;

        if enabled {
            return Err(ApiError::from_code(ErrorCode::TotpAlreadyEnabled).into());
        }

        self.verify_totp_code(&secret, code)?;

        txn.auth_totp_set(user_id, Some(secret), true).await?;
        txn.commit().await?;

        Ok(Changes::new().change("has_totp", &false, &true).build())
    }

    /// disable totp for a user
    ///
    /// returns the audit log changes, or `None` if totp was not enabled
    pub async fn delete_totp(&self, user_id: UserId) -> Result<Option<Vec<AuditLogChange>>> {
        // NOTE: mirrors pre-refactor behavior; `can_login` doesn't consider totp
        self.ensure_can_still_login_after_removal(user_id, |_state| {})
            .await?;

        let mut txn = self.state.begin().await?;

        let enabled = txn
            .auth_totp_get(user_id)
            .await?
            .map(|(_secret, enabled)| enabled)
            .unwrap_or(false);
        if !enabled {
            return Ok(None);
        }

        if txn.user_owns_room_requiring_mfa(user_id).await? {
            return Err(ApiError::from_code(ErrorCode::InvalidData).into());
        }

        txn.auth_totp_set(user_id, None, false).await?;
        txn.commit().await?;

        Ok(Some(
            Changes::new().change("has_totp", &true, &false).build(),
        ))
    }

    /// verify a totp code and upgrade the session to sudo mode
    pub async fn totp_exec(
        &self,
        session_id: SessionId,
        user_id: UserId,
        code: &str,
    ) -> Result<AuthState> {
        let (secret, enabled) = {
            let mut txn = self.state.begin_read().await?;
            txn.auth_totp_get(user_id)
                .await?
                .ok_or_else(|| ApiError::from_code(ErrorCode::TotpNotEnabled))?
        };
        if !enabled {
            return Err(ApiError::from_code(ErrorCode::TotpNotEnabled).into());
        }

        self.verify_totp_code(&secret, code)?;
        self.sudo_upgrade(session_id, user_id).await?;

        self.get_state(user_id).await
    }

    /// list existing recovery codes without generating new ones
    pub async fn recovery_codes_get(&self, user_id: UserId) -> Result<Vec<TotpRecoveryCode>> {
        let mut txn = self.state.begin_read().await?;
        let codes = txn.auth_totp_recovery_get_all(user_id).await?;

        Ok(codes
            .into_iter()
            .map(|(code, used_at)| TotpRecoveryCode { code, used_at })
            .collect())
    }

    /// generate new recovery codes, invalidating old ones
    pub async fn recovery_codes_rotate(&self, user_id: UserId) -> Result<Vec<TotpRecoveryCode>> {
        let codes: Vec<TotpRecoveryCode> = self
            .generate_recovery_codes()
            .into_iter()
            .map(|code| TotpRecoveryCode {
                code,
                used_at: None,
            })
            .collect();

        let strings: Vec<String> = codes.iter().map(|c| c.code.clone()).collect();
        let mut txn = self.state.begin().await?;
        txn.auth_totp_recovery_generate(user_id, &strings).await?;
        txn.commit().await?;

        Ok(codes)
    }

    /// consume a recovery code and upgrade the session to sudo mode
    pub async fn recovery_code_exec(
        &self,
        session_id: SessionId,
        user_id: UserId,
        code: &str,
    ) -> Result<AuthState> {
        let mut txn = self.state.begin().await?;
        let (_secret, enabled) = txn
            .auth_totp_get(user_id)
            .await?
            .ok_or_else(|| ApiError::from_code(ErrorCode::TotpNotEnabled))?;
        if !enabled {
            return Err(ApiError::from_code(ErrorCode::TotpNotEnabled).into());
        }

        txn.auth_totp_recovery_use(user_id, code).await?;
        txn.commit().await?;

        self.sudo_upgrade(session_id, user_id).await?;

        self.get_state(user_id).await
    }

    /// set (or replace) a user's password
    pub async fn set_password(
        &self,
        user_id: UserId,
        password: &str,
    ) -> Result<Vec<AuditLogChange>> {
        let (hash, salt) = self.hash_password(password)?;

        let mut txn = self.state.begin().await?;
        let had_password = txn.auth_password_get(user_id).await?.is_some();
        txn.auth_password_set(user_id, &hash, &salt).await?;
        txn.commit().await?;

        Ok(Changes::new()
            .change("has_password", &had_password, &true)
            .build())
    }

    /// remove a user's password
    ///
    /// returns the audit log changes, or `None` if no password was set
    pub async fn delete_password(&self, user_id: UserId) -> Result<Option<Vec<AuditLogChange>>> {
        let has_password = {
            let mut txn = self.state.begin_read().await?;
            txn.auth_password_get(user_id).await?.is_some()
        };
        if !has_password {
            return Ok(None);
        }

        self.ensure_can_still_login_after_removal(user_id, |state| state.has_password = false)
            .await?;

        let mut txn = self.state.begin().await?;
        txn.auth_password_delete(user_id).await?;
        txn.commit().await?;

        Ok(Some(
            Changes::new().change("has_password", &true, &false).build(),
        ))
    }

    /// resolve who is trying to log in with a password
    pub async fn resolve_password_ident(&self, ident: &PasswordExecIdent) -> Result<UserId> {
        match ident {
            PasswordExecIdent::UserId { user_id } => Ok(*user_id),
            PasswordExecIdent::Email { email } => {
                let mut txn = self.state.begin_read().await?;
                Ok(txn.user_email_lookup(email).await?)
            }
        }
    }

    /// verify a password and authorize the session
    ///
    /// returns `true` if the session was authorized, or `false` if a second
    /// factor is still required
    pub async fn password_exec(
        &self,
        user_id: UserId,
        password: &str,
        session_id: SessionId,
    ) -> Result<bool> {
        let password_hash = {
            let mut txn = self.state.begin_read().await?;
            txn.auth_password_get(user_id).await?
        };

        let Some((stored_hash, stored_salt)) = password_hash else {
            return Err(ApiError::from_code(ErrorCode::InvalidPassword).into());
        };

        if !self.verify_password(password, &stored_hash, &stored_salt) {
            return Err(ApiError::from_code(ErrorCode::InvalidPassword).into());
        }

        // no txn is open here, so it's safe to open a read txn via get_state
        let auth_state = self.get_state(user_id).await?;
        if auth_state.has_mfa() {
            self.set_session_status(session_id, bound_status(user_id))
                .await?;
            return Ok(false);
        }

        self.set_session_status(session_id, SessionStatus::Authorized { user_id })
            .await?;

        Ok(true)
    }

    /// upgrade a session to sudo mode
    pub async fn sudo_upgrade(&self, session_id: SessionId, user_id: UserId) -> Result<()> {
        let status = SessionStatus::Sudo {
            user_id,
            sudo_expires_at: Time::now_utc().saturating_add(SUDO_DURATION).into(),
        };
        let session = self.set_session_status(session_id, status).await?;
        self.state
            .messaging()
            .broadcast_global(MessageSync::SessionUpdate { session })
            .await?;
        Ok(())
    }

    /// downgrade a session out of sudo mode
    pub async fn sudo_downgrade(&self, session_id: SessionId, user_id: UserId) -> Result<()> {
        let session = self
            .set_session_status(session_id, SessionStatus::Authorized { user_id })
            .await?;
        self.state
            .messaging()
            .broadcast_global(MessageSync::SessionUpdate { session })
            .await?;
        Ok(())
    }

    /// remove an oauth provider from a user's account
    pub async fn delete_oauth(
        &self,
        user_id: UserId,
        provider: &str,
    ) -> Result<Vec<AuditLogChange>> {
        self.ensure_can_still_login_after_removal(user_id, |state| {
            state.oauth_providers.retain(|p| p != provider);
        })
        .await?;

        let start = self.get_state(user_id).await?;

        let mut txn = self.state.begin().await?;
        txn.auth_oauth_delete(provider.to_string(), user_id).await?;
        txn.commit().await?;

        let mut end = start.clone();
        end.oauth_providers.retain(|p| p != provider);

        Ok(Changes::new()
            .change(
                "oauth_providers",
                &start.oauth_providers,
                &end.oauth_providers,
            )
            .build())
    }

    /// send a "magic link" email to log in
    pub async fn email_login(&self, session_id: SessionId, addr: &str) -> Result<()> {
        self.email_auth_request(
            session_id,
            addr,
            EmailPurpose::Authn,
            "Login to lamprey",
            "click this link to login",
        )
        .await
    }

    /// send an email that also allows resetting the password
    pub async fn email_reset(&self, session_id: SessionId, addr: &str) -> Result<()> {
        self.email_auth_request(
            session_id,
            addr,
            EmailPurpose::Reset,
            "Lamprey password reset",
            "click this link to reset password",
        )
        .await
    }

    async fn email_auth_request(
        &self,
        session_id: SessionId,
        addr: &str,
        purpose: EmailPurpose,
        subject: &str,
        intro: &str,
    ) -> Result<()> {
        let email: EmailAddr = addr
            .to_owned()
            .try_into()
            .map_err(|_| ApiError::from_code(ErrorCode::InvalidData))?;

        let code = Uuid::new_v4().to_string();
        let mut url = self.state.config().html_url.join("email-auth")?;
        url.set_query(Some(&format!("code={code}")));
        let message = format!("{intro}: {url}\n\nif you didn't request this, ignore this email.");

        let mut txn = self.state.begin().await?;
        txn.auth_email_create(code, email.clone(), session_id, purpose)
            .await?;
        txn.commit().await?;

        self.state
            .services()
            .email
            .send(email, subject.to_owned(), message, None)
            .await?;

        Ok(())
    }

    /// consume an email auth code and authorize the session
    ///
    /// returns the user who logged in, what the code was for, and whether a
    /// second factor is still pending
    pub async fn email_complete(
        &self,
        session: &Session,
        addr: &str,
        code: String,
    ) -> Result<(UserId, EmailPurpose, bool)> {
        let email: EmailAddr = addr
            .to_owned()
            .try_into()
            .map_err(|_| ApiError::from_code(ErrorCode::InvalidData))?;

        let mut txn = self.state.begin().await?;
        let (req_addr, req_session, purpose) = txn.auth_email_use(code).await?;

        if req_addr != email || req_session != session.id {
            return Err(ApiError::from_code(ErrorCode::InvalidOrExpiredCode).into());
        }

        if session.status != SessionStatus::Unauthorized && !session.status.bound_expired() {
            return Err(ApiError::from_code(ErrorCode::InvalidOrExpiredCode).into());
        }

        let user_id = txn.user_email_lookup(&email).await?;
        let (_secret, totp_enabled) = txn.auth_totp_get(user_id).await?.unwrap_or_default();
        // TODO: include webauthn authenticators once implemented
        let mfa_pending = totp_enabled;
        let status = match &purpose {
            EmailPurpose::Authn => {
                if mfa_pending {
                    bound_status(user_id)
                } else {
                    SessionStatus::Authorized { user_id }
                }
            }
            // a password reset is a single factor, it only enters sudo mode once
            // the second factor is completed
            EmailPurpose::Reset => {
                if mfa_pending {
                    bound_status(user_id)
                } else {
                    SessionStatus::Sudo {
                        user_id,
                        sudo_expires_at: Time::now_utc().saturating_add(SUDO_DURATION).into(),
                    }
                }
            }
        };
        txn.session_set_status(session.id, status).await?;
        txn.commit().await?;

        let srv = self.state.services();
        srv.sessions.invalidate(session.id).await;
        let session = srv.sessions.get(session.id).await?;
        self.state
            .messaging()
            .broadcast_global(MessageSync::SessionCreate {
                session: (*session).clone(),
            })
            .await?;

        Ok((user_id, purpose, mfa_pending))
    }
}
