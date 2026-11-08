use async_trait::async_trait;
use common::v1::types::application::Scopes;
use common::v1::types::email::EmailAddr;
use common::v1::types::error::{ApiError, ErrorCode};
use common::v1::types::util::Time;
use common::v1::types::{ApplicationId, SessionId};
use sqlx::{query, query_as, query_scalar};
use uuid::Uuid;

use crate::error::{Error, Result};
use crate::types::{DbWebauthnCredential, EmailPurpose, UserId};

use crate::data::DataAuth;

use super::Postgres;

#[async_trait]
impl DataAuth for Postgres {
    async fn auth_oauth_put(
        &mut self,
        provider: String,
        user_id: UserId,
        remote_id: String,
        can_auth: bool,
    ) -> Result<()> {
        let mut conn = self.acquire().await?;
        query!(
            "INSERT INTO oauth (provider, user_id, remote_id, can_auth) VALUES ($1, $2, $3, $4)",
            provider,
            user_id.into_inner(),
            remote_id,
            can_auth,
        )
        .execute(conn.ext())
        .await?;
        Ok(())
    }

    async fn auth_oauth_get_all(&mut self, user_id: UserId) -> Result<Vec<String>> {
        let mut conn = self.acquire().await?;
        let providers = query_scalar!("SELECT provider FROM oauth WHERE user_id = $1", *user_id,)
            .fetch_all(conn.ext())
            .await?;
        Ok(providers)
    }

    async fn auth_oauth_get_remote(
        &mut self,
        provider: String,
        remote_id: String,
    ) -> Result<Option<UserId>> {
        let mut conn = self.acquire().await?;
        let remote_id = query_scalar!(
            "SELECT user_id FROM oauth WHERE remote_id = $1 AND provider = $2",
            remote_id,
            provider,
        )
        .fetch_optional(conn.ext())
        .await?
        .map(|i| i.into());
        Ok(remote_id)
    }

    async fn auth_oauth_delete(&mut self, provider: String, user_id: UserId) -> Result<()> {
        let mut conn = self.acquire().await?;
        query!(
            "DELETE FROM oauth WHERE provider = $1 AND user_id = $2",
            provider,
            user_id.into_inner(),
        )
        .execute(conn.ext())
        .await?;
        Ok(())
    }

    async fn auth_password_set(&mut self, user_id: UserId, hash: &[u8], salt: &[u8]) -> Result<()> {
        let mut conn = self.acquire().await?;
        query!(
            "update usr set password_hash = $2, password_salt = $3 where id = $1",
            *user_id,
            hash,
            salt
        )
        .execute(conn.ext())
        .await?;
        Ok(())
    }

    async fn auth_password_get(&mut self, user_id: UserId) -> Result<Option<(Vec<u8>, Vec<u8>)>> {
        let mut conn = self.acquire().await?;
        let row = query!(
            "select password_hash, password_salt from usr where id = $1",
            *user_id,
        )
        .fetch_optional(conn.ext())
        .await?;
        let Some(row) = row else { return Ok(None) };
        match (row.password_hash, row.password_salt) {
            (Some(hash), Some(salt)) => Ok(Some((hash, salt))),
            _ => Ok(None),
        }
    }

    async fn auth_password_delete(&mut self, user_id: UserId) -> Result<()> {
        let mut conn = self.acquire().await?;
        query!(
            "update usr set password_hash = null, password_salt = null where id = $1",
            *user_id,
        )
        .execute(conn.ext())
        .await?;
        Ok(())
    }

    async fn auth_email_create(
        &mut self,
        code: String,
        addr: EmailAddr,
        session_id: SessionId,
        purpose: EmailPurpose,
    ) -> Result<()> {
        let mut conn = self.acquire().await?;
        let purpose = match purpose {
            EmailPurpose::Authn => "Authn",
            EmailPurpose::Reset => "Reset",
        };
        query!(
            "insert into email_auth_code (code, addr, session_id, expires_at, purpose) values ($1, $2, $3, now() + '10 minutes', $4)",
            code,
            addr.into_inner(),
            *session_id,
            purpose,
        )
        .execute(conn.ext())
        .await?;
        Ok(())
    }

    async fn auth_email_use(
        &mut self,
        code: String,
    ) -> Result<(EmailAddr, SessionId, EmailPurpose)> {
        let mut conn = self.acquire().await?;
        let asdf = query!(
            "delete from email_auth_code where code = $1 returning *",
            code,
        )
        .fetch_one(conn.ext())
        .await?;
        let purpose = match asdf.purpose.as_str() {
            "Authn" => EmailPurpose::Authn,
            "Reset" => EmailPurpose::Reset,
            purpose => panic!("invalid data in db: unknown email purpose {purpose}"),
        };
        Ok((
            asdf.addr.try_into().expect("invalid data in db"),
            asdf.session_id.into(),
            purpose,
        ))
    }

