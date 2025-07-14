use std::sync::Arc;

use common::v1::types::email::EmailAddr;
use lettre::{
    message::Mailbox, transport::smtp::authentication::Credentials, AsyncSmtpTransport,
    AsyncTransport, Message, Tokio1Executor,
};

use crate::{Error, Result, ServerStateInner};

pub struct ServiceEmail {
    state: Arc<ServerStateInner>,
    mailer: AsyncSmtpTransport<Tokio1Executor>,
}

impl ServiceEmail {
    pub fn new(state: Arc<ServerStateInner>) -> Self {
        let config = state.config.smtp.clone();
        let creds = Credentials::new(config.username, config.password);
        let mailer = AsyncSmtpTransport::<Tokio1Executor>::relay(&config.host)
            .unwrap()
            .credentials(creds)
            .build();
        Self { state, mailer }
    }

    pub async fn send(&self, to: EmailAddr, subject: String, body: String) -> Result<()> {
        let config = &self.state.config.smtp;
        let email = Message::builder()
            .from(Mailbox::new(
                Some("system".to_owned()),
                config.username.parse().unwrap(),
            ))
            .to(Mailbox::new(
                Some("remote".to_owned()),
                to.into_inner().parse().unwrap(),
            ))
            .date_now()
            .subject(subject)
            .body(body)
            .map_err(|e| Error::Internal(e.to_string()))?;
        self.mailer
            .send(email)
            .await
            .map_err(|e| Error::Internal(e.to_string()))?;
        Ok(())
    }

    pub async fn test(&self) -> Result<()> {
        self.mailer.test_connection().await.unwrap();
        Ok(())
    }
}
