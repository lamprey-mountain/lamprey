use async_trait::async_trait;
use common::v1::types::UserId;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::types::MessageRef;
use crate::Result;

pub trait Queue: QueueUrl + QueueEmail + QueueNotification + QueueSearch {}

#[async_trait]
pub trait QueueUrl {
    async fn url_push(
        &self,
        message_ref: Option<MessageRef>,
        user_id: UserId,
        url: String,
    ) -> Result<Uuid>;
}

#[async_trait]
pub trait QueueEmail {
    async fn email_push(
        &self,
        to: String,
        from: String,
        subject: String,
        plain_text_body: String,
        html_body: Option<String>,
    ) -> Result<Uuid>;
}

#[async_trait]
pub trait QueueNotification {
    // ???
}

#[async_trait]
pub trait QueueSearch {
    // async fn search_reindex_push(
    //     &self,
    //     channel_id: ChannelId,
    //     last_message_id: Option<MessageId>,
    // ) -> Result<()>;
}

/// a wrapper around postgres
///
/// only used if nats isnt available
pub struct PostgresQueue;

/// a wrapper around nats jetstream
///
/// generally preferred over postgres
pub struct JetstreamQueue {
    context: async_nats::jetstream::Context,
}

impl JetstreamQueue {
    pub fn new(client: async_nats::Client) -> Self {
        Self {
            context: async_nats::jetstream::new(client),
        }
    }

    /// initialize all jetstream streams for queues
    pub async fn init_streams(&self) -> Result<()> {
        // URL embed queue stream
        self.context
            .get_or_create_stream(async_nats::jetstream::stream::Config {
                name: "QUEUE_URL_EMBED".to_string(),
                subjects: vec!["queue.url_embed".to_string()],
                storage: async_nats::jetstream::stream::StorageType::File,
                retention: async_nats::jetstream::stream::RetentionPolicy::WorkQueue,
                ..Default::default()
            })
            .await
            .map_err(|e| crate::Error::NatsJetstream(format!("Create stream failed: {}", e)))?;

        // Email queue stream
        self.context
            .get_or_create_stream(async_nats::jetstream::stream::Config {
                name: "QUEUE_EMAIL".to_string(),
                subjects: vec!["queue.email".to_string()],
                storage: async_nats::jetstream::stream::StorageType::File,
                retention: async_nats::jetstream::stream::RetentionPolicy::WorkQueue,
                ..Default::default()
            })
            .await
            .map_err(|e| crate::Error::NatsJetstream(format!("Create stream failed: {}", e)))?;

        // Notification queue stream
        self.context
            .get_or_create_stream(async_nats::jetstream::stream::Config {
                name: "QUEUE_NOTIFICATION".to_string(),
                subjects: vec!["queue.notification".to_string()],
                storage: async_nats::jetstream::stream::StorageType::File,
                retention: async_nats::jetstream::stream::RetentionPolicy::WorkQueue,
                ..Default::default()
            })
            .await
            .map_err(|e| crate::Error::NatsJetstream(format!("Create stream failed: {}", e)))?;

        // Search queue stream
        self.context
            .get_or_create_stream(async_nats::jetstream::stream::Config {
                name: "QUEUE_SEARCH".to_string(),
                subjects: vec!["queue.search".to_string()],
                storage: async_nats::jetstream::stream::StorageType::File,
                retention: async_nats::jetstream::stream::RetentionPolicy::WorkQueue,
                ..Default::default()
            })
            .await
            .map_err(|e| crate::Error::NatsJetstream(format!("Create stream failed: {}", e)))?;

        Ok(())
    }

    /// publish a message to a jetstream stream
    async fn publish(&self, subject: &str, payload: impl Serialize) -> Result<Uuid> {
        let id = Uuid::new_v4();
        let envelope = QueueEnvelope {
            id,
            payload,
            created_at: common::v1::types::util::Time::now_utc(),
        };
        let bytes = serde_json::to_vec(&envelope)?;
        let subject = subject.to_string();
        self.context
            .publish(subject, bytes.into())
            .await
            .map_err(|e| crate::Error::NatsJetstream(format!("NATS publish failed: {}", e)))?;
        Ok(id)
    }
}

/// internal envelope for queue items
#[derive(Debug, Clone, Serialize, Deserialize)]
struct QueueEnvelope<T> {
    id: Uuid,
    payload: T,
    created_at: common::v1::types::util::Time,
}

/// URL embed queue item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UrlEmbedQueueItem {
    pub message_ref: Option<MessageRef>,
    pub user_id: UserId,
    pub url: String,
}

/// Email queue item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailQueueItem {
    pub to: String,
    pub from: String,
    pub subject: String,
    pub plain_text_body: String,
    pub html_body: Option<String>,
}

#[async_trait]
impl QueueUrl for JetstreamQueue {
    async fn url_push(
        &self,
        message_ref: Option<MessageRef>,
        user_id: UserId,
        url: String,
    ) -> Result<Uuid> {
        let item = UrlEmbedQueueItem {
            message_ref,
            user_id,
            url,
        };
        self.publish("queue.url_embed", item).await
    }
}

#[async_trait]
impl QueueEmail for JetstreamQueue {
    async fn email_push(
        &self,
        to: String,
        from: String,
        subject: String,
        plain_text_body: String,
        html_body: Option<String>,
    ) -> Result<Uuid> {
        let item = EmailQueueItem {
            to,
            from,
            subject,
            plain_text_body,
            html_body,
        };
        self.publish("queue.email", item).await
    }
}