    async fn oauth_auth_code_create(
        &mut self,
        code: String,
        application_id: ApplicationId,
        user_id: UserId,
        redirect_uri: String,
        scopes: Scopes,
        code_challenge: Option<String>,
        code_challenge_method: Option<String>,
        auth_time: Option<Time>,
    ) -> Result<()> {
        let mut conn = self.acquire().await?;
        query!(
            r#"
            INSERT INTO oauth_authorization_code (code, application_id, user_id, redirect_uri, scopes, expires_at, code_challenge, code_challenge_method, auth_time)
            VALUES ($1, $2, $3, $4, $5, now() + '10 minutes', $6, $7, $8)
            "#,
            code,
            *application_id,
            *user_id,
            redirect_uri,
            serde_json::to_value(scopes).unwrap(),
            code_challenge,
            code_challenge_method,
            auth_time.map(time::PrimitiveDateTime::from),
        )
        .execute(conn.ext())
        .await?;
        Ok(())
    }

    async fn oauth_auth_code_use(
        &mut self,
        code: String,
    ) -> Result<(
        ApplicationId,
        UserId,
        String,
        Scopes,
        Option<String>,
        Option<String>,
        Option<Time>,
    )> {
        let mut conn = self.acquire().await?;
        let row = query!(
            "DELETE FROM oauth_authorization_code WHERE code = $1 AND expires_at > now() RETURNING application_id, user_id, redirect_uri, scopes, code_challenge, code_challenge_method, auth_time",
            code,
        )
        .fetch_one(conn.ext())
        .await?;

        let scopes: Scopes = serde_json::from_value(row.scopes).unwrap_or_default();

        Ok((
            row.application_id.into(),
            row.user_id.into(),
            row.redirect_uri,
            scopes,
            row.code_challenge,
            row.code_challenge_method,
            row.auth_time.map(Into::into),
        ))
    }

    async fn oauth_refresh_token_create(
        &mut self,
        token: String,
        session_id: SessionId,
    ) -> Result<()> {
        let mut conn = self.acquire().await?;
        query!(
            "INSERT INTO oauth_refresh_token (token, session_id, created_at) VALUES ($1, $2, now())",
            token,
            *session_id,
        )
        .execute(conn.ext())
        .await?;
        Ok(())
    }

    async fn oauth_refresh_token_use(&mut self, token: String) -> Result<SessionId> {
        let mut conn = self.acquire().await?;
        let row = query!(
            "DELETE FROM oauth_refresh_token WHERE token = $1 RETURNING session_id",
            token,
        )
        .fetch_one(conn.ext())
        .await?;
        Ok(row.session_id.into())
    }

