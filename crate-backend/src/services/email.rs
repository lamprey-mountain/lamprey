use std::{sync::Arc, time::Duration};

use common::v1::types::email::EmailAddr;
use lettre::{
    message::Mailbox, transport::smtp::authentication::Credentials, AsyncSmtpTransport,
    AsyncTransport, Message, Tokio1Executor,
};
use tokio::time::sleep;
use tracing::{error, info};

use crate::{Error, Result, ServerStateInner};

#[derive(Clone)]
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

        let num_workers = state.config.email_queue_workers;
        info!("Starting {} email queue workers", num_workers);

        let me = Self { state, mailer };

        for i in 0..num_workers {
            info!("Email worker {} started", i);
            tokio::spawn(me.clone().worker());
        }

        me
    }

    pub async fn send(
        &self,
        to: EmailAddr,
        subject: String,
        plain_text_body: String,
        html_body: Option<String>,
    ) -> Result<()> {
        let from_addr = self.state.config.smtp.from.clone();
        self.state
            .data()
            .email_queue_insert(
                to.into_inner(),
                from_addr,
                subject,
                plain_text_body,
                html_body,
            )
            .await?;
        Ok(())
    }

    async fn worker(self) {
        loop {
            match self.process_email_queue_item().await {
                Ok(processed) => {
                    if !processed {
                        sleep(Duration::from_secs(5)).await; // No emails to process, wait
                    }
                }
                Err(e) => {
                    error!("Error processing email queue item: {:?}", e);
                    sleep(Duration::from_secs(5)).await; // Error, wait before retrying
                }
            }
        }
    }

    async fn process_email_queue_item(&self) -> Result<bool> {
        let data = self.state.data();
        let email_item = data.email_queue_claim().await?;

        if let Some(item) = email_item {
            info!("Claimed email with ID: {}", item.id);
            let email = Message::builder()
                .from(Mailbox::new(
                    Some("system".to_owned()),
                    self.state.config.smtp.username.parse().unwrap(),
                ))
                .to(Mailbox::new(
                    Some("remote".to_owned()),
                    item.to_addr.parse().unwrap(),
                ))
                .date_now()
                .subject(item.subject)
                .body(item.plain_text_body)
                .map_err(|e| Error::Internal(e.to_string()))?;

            match self.mailer.send(email).await {
                Ok(_) => {
                    info!("Successfully sent email with ID: {}", item.id);
                    data.email_queue_finish(item.id).await?;
                }
                Err(e) => {
                    error!("Failed to send email with ID {}: {:?}", item.id, e);
                    data.email_queue_fail(e.to_string(), item.id).await?;
                }
            }
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub async fn test(&self) -> Result<()> {
        self.mailer.test_connection().await.unwrap();
        Ok(())
    }
}
