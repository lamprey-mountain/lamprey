use async_trait::async_trait;
use common::v1::types::application::Scope;
use common::v1::types::email::EmailAddr;
use common::v1::types::oauth::CodeChallengeMethod;
use common::v1::types::{ApplicationId, SessionId};
use sqlx::{query, query_scalar};

use crate::error::Result;
use crate::types::{EmailPurpose, UserId};

use crate::data::DataAuth;

use super::Postgres;

#[async_trait]
impl DataAuth for Postgres {
    async fn auth_oauth_put(
        &self,
        provider: String,
        user_id: UserId,
        remote_id: String,
        can_auth: bool,
    ) -> Result<()> {
        query!(
            "INSERT INTO oauth (provider, user_id, remote_id, can_auth) VALUES ($1, $2, $3, $4)",
            provider,
            user_id.into_inner(),
            remote_id,
            can_auth,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn auth_oauth_get_all(&self, user_id: UserId) -> Result<Vec<String>> {
        let providers = query_scalar!("SELECT provider FROM oauth WHERE user_id = $1", *user_id,)
            .fetch_all(&self.pool)
            .await?;
        Ok(providers)
    }

    async fn auth_oauth_get_remote(&self, provider: String, remote_id: String) -> Result<UserId> {
        let remote_id = query_scalar!(
            "SELECT user_id FROM oauth WHERE remote_id = $1 AND provider = $2",
            remote_id,
            provider,
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(remote_id.into())
    }

    async fn auth_oauth_delete(&self, provider: String, user_id: UserId) -> Result<()> {
        query!(
            "DELETE FROM oauth WHERE provider = $1 AND user_id = $2",
            provider,
            user_id.into_inner(),
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn auth_password_set(&self, user_id: UserId, hash: &[u8], salt: &[u8]) -> Result<()> {
        sqlx::query!(
            "update usr set password_hash = $2, password_salt = $3 where id = $1",
            *user_id,
            hash,
            salt
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn auth_password_get(&self, user_id: UserId) -> Result<Option<(Vec<u8>, Vec<u8>)>> {
        let row = sqlx::query!(
            "select password_hash, password_salt from usr where id = $1",
            *user_id,
        )
        .fetch_optional(&self.pool)
        .await?;
        let Some(row) = row else { return Ok(None) };
        match (row.password_hash, row.password_salt) {
            (Some(hash), Some(salt)) => Ok(Some((hash, salt))),
            _ => Ok(None),
        }
    }

    async fn auth_password_delete(&self, user_id: UserId) -> Result<()> {
        sqlx::query!(
            "update usr set password_hash = null, password_salt = null where id = $1",
            *user_id,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn auth_email_create(
        &self,
        code: String,
        addr: EmailAddr,
        session_id: SessionId,
        purpose: EmailPurpose,
    ) -> Result<()> {
        let purpose = match purpose {
            EmailPurpose::Authn => "Authn",
            EmailPurpose::Reset => "Reset",
        };
        sqlx::query!(
            "insert into email_auth_code (code, addr, session_id, expires_at, purpose) values ($1, $2, $3, now() + '10 minutes', $4)",
            code,
            addr.into_inner(),
            *session_id,
            purpose,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn auth_email_use(&self, code: String) -> Result<(EmailAddr, SessionId, EmailPurpose)> {
        let asdf = sqlx::query!(
            "delete from email_auth_code where code = $1 returning *",
            code,
        )
        .fetch_one(&self.pool)
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
        &self,
        code: String,
        application_id: ApplicationId,
        user_id: UserId,
        redirect_uri: String,
        scopes: Vec<Scope>,
        code_challenge: Option<String>,
        code_challenge_method: Option<CodeChallengeMethod>,
    ) -> Result<()> {
        let method = code_challenge_method.map(|m| match m {
            CodeChallengeMethod::S256 => "S256",
            CodeChallengeMethod::Plain => "plain",
        });

        sqlx::query!(
            r#"
            INSERT INTO oauth_authorization_code (code, application_id, user_id, redirect_uri, scopes, expires_at, code_challenge, code_challenge_method)
            VALUES ($1, $2, $3, $4, $5, now() + '10 minutes', $6, $7)
            "#,
            code,
            *application_id,
            *user_id,
            redirect_uri,
            serde_json::to_value(scopes).unwrap(),
            code_challenge,
            method,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn oauth_auth_code_use(
        &self,
        code: String,
    ) -> Result<(
        ApplicationId,
        UserId,
        String,
        Vec<Scope>,
        Option<String>,
        Option<String>,
    )> {
        let row = sqlx::query!(
            "DELETE FROM oauth_authorization_code WHERE code = $1 AND expires_at > now() RETURNING application_id, user_id, redirect_uri, scopes, code_challenge, code_challenge_method",
            code,
        )
        .fetch_one(&self.pool)
        .await?;

        let scopes: Vec<Scope> = serde_json::from_value(row.scopes).unwrap_or_default();

        Ok((
            row.application_id.into(),
            row.user_id.into(),
            row.redirect_uri,
            scopes,
            row.code_challenge,
            row.code_challenge_method,
        ))
    }

    async fn oauth_refresh_token_create(&self, token: String, session_id: SessionId) -> Result<()> {
        sqlx::query!(
            "INSERT INTO oauth_refresh_token (token, session_id) VALUES ($1, $2)",
            token,
            *session_id,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn oauth_refresh_token_use(&self, token: String) -> Result<SessionId> {
        let row = sqlx::query!(
            "DELETE FROM oauth_refresh_token WHERE token = $1 RETURNING session_id",
            token,
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(row.session_id.into())
    }
}