    async fn auth_totp_set(
        &mut self,
        user_id: UserId,
        secret: Option<String>,
        enabled: bool,
    ) -> Result<()> {
        let mut tx = self.begin_tx().await?;

        query!(
            "update usr set totp_secret = $2, totp_enabled = $3 where id = $1",
            *user_id,
            secret,
            enabled
        )
        .execute(tx.ext())
        .await?;

        if secret.is_none() {
            // If secret is set to None, delete all recovery codes
            query!("delete from totp_recovery where user_id = $1", *user_id)
                .execute(tx.ext())
                .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    async fn auth_totp_get(&mut self, user_id: UserId) -> Result<Option<(String, bool)>> {
        let mut conn = self.acquire().await?;
        let row = query!(
            "select totp_secret, totp_enabled from usr where id = $1",
            *user_id,
        )
        .fetch_optional(conn.ext())
        .await?;

        Ok(row.and_then(|r| r.totp_secret.map(|secret| (secret, r.totp_enabled))))
    }

    async fn auth_totp_recovery_generate(
        &mut self,
        user_id: UserId,
        codes: &[String],
    ) -> Result<()> {
        let mut tx = self.begin_tx().await?;

        query!("delete from totp_recovery where user_id = $1", *user_id)
            .execute(tx.ext())
            .await?;

        for code in codes {
            query!(
                "insert into totp_recovery (user_id, code) values ($1, $2)",
                *user_id,
                code
            )
            .execute(tx.ext())
            .await?;
        }

        tx.commit().await?;

        Ok(())
    }

    async fn auth_totp_recovery_get_all(
        &mut self,
        user_id: UserId,
    ) -> Result<Vec<(String, Option<Time>)>> {
        let mut conn = self.acquire().await?;
        Ok(query!(
            "select code, used_at from totp_recovery where user_id = $1",
            *user_id
        )
        .fetch_all(conn.ext())
        .await?
        .into_iter()
        .map(|r| (r.code, r.used_at.map(|t| t.into())))
        .collect())
    }

    async fn auth_totp_recovery_use(&mut self, user_id: UserId, code: &str) -> Result<()> {
        let mut conn = self.acquire().await?;
        let rows_affected = query!(
            "update totp_recovery set used_at = now() where user_id = $1 and code = $2 and used_at is null",
            *user_id,
            code
        )
        .execute(conn.ext())
        .await?
        .rows_affected();

        if rows_affected == 0 {
            return Err(crate::error::Error::BadStatic("invalid or used code"));
        }

        Ok(())
    }

    async fn auth_totp_recovery_delete_all(&mut self, user_id: UserId) -> Result<()> {
        let mut conn = self.acquire().await?;
        query!("delete from totp_recovery where user_id = $1", *user_id)
            .execute(conn.ext())
            .await?;
        Ok(())
    }

    async fn user_lock(&mut self, user_id: UserId) -> Result<()> {
        let mut conn = self.acquire().await?;
        query!("SELECT id FROM usr WHERE id = $1 FOR UPDATE", *user_id)
            .fetch_one(conn.ext())
            .await?;
        Ok(())
    }

    async fn webauthn_credential_list(
        &mut self,
        user_id: UserId,
    ) -> Result<Vec<DbWebauthnCredential>> {
        let mut conn = self.acquire().await?;
        Ok(query_as!(
            DbWebauthnCredential,
            "SELECT id, user_id, name, credential_id, passkey, created_at, last_used_at FROM webauthn_credential WHERE user_id = $1 ORDER BY created_at",
            *user_id,
        )
        .fetch_all(conn.ext())
        .await?)
    }

    async fn webauthn_credential_get(
        &mut self,
        user_id: UserId,
        id: Uuid,
    ) -> Result<Option<DbWebauthnCredential>> {
        let mut conn = self.acquire().await?;
        Ok(query_as!(
            DbWebauthnCredential,
            "SELECT id, user_id, name, credential_id, passkey, created_at, last_used_at FROM webauthn_credential WHERE user_id = $1 AND id = $2",
            *user_id,
            id,
        )
        .fetch_optional(conn.ext())
        .await?)
    }

    async fn webauthn_credential_get_by_credential_id(
        &mut self,
        credential_id: &[u8],
    ) -> Result<Option<DbWebauthnCredential>> {
        let mut conn = self.acquire().await?;
        Ok(query_as!(
            DbWebauthnCredential,
            "SELECT id, user_id, name, credential_id, passkey, created_at, last_used_at FROM webauthn_credential WHERE credential_id = $1",
            credential_id,
        )
        .fetch_optional(conn.ext())
        .await?)
    }

    async fn webauthn_credential_create(
        &mut self,
        user_id: UserId,
        name: &str,
        credential_id: &[u8],
        passkey: &str,
    ) -> Result<Uuid> {
        let mut txn = self.begin_tx().await?;
        let id = Uuid::new_v4();
        query!(
            "INSERT INTO webauthn_credential (id, user_id, name, credential_id, passkey) VALUES ($1, $2, $3, $4, $5)",
            id,
            *user_id,
            name,
            credential_id,
            passkey,
        )
        .execute(txn.ext())
        .await
        .map_err(|err| match &err {
            sqlx::Error::Database(db) if db.is_unique_violation() => Error::ApiError(
                ApiError::from_code(ErrorCode::AuthenticatorAlreadyRegistered),
            ),
            _ => Error::Sqlx(err),
        })?;
        txn.commit().await?;
        Ok(id)
    }

    async fn webauthn_credential_update_passkey(
        &mut self,
        user_id: UserId,
        id: Uuid,
        passkey: &str,
    ) -> Result<()> {
        let mut conn = self.acquire().await?;
        let res = query!(
            "UPDATE webauthn_credential SET passkey = $3, last_used_at = now() WHERE user_id = $1 AND id = $2",
            *user_id,
            id,
            passkey,
        )
        .execute(conn.ext())
        .await?;
        if res.rows_affected() == 0 {
            return Err(Error::ApiError(ApiError::from_code(
                ErrorCode::UnknownAuthenticator,
            )));
        }
        Ok(())
    }

    async fn webauthn_credential_rename(
        &mut self,
        user_id: UserId,
        id: Uuid,
        name: &str,
    ) -> Result<()> {
        let mut conn = self.acquire().await?;
        let res = query!(
            "UPDATE webauthn_credential SET name = $3 WHERE user_id = $1 AND id = $2",
            *user_id,
            id,
            name,
        )
        .execute(conn.ext())
        .await?;
        if res.rows_affected() == 0 {
            return Err(Error::ApiError(ApiError::from_code(
                ErrorCode::UnknownAuthenticator,
            )));
        }
        Ok(())
    }

    async fn webauthn_credential_delete(&mut self, user_id: UserId, id: Uuid) -> Result<()> {
        let mut conn = self.acquire().await?;
        let res = query!(
            "DELETE FROM webauthn_credential WHERE user_id = $1 AND id = $2",
            *user_id,
            id,
        )
        .execute(conn.ext())
        .await?;
        if res.rows_affected() == 0 {
            return Err(Error::ApiError(ApiError::from_code(
                ErrorCode::UnknownAuthenticator,
            )));
        }
        Ok(())
    }
}
