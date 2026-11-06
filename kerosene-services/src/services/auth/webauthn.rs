//! webauthn (passkey) registration and authentication

use std::time::Instant;

use common::v1::types::AuditLogChange;
use common::v1::types::UserId;
use common::v1::types::WebauthnAuthenticatorId;
use common::v1::types::auth::{AuthState, WebauthnAuthenticator};
use common::v1::types::error::{ApiError, ErrorCode};
use common::v1::types::util::{Changes, Time};
use common::v1::types::{Session, SessionId, SessionStatus, User};
use webauthn_rs::prelude::{
    Passkey, PasskeyAuthentication, PasskeyRegistration, PublicKeyCredential,
    RegisterPublicKeyCredential, Webauthn,
};

use crate::prelude::*;

use super::ServiceAuth;

/// an in-flight webauthn ceremony
pub(super) enum PendingWebauthn {
    Register(Box<PasskeyRegistration>),
    Authenticate(Box<PasskeyAuthentication>),
    Sudo(Box<PasskeyAuthentication>),
}

// TODO: move audit log logic here
impl ServiceAuth {
    pub(super) fn webauthn(&self) -> Result<&Webauthn> {
        self.webauthn
            .as_ref()
            .ok_or_else(|| Error::Internal("webauthn is not configured".to_owned()))
    }

    pub(super) fn put_ceremony(&self, session_id: SessionId, ceremony: PendingWebauthn) {
        self.webauthn_states
            .insert(session_id, (ceremony, Instant::now()));
    }

    pub(super) fn take_ceremony(&self, session_id: SessionId) -> Result<PendingWebauthn> {
        self.webauthn_states
            .remove(&session_id)
            .map(|(_, (ceremony, _))| ceremony)
            .ok_or_else(|| {
                ApiError::with_message(
                    ErrorCode::InvalidData,
                    "no webauthn ceremony in progress".to_owned(),
                )
                .into()
            })
    }

    /// list a user's registered webauthn authenticators
    pub async fn webauthn_authenticators(
        &self,
        user_id: UserId,
    ) -> Result<Vec<WebauthnAuthenticator>> {
        let mut txn = self.state.begin_read().await?;
        Ok(txn
            .webauthn_credential_list(user_id)
            .await?
            .into_iter()
            .map(|credential| WebauthnAuthenticator {
                id: credential.id.into(),
                name: credential.name,
                created_at: credential.created_at.into(),
            })
            .collect())
    }

    /// rename a webauthn authenticator
    pub async fn webauthn_rename(
        &self,
        user_id: UserId,
        id: WebauthnAuthenticatorId,
        name: Option<&str>,
    ) -> Result<Vec<AuditLogChange>> {
        let Some(name) = name else {
            return Ok(Vec::new());
        };

        let mut txn = self.state.begin().await?;
        let credential = txn
            .webauthn_credential_get(user_id, id.into_inner())
            .await?
            .ok_or_else(|| ApiError::from_code(ErrorCode::UnknownAuthenticator))?;
        txn.webauthn_credential_rename(user_id, id.into_inner(), name)
            .await?;
        txn.commit().await?;

        Ok(Changes::new()
            .change("name", &credential.name, &name.to_owned())
            .build())
    }

    /// delete a webauthn authenticator
    pub async fn webauthn_delete(
        &self,
        user_id: UserId,
        id: WebauthnAuthenticatorId,
    ) -> Result<Vec<AuditLogChange>> {
        let mut txn = self.state.begin().await?;

        // serialize auth method removals for this user so concurrent deletes can't
        // both leave the user without a way to log in
        txn.user_lock(user_id).await?;

        let credential = txn
            .webauthn_credential_get(user_id, id.into_inner())
            .await?
            .ok_or_else(|| ApiError::from_code(ErrorCode::UnknownAuthenticator))?;
        txn.webauthn_credential_delete(user_id, id.into_inner())
            .await?;

        let has_authenticator = !txn.webauthn_credential_list(user_id).await?.is_empty();
        let has_email = !txn.user_email_list(user_id).await?.is_empty();
        let has_oauth = !txn.auth_oauth_get_all(user_id).await?.is_empty();
        if !has_authenticator && !has_email && !has_oauth {
            return Err(ApiError::from_code(ErrorCode::CannotRemoveLastAuthMethod).into());
        }

        txn.commit().await?;

        Ok(Changes::new()
            .remove("authenticator", &credential.name)
            .build())
    }