#[async_trait]
impl QueueNotification for JetstreamQueue {
    // TODO: implement notification queue
}

#[async_trait]
impl QueueSearch for JetstreamQueue {
    // TODO: implement search queue
}

impl Queue for JetstreamQueue {}

/// consumer for processing URL embed queue
pub struct UrlEmbedConsumer {
    consumer:
        async_nats::jetstream::consumer::Consumer<async_nats::jetstream::consumer::pull::Config>,
}

impl UrlEmbedConsumer {
    pub async fn new(js: &JetstreamQueue) -> Result<Self> {
        let stream = js
            .context
            .get_stream("QUEUE_URL_EMBED")
            .await
            .map_err(|e| crate::Error::NatsJetstream(format!("NATS get stream failed: {}", e)))?;

        let consumer = stream
            .get_or_create_consumer(
                "url_embed_worker",
                async_nats::jetstream::consumer::pull::Config {
                    durable_name: Some("url_embed_worker".to_string()),
                    ack_policy: async_nats::jetstream::consumer::AckPolicy::Explicit,
                    max_deliver: 3,
                    ..Default::default()
                },
            )
            .await
            .map_err(|e| {
                crate::Error::NatsJetstream(format!("NATS create consumer failed: {}", e))
            })?;

        Ok(Self { consumer })
    }

    pub async fn next(&self) -> Result<Option<(Uuid, UrlEmbedQueueItem)>> {
        use futures_util::StreamExt;

        let mut messages = self
            .consumer
            .stream()
            .max_messages_per_batch(100)
            .messages()
            .await
            .map_err(|e| crate::Error::NatsJetstream(format!("NATS stream failed: {}", e)))?;

        while let Some(message) = messages.next().await {
            let message = message
                .map_err(|e| crate::Error::NatsJetstream(format!("NATS message failed: {}", e)))?;

            match serde_json::from_slice::<QueueEnvelope<UrlEmbedQueueItem>>(&message.payload) {
                Ok(envelope) => {
                    // acknowledge the message
                    message.ack().await.map_err(|e| {
                        crate::Error::NatsJetstream(format!("NATS ack failed: {}", e))
                    })?;
                    return Ok(Some((envelope.id, envelope.payload)));
                }
                Err(e) => {
                    tracing::error!("Failed to deserialize URL embed queue item: {}", e);
                    // NAK the message to redeliver - signals that the message will not be processed now
                    message
                        .ack_with(async_nats::jetstream::AckKind::Nak(None))
                        .await
                        .ok();
                }
            }
        }

        Ok(None)
    }
}

/// consumer for processing email queue
pub struct EmailConsumer {
    consumer:
        async_nats::jetstream::consumer::Consumer<async_nats::jetstream::consumer::pull::Config>,
}

impl EmailConsumer {
    pub async fn new(js: &JetstreamQueue) -> Result<Self> {
        let stream =
            js.context.get_stream("QUEUE_EMAIL").await.map_err(|e| {
                crate::Error::NatsJetstream(format!("NATS get stream failed: {}", e))
            })?;

        let consumer = stream
            .get_or_create_consumer(
                "email_worker",
                async_nats::jetstream::consumer::pull::Config {
                    durable_name: Some("email_worker".to_string()),
                    ack_policy: async_nats::jetstream::consumer::AckPolicy::Explicit,
                    max_deliver: 3,
                    ..Default::default()
                },
            )
            .await
            .map_err(|e| {
                crate::Error::NatsJetstream(format!("NATS create consumer failed: {}", e))
            })?;

        Ok(Self { consumer })
    }

    pub async fn next(&self) -> Result<Option<(Uuid, EmailQueueItem)>> {
        use futures_util::StreamExt;

        let mut messages = self
            .consumer
            .stream()
            .max_messages_per_batch(100)
            .messages()
            .await
            .map_err(|e| crate::Error::NatsJetstream(format!("NATS stream failed: {}", e)))?;

        while let Some(message) = messages.next().await {
            let message = message
                .map_err(|e| crate::Error::NatsJetstream(format!("NATS message failed: {}", e)))?;

            match serde_json::from_slice::<QueueEnvelope<EmailQueueItem>>(&message.payload) {
                Ok(envelope) => {
                    message.ack().await.map_err(|e| {
                        crate::Error::NatsJetstream(format!("NATS ack failed: {}", e))
                    })?;
                    return Ok(Some((envelope.id, envelope.payload)));
                }
                Err(e) => {
                    tracing::error!("Failed to deserialize email queue item: {}", e);
                    message
                        .ack_with(async_nats::jetstream::AckKind::Nak(None))
                        .await
                        .ok();
                }
            }
        }

        Ok(None)
    }
}

// Example of how to use the queue in background workers
#[allow(dead_code)]
async fn example_url_embed_worker(js: JetstreamQueue) {
    let consumer = UrlEmbedConsumer::new(&js).await.unwrap();

    loop {
        if let Some((id, item)) = consumer.next().await.unwrap() {
            tracing::info!("Processing URL embed queue item {}: {}", id, item.url);
            // Process the URL embed...
        }
    }
}

#[allow(dead_code)]
async fn example_email_worker(js: JetstreamQueue) {
    let consumer = EmailConsumer::new(&js).await.unwrap();

    loop {
        if let Some((id, item)) = consumer.next().await.unwrap() {
            tracing::info!("Processing email queue item {}: to={}", id, item.to);
            // Send the email...
        }
    }
}
