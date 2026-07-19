use std::{sync::Arc, time::Duration};

use common::v1::types::email::EmailAddr;
use lettre::{
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
    message::{Mailbox, MultiPart},
    transport::smtp::authentication::Credentials,
};
use tokio::time::sleep;
use tracing::{error, info};

use crate::{Error, Result, ServerStateInner};

pub struct ServiceEmail {
    state: Arc<ServerStateInner>,
    mailer: AsyncSmtpTransport<Tokio1Executor>,
}

impl ServiceEmail {
    pub fn new(state: Arc<ServerStateInner>) -> Self {
        let config = state.config.smtp.clone();
        let password = config
            .password
            .load()
            .expect("TODO: better error handling")
            .to_string();
        let creds = Credentials::new(config.username, password);
        let mailer = AsyncSmtpTransport::<Tokio1Executor>::relay(&config.host)
            .unwrap()
            .credentials(creds)
            .build();

        Self { state, mailer }
    }

    pub fn start_background_tasks(&self) {
        let num_workers = self.state.config.email_queue_workers;
        info!("Starting {} email queue workers", num_workers);

        for i in 0..num_workers {
            info!("Email worker {} started", i);
            tokio::spawn(Self::worker(Arc::clone(&self.state)));
        }
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

    async fn worker(state: Arc<ServerStateInner>) {
        loop {
            let srv = state.services();
            match srv.email.process_email_queue_item().await {
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
        let mut data = self.state.data();
        let email_item = data.email_queue_claim().await?;

        if let Some(item) = email_item {
            info!("Claimed email with ID: {}", item.id);
            let body = MultiPart::alternative_plain_html(
                item.plain_text_body.clone(),
                item.html_body
                    .unwrap_or_else(|| html_escape(item.plain_text_body)),
            );
            let email = Message::builder()
                .from(Mailbox::new(
                    Some("system".to_owned()),
                    self.state.config.smtp.username.parse().unwrap(),
                ))
                .to(Mailbox::new(
                    Some("user".to_owned()),
                    item.to_addr.parse().unwrap(),
                ))
                .date_now()
                .subject(item.subject)
                .multipart(body)
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

fn html_escape(s: String) -> String {
    s.replace("&", "&amp;")
        .replace(">", "&lt;")
        .replace("<", "&gt;")
        .replace("\"", "&quot;")
}