    /// start registering a new authenticator for a user
    pub async fn webauthn_register_challenge(
        &self,
        user: &User,
        session_id: SessionId,
    ) -> Result<String> {
        let existing = {
            let mut txn = self.state.begin_read().await?;
            txn.webauthn_credential_list(user.id).await?
        };
        let exclude_credentials = Some(
            existing
                .into_iter()
                .map(|credential| credential.credential_id)
                .collect(),
        );

        let (challenge, state) = self
            .webauthn()?
            .start_passkey_registration(
                user.id.into_inner(),
                &user.name,
                &user.name,
                exclude_credentials,
            )
            .map_err(|err| {
                Error::Internal(format!("failed to start webauthn registration: {err}"))
            })?;

        self.put_ceremony(session_id, PendingWebauthn::Register(Box::new(state)));

        serde_json::to_string(&challenge).map_err(|err| {
            Error::Internal(format!("failed to serialize webauthn challenge: {err}"))
        })
    }

    /// verify a registration response and store the new authenticator
    pub async fn webauthn_register_finish(
        &self,
        user: &User,
        session_id: SessionId,
        credential: &str,
    ) -> Result<WebauthnAuthenticator> {
        let PendingWebauthn::Register(state) = self.take_ceremony(session_id)? else {
            return Err(ApiError::with_message(
                ErrorCode::InvalidData,
                "no webauthn registration in progress".to_owned(),
            )
            .into());
        };

        let credential: RegisterPublicKeyCredential = serde_json::from_str(credential)
            .map_err(|_| ApiError::from_code(ErrorCode::InvalidData))?;
        let passkey = self
            .webauthn()?
            .finish_passkey_registration(&credential, &state)
            .map_err(|err| {
                tracing::debug!(?err, "webauthn registration verification failed");
                Error::ApiError(ApiError::with_message(
                    ErrorCode::InvalidData,
                    "webauthn registration failed".to_owned(),
                ))
            })?;

        let credential_id = passkey.cred_id().clone();
        let passkey_json = serde_json::to_string(&passkey)
            .map_err(|err| Error::Internal(format!("failed to serialize passkey: {err}")))?;
        let name = "passkey".to_owned();

        let mut txn = self.state.begin().await?;
        let id = txn
            .webauthn_credential_create(user.id, &name, &credential_id, &passkey_json)
            .await?;
        txn.commit().await?;

        // TODO: broadcast auth update sync event

        Ok(WebauthnAuthenticator {
            id: id.into(),
            name,
            created_at: Time::now_utc(),
        })
    }

    /// start authenticating with a registered authenticator
    ///
    /// requires a session that is already bound to a user; usernameless (discoverable
    /// credential) authentication is not implemented
    pub async fn webauthn_auth_challenge(
        &self,
        session_id: SessionId,
        user_id: Option<UserId>,
    ) -> Result<String> {
        self.webauthn_authentication_challenge(session_id, user_id, false)
            .await
    }

    /// start a webauthn ceremony to enter sudo mode
    pub async fn webauthn_sudo_challenge(
        &self,
        session_id: SessionId,
        user_id: UserId,
    ) -> Result<String> {
        self.webauthn_authentication_challenge(session_id, Some(user_id), true)
            .await
    }

    async fn webauthn_authentication_challenge(
        &self,
        session_id: SessionId,
        user_id: Option<UserId>,
        sudo: bool,
    ) -> Result<String> {
        let Some(user_id) = user_id else {
            return Err(ApiError::with_message(
                ErrorCode::MissingAuth,
                "webauthn login requires an authenticated session".to_owned(),
            )
            .into());
        };

        let passkeys: Vec<Passkey> = {
            let mut txn = self.state.begin_read().await?;
            txn.webauthn_credential_list(user_id)
                .await?
                .into_iter()
                .filter_map(|credential| match serde_json::from_str(&credential.passkey) {
                    Ok(passkey) => Some(passkey),
                    Err(err) => {
                        tracing::error!(?err, id = %credential.id, "failed to deserialize stored passkey");
                        None
                    }
                })
                .collect()
        };

        if passkeys.is_empty() {
            return Err(ApiError::with_message(
                ErrorCode::UnknownAuthenticator,
                "no registered webauthn authenticators".to_owned(),
            )
            .into());
        }

        let (challenge, state) = self
            .webauthn()?
            .start_passkey_authentication(&passkeys)
            .map_err(|err| {
                Error::Internal(format!("failed to start webauthn authentication: {err}"))
            })?;

        let ceremony = if sudo {
            PendingWebauthn::Sudo(Box::new(state))
        } else {
            PendingWebauthn::Authenticate(Box::new(state))
        };
        self.put_ceremony(session_id, ceremony);

        serde_json::to_string(&challenge).map_err(|err| {
            Error::Internal(format!("failed to serialize webauthn challenge: {err}"))
        })
    }

