use async_trait::async_trait;
use common::v1::types::email::{EmailAddr, EmailInfo};

use sqlx::{query, query_as, query_scalar};
use time::{Duration, OffsetDateTime, PrimitiveDateTime};
use uuid::Uuid;

use crate::data::DataUserEmail;
use crate::error::{Error, Result};
use crate::types::UserId;

use super::Postgres;

#[derive(sqlx::FromRow)]
struct DbUserEmail {
    addr: String,
    is_verified: bool,
    is_primary: bool,
}

#[async_trait]
impl DataUserEmail for Postgres {
    async fn user_email_add(
        &self,
        user_id: UserId,
        email: EmailInfo,
        max_user_emails: usize,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        let email_count: i64 =
            query_scalar("SELECT count(*) FROM user_email_addresses WHERE user_id = $1")
                .bind(*user_id)
                .fetch_one(&mut *tx)
                .await?;

        if email_count >= max_user_emails as i64 {
            return Err(Error::BadRequest(
                "Maximum number of email addresses reached.".to_string(),
            ));
        }

        let is_verified_by_anyone = query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM user_email_addresses WHERE addr = $1 AND is_verified = true)",
            email.email.as_ref()
        )
        .fetch_one(&mut *tx)
        .await?.unwrap_or(false);

        if is_verified_by_anyone {
            return Err(Error::BadRequest(
                "Email address is already in use.".to_string(),
            ));
        }

        let res = query!(
            "INSERT INTO user_email_addresses (user_id, addr, is_verified, is_primary) VALUES ($1, $2, $3, $4)",
            *user_id,
            email.email.into_inner(),
            email.is_verified,
            email.is_primary,
        )
        .execute(&mut *tx)
        .await;

        if let Err(e) = res {
            if let Some(db_err) = e.as_database_error() {
                if db_err.is_unique_violation() {
                    return Err(Error::EmailAlreadyExists);
                }
            }
            return Err(e.into());
        }

        tx.commit().await?;

        Ok(())
    }

    async fn user_email_delete(&self, user_id: UserId, email_addr: EmailAddr) -> Result<()> {
        let res = query!(
            "DELETE FROM user_email_addresses WHERE user_id = $1 AND addr = $2",
            *user_id,
            email_addr.into_inner()
        )
        .execute(&self.pool)
        .await?;

        if res.rows_affected() == 0 {
            return Err(Error::NotFound);
        }

        Ok(())
    }

    async fn user_email_list(&self, user_id: UserId) -> Result<Vec<EmailInfo>> {
        let db_emails = query_as!(
            DbUserEmail,
            r#"SELECT addr, is_verified, is_primary FROM user_email_addresses WHERE user_id = $1"#,
            *user_id,
        )
        .fetch_all(&self.pool)
        .await?;

        let mut emails = Vec::with_capacity(db_emails.len());
        for db_email in db_emails {
            emails.push(EmailInfo {
                email: db_email.addr.try_into()?,
                is_verified: db_email.is_verified,
                is_primary: db_email.is_primary,
            });
        }

        Ok(emails)
    }

    async fn user_email_verify_use(
        &self,
        user_id: UserId,
        email_addr: EmailAddr,
        code: String,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        let existing_verified_owner: Option<Uuid> = query_scalar(
            "SELECT user_id FROM user_email_addresses WHERE addr = $1 AND is_verified = true",
        )
        .bind(email_addr.as_ref())
        .fetch_optional(&mut *tx)
        .await?;

        if let Some(owner_id) = existing_verified_owner {
            if owner_id != *user_id {
                return Err(Error::BadRequest(
                    "Email address is already in use.".to_string(),
                ));
            }
        }

        let verification = query!(
            "SELECT expires_at FROM email_address_verification WHERE user_id = $1 AND addr = $2 AND code = $3 FOR UPDATE",
            *user_id,
            email_addr.as_ref(),
            code
        )
        .fetch_optional(&mut *tx)
        .await?;

        if let Some(verification) = verification {
            if OffsetDateTime::now_utc() < verification.expires_at.assume_utc() {
                // Code is valid and not expired.
                query!(
                    "UPDATE user_email_addresses SET is_verified = true WHERE user_id = $1 AND addr = $2",
                    *user_id,
                    email_addr.as_ref()
                )
                .execute(&mut *tx)
                .await?;

                // Delete other unverified entries for this email.
                query!(
                    "DELETE FROM user_email_addresses WHERE addr = $1 AND is_verified = false AND user_id != $2",
                    email_addr.as_ref(),
                    *user_id
                )
                .execute(&mut *tx)
                .await?;

                // Delete the used verification code.
                query!(
                    "DELETE FROM email_address_verification WHERE user_id = $1 AND addr = $2 AND code = $3",
                    *user_id,
                    email_addr.as_ref(),
                    code
                )
                .execute(&mut *tx)
                .await?;

                tx.commit().await?;
                return Ok(());
            }
        }

        tx.rollback().await?;
        Err(Error::InvalidCredentials)
    }

    async fn user_email_verify_create(
        &self,
        user_id: UserId,
        email_addr: EmailAddr,
    ) -> Result<String> {
        let code = ((Uuid::new_v4().as_u128() % 900_000) + 100_000).to_string();
        let expires_at = OffsetDateTime::now_utc() + Duration::minutes(15);

        let mut tx = self.pool.begin().await?;

        query!(
            "DELETE FROM email_address_verification WHERE user_id = $1 AND addr = $2",
            *user_id,
            email_addr.as_ref()
        )
        .execute(&mut *tx)
        .await?;

        query!(
            "INSERT INTO email_address_verification (code, addr, user_id, expires_at) VALUES ($1, $2, $3, $4)",
            code,
            email_addr.as_ref(),
            *user_id,
            PrimitiveDateTime::new(expires_at.date(), expires_at.time())
        )
        .execute(&mut *tx)
        .await?;

        let res = query!(
            "UPDATE user_email_addresses SET last_verification_email_sent_at = $1 WHERE user_id = $2 AND addr = $3",
            PrimitiveDateTime::new(OffsetDateTime::now_utc().date(), OffsetDateTime::now_utc().time()),
            *user_id,
            email_addr.as_ref(),
        )
        .execute(&mut *tx)
        .await?;

        if res.rows_affected() == 0 {
            tx.rollback().await?;
            return Err(Error::NotFound);
        }

        tx.commit().await?;

        Ok(code)
    }

    async fn user_email_lookup(&self, email_addr: &EmailAddr) -> Result<UserId> {
        let user_id = query_scalar!(
            "SELECT user_id FROM user_email_addresses WHERE addr = $1 AND is_verified = true",
            email_addr.as_ref()
        )
        .fetch_optional(&self.pool)
        .await?
        .ok_or(Error::NotFound)?;
        Ok(user_id.into())
    }
}
