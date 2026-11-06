//! TOTP two-factor authentication and recovery codes

use common::v1::types::AuditLogChange;
use common::v1::types::UserId;
use common::v1::types::auth::{AuthState, TotpRecoveryCode};
use common::v1::types::error::{ApiError, ErrorCode};
use common::v1::types::util::Changes;
use common::v1::types::{Session, SessionStatus};
use totp_rs::{Algorithm as TotpAlgorithm, Secret as TotpSecret, TOTP as Totp};
use uuid::Uuid;

use crate::prelude::*;

use super::ServiceAuth;

/// number of recovery codes generated per rotation
const RECOVERY_CODE_COUNT: usize = 5;

/// length in bytes of a generated TOTP secret
const TOTP_SECRET_LEN: usize = 20;

impl ServiceAuth {
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

    /// verify a user's totp code
    async fn verify_totp_for_user(&self, user_id: UserId, code: &str) -> Result<()> {
        let (secret, enabled) = {
            let mut txn = self.state.begin_read().await?;
            txn.auth_totp_get(user_id)
                .await?
                .ok_or_else(|| ApiError::from_code(ErrorCode::TotpNotEnabled))?
        };
        if !enabled {
            return Err(ApiError::from_code(ErrorCode::TotpNotEnabled).into());
        }

        self.verify_totp_code(&secret, code)
    }

    /// verify a totp code and authenticate the session
    pub async fn totp_exec(
        &self,
        session: &Session,
        user_id: UserId,
        code: &str,
    ) -> Result<AuthState> {
        self.verify_totp_for_user(user_id, code).await?;

        let was_authenticated = session.status.is_authenticated();
        let status = match &session.status {
            // never downgrade an existing sudo session
            SessionStatus::Sudo { .. } => session.status.clone(),
            _ => SessionStatus::Authorized { user_id },
        };
        self.set_session_status_and_broadcast(session.id, status, was_authenticated)
            .await?;

        self.get_state(user_id).await
    }

    /// verify a totp code and upgrade the session to sudo mode
    pub async fn totp_sudo(
        &self,
        session: &Session,
        user_id: UserId,
        code: &str,
    ) -> Result<AuthState> {
        self.verify_totp_for_user(user_id, code).await?;
        self.set_session_status_and_broadcast(
            session.id,
            super::sudo_status(user_id),
            session.status.is_authenticated(),
        )
        .await?;

        self.get_state(user_id).await
    }

    /// generate a new set of recovery codes
    pub fn generate_recovery_codes(&self) -> Vec<String> {
        (0..RECOVERY_CODE_COUNT)
            .map(|_| Uuid::new_v4().to_string())
            .collect()
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

    /// consume a recovery code
    async fn consume_recovery_code(&self, user_id: UserId, code: &str) -> Result<()> {
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
        Ok(())
    }

    /// consume a recovery code and authenticate the session
    pub async fn recovery_code_exec(
        &self,
        session: &Session,
        user_id: UserId,
        code: &str,
    ) -> Result<AuthState> {
        self.consume_recovery_code(user_id, code).await?;

        let was_authenticated = session.status.is_authenticated();
        let status = match &session.status {
            // never downgrade an existing sudo session
            SessionStatus::Sudo { .. } => session.status.clone(),
            _ => SessionStatus::Authorized { user_id },
        };
        self.set_session_status_and_broadcast(session.id, status, was_authenticated)
            .await?;

        self.get_state(user_id).await
    }

    /// consume a recovery code and upgrade the session to sudo mode
    pub async fn recovery_code_sudo(
        &self,
        session: &Session,
        user_id: UserId,
        code: &str,
    ) -> Result<AuthState> {
        self.consume_recovery_code(user_id, code).await?;
        self.set_session_status_and_broadcast(
            session.id,
            super::sudo_status(user_id),
            session.status.is_authenticated(),
        )
        .await?;

        self.get_state(user_id).await
    }
}