    /// verify an assertion, returning the credential's user
    async fn webauthn_verify_assertion(
        &self,
        session: &Session,
        state: &PasskeyAuthentication,
        credential: &str,
    ) -> Result<UserId> {
        let credential: PublicKeyCredential = serde_json::from_str(credential)
            .map_err(|_| ApiError::from_code(ErrorCode::InvalidData))?;
        let result = self
            .webauthn()?
            .finish_passkey_authentication(&credential, state)
            .map_err(|err| {
                tracing::debug!(?err, "webauthn assertion verification failed");
                Error::ApiError(ApiError::with_message(
                    ErrorCode::InvalidOrExpiredCode,
                    "webauthn authentication failed".to_owned(),
                ))
            })?;

        let credential_id = result.cred_id().clone();

        let mut txn = self.state.begin().await?;
        let stored = txn
            .webauthn_credential_get_by_credential_id(&credential_id)
            .await?
            .ok_or_else(|| ApiError::from_code(ErrorCode::UnknownAuthenticator))?;

        let user_id: UserId = stored.user_id.into();
        if session.user_id() != Some(user_id) {
            return Err(ApiError::with_message(
                ErrorCode::UnknownAuthenticator,
                "authenticator does not belong to this session's user".to_owned(),
            )
            .into());
        }

        let mut passkey: Passkey = serde_json::from_str(&stored.passkey)
            .map_err(|err| Error::Internal(format!("failed to deserialize passkey: {err}")))?;
        if passkey.update_credential(&result).is_some() {
            let passkey_json = serde_json::to_string(&passkey)
                .map_err(|err| Error::Internal(format!("failed to serialize passkey: {err}")))?;
            txn.webauthn_credential_update_passkey(user_id, stored.id, &passkey_json)
                .await?;
        }
        txn.commit().await?;

        Ok(user_id)
    }

    /// verify an assertion and authenticate the session
    ///
    /// returns the authenticated user and their auth state
    pub async fn webauthn_auth_finish(
        &self,
        session: &Session,
        credential: &str,
    ) -> Result<(UserId, AuthState)> {
        let PendingWebauthn::Authenticate(state) = self.take_ceremony(session.id)? else {
            return Err(ApiError::with_message(
                ErrorCode::InvalidData,
                "no webauthn authentication in progress".to_owned(),
            )
            .into());
        };

        let user_id = self
            .webauthn_verify_assertion(session, &state, credential)
            .await?;
        let was_authenticated = session.status.is_authenticated();
        let status = match &session.status {
            // never downgrade an existing sudo session
            SessionStatus::Sudo { .. } => session.status.clone(),
            _ => SessionStatus::Authorized { user_id },
        };
        self.set_session_status_and_broadcast(session.id, status, was_authenticated)
            .await?;

        Ok((user_id, self.get_state(user_id).await?))
    }

    /// verify an assertion and upgrade the session to sudo mode
    pub async fn webauthn_sudo_finish(
        &self,
        session: &Session,
        credential: &str,
    ) -> Result<(UserId, AuthState)> {
        let PendingWebauthn::Sudo(state) = self.take_ceremony(session.id)? else {
            return Err(ApiError::with_message(
                ErrorCode::InvalidData,
                "no webauthn sudo ceremony in progress".to_owned(),
            )
            .into());
        };

        let user_id = self
            .webauthn_verify_assertion(session, &state, credential)
            .await?;
        self.set_session_status_and_broadcast(
            session.id,
            super::sudo_status(user_id),
            session.status.is_authenticated(),
        )
        .await?;

        Ok((user_id, self.get_state(user_id).await?))
    }
}
