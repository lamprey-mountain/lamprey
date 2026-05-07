use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::time::Duration;

use common::v2::types::media::MediaReference;
use dashmap::mapref::one::RefMut;
use futures::stream::FuturesUnordered;
use futures::StreamExt;

use crate::services::messages::util::MediaRegistry;
use crate::{routes::util::Auth, services::messages::ServiceMessages, Error, Result};

use common::v1::types::components::{self, Components, Thin};
use common::v1::types::error::{ApiError, ErrorCode};
use common::v1::types::flume::FlumeDelta;
use common::v1::types::message::flume::{FlumeCreate, FlumeState, MessageFlume};
use common::v1::types::message::{MessageDefaultMarkdown, MessageType};
use common::v1::types::sync::MessageSync;
use common::v1::types::util::Time;
use common::v1::types::{ChannelId, MediaId, Mentions, Message, MessageId};

use http::StatusCode;
use time::PrimitiveDateTime;
use tokio::task::JoinHandle;
use tracing::{debug, error};
use validator::Validate;

use crate::types::DbMessageCreate;

/// automatically commit a flume if no update is received for this duration
pub const FLUME_AUTOCOMMIT: Duration = Duration::from_secs(30);

pub struct Flume {
    pub channel_id: ChannelId,
    pub content: FlumeContent,
    expire_handle: JoinHandle<Result<()>>,
}

#[derive(Debug)]
pub struct FlumeContent {
    components: Components<Thin>,
}

impl FlumeContent {
    /// apply a delta, resolving media references
    pub fn apply(
        &mut self,
        delta: FlumeDelta,
        resolve_media: impl Fn(MediaReference) -> std::result::Result<MediaId, ApiError>,
    ) -> std::result::Result<(), ApiError> {
        self.components.apply_delta(delta, resolve_media)
    }
}

impl ServiceMessages {
    /// create a new flume
    ///
    /// flumes allow updating messages in real time
    // TODO: handle nonce
    // TODO: handle header_timestamp
    pub async fn flume_create(
        &self,
        channel_id: ChannelId,
        auth: &Auth,
        _nonce: Option<String>,
        json: FlumeCreate,
        header_timestamp: Option<Time>,
    ) -> Result<(StatusCode, Message)> {
        let srv = self.state.services();
        let channel = srv.channels.get(channel_id, Some(auth.user.id)).await?;
        channel.ensure_has_text()?;

        // permission checks (reuses validate_session_permissions from create.rs)
        let (_permissions, _created_at) = self
            .validate_session_permissions(
                auth,
                &channel,
                false, // flumes don't support attachments
                false, // flumes don't support embeds
                header_timestamp,
            )
            .await?;

        json.validate()?;
        if json.components.is_empty() {
            return Err(ApiError::with_message(
                ErrorCode::InvalidData,
                "at least one component must be defined".to_owned(),
            )
            .into());
        }

        // 2. prepare components and collect media IDs (reuses process_components_with_media from create.rs)
        let mut all_media_ids = MediaRegistry::default();
        let components_thin = self
            .process_components_with_media(&json.components, None, &mut all_media_ids)
            .await?;

        // 3. commit
        let message_id = MessageId::new();
        let version_id = (*message_id).into();
        let user_id = auth.user.id;

        let payload = MessageType::DefaultMarkdown(MessageDefaultMarkdown {
            content: None,
            metadata: json.metadata.clone(),
            reply_id: json.reply_id,
            attachments: vec![],
            embeds: vec![],
            components: Components::default(), // components stored in flume, not in version
        });

        let flume_json = serde_json::to_value(MessageFlume {
            state: FlumeState::Live,
        })?;

        let created_at = header_timestamp
            .map(|t| PrimitiveDateTime::new(t.date(), t.time()))
            .unwrap_or_else(|| {
                let now = time::OffsetDateTime::now_utc();
                PrimitiveDateTime::new(now.date(), now.time())
            });

        let mut data = self.state.data();
        let components_inner = components_thin.inner.clone();
        data.message_create(DbMessageCreate {
            id: Some(message_id),
            channel_id,
            attachment_ids: vec![],
            author_id: user_id,
            embeds: vec![],
            components: components_inner,
            message_type: payload,
            created_at: Some(created_at),
            removed_at: None,
            mentions: Mentions::default(),
            flume: Some(flume_json),
        })
        .await?;

        // 4. validate media ownership
        self.validate_media(&all_media_ids, message_id, user_id)
            .await?;
        // 5. insert media links
        self.claim_media(&mut all_media_ids, message_id, version_id)
            .await?;

        let message = self.get(channel_id, message_id, None).await?;

        let flume_content = FlumeContent {
            components: components_thin,
        };

        let expire_handle = self.spawn_autocommit_timer(channel_id, message_id);

        self.flumes.insert(
            message_id,
            Flume {
                channel_id,
                content: flume_content,
                expire_handle,
            },
        );

        self.state.broadcast(MessageSync::MessageCreate {
            message: message.clone(),
        })?;

        debug!(message_id = %message_id, "flume created");
        Ok((StatusCode::CREATED, message))
    }

    /// apply an update to the flume
    pub async fn flume_update(
        &self,
        channel_id: ChannelId,
        message_id: MessageId,
        auth: &Auth,
        delta: FlumeDelta,
    ) -> Result<StatusCode> {
        let mut flume_ref = self.flume_lookup(channel_id, message_id).await?;

        // check author: only author can update flume
        let message = self.get(channel_id, message_id, None).await?;
        if message.author_id != auth.user.id {
            return Err(ApiError::from_code(ErrorCode::OnlyMessageAuthorCanManageFlume).into());
        }

        // 1. abort old timer
        flume_ref.expire_handle.abort();

        // 2. apply delta and collect media
        let all_media_ids = RefCell::new(MediaRegistry::default());

        let resolve_media = |mr: MediaReference| {
            let media_id = match mr {
                MediaReference::Media { media_id } => media_id,
                _ => return Err(ApiError::from_code(ErrorCode::Unimplemented)),
            };
            all_media_ids.borrow_mut().insert(media_id);
            Ok(media_id)
        };

        flume_ref.content.apply(delta.clone(), resolve_media)?;

        let mut all_media_ids = all_media_ids.into_inner();

        // 3. validate media ownership if there are new media IDs
        if !all_media_ids.known.is_empty() {
            let version_id = (*message_id).into();
            self.validate_media(&all_media_ids, message_id, auth.user.id)
                .await?;
            self.claim_media(&mut all_media_ids, message_id, version_id)
                .await?;
        }

        // 4. restart timer
        flume_ref.expire_handle = self.spawn_autocommit_timer(channel_id, message_id);

        // 5. broadcast delta
        self.state.broadcast(MessageSync::FlumeDelta {
            channel_id,
            message_id,
            delta,
        })?;

        Ok(StatusCode::NO_CONTENT)
    }

    /// keep the flume alive
    pub async fn flume_ping(
        &self,
        channel_id: ChannelId,
        message_id: MessageId,
        auth: &Auth,
    ) -> Result<()> {
        let mut flume_ref = self.flume_lookup(channel_id, message_id).await?;

        // check author
        let message = self.get(channel_id, message_id, None).await?;
        if message.author_id != auth.user.id {
            return Err(ApiError::from_code(ErrorCode::OnlyMessageAuthorCanManageFlume).into());
        }

        // reset timer
        flume_ref.expire_handle.abort();
        flume_ref.expire_handle = self.spawn_autocommit_timer(channel_id, message_id);

        Ok(())
    }

    /// commit the flume
    ///
    /// creates a new message version with everything written so far and no longer allows live updating.
    pub async fn flume_commit(
        &self,
        channel_id: ChannelId,
        message_id: MessageId,
    ) -> Result<Message> {
        // use flume_lookup to validate
        let _ = self.flume_lookup(channel_id, message_id).await?;

        let Some((_, flume)) = self.flumes.remove(&message_id) else {
            return Err(Error::Internal(
                "flume disappeared while committing?".to_string(),
            ));
        };

        self.flume_commit_inner(channel_id, message_id, flume, FlumeState::Committed)
            .await
    }

    /// internal commit logic shared by flume_commit and the expiration handler
    async fn flume_commit_inner(
        &self,
        channel_id: ChannelId,
        message_id: MessageId,
        flume: Flume,
        new_state: FlumeState,
    ) -> Result<Message> {
        if flume.content.components.is_empty() {
            return self.get(channel_id, message_id, None).await;
        }

        let mut data = self.state.data();

        // 1. get the message to get author_id and version_id
        let message = self.get(channel_id, message_id, None).await?;
        let author_id = message.author_id;
        let version_id = message.latest_version.version_id;

        // 2. update message.flume state
        let flume_json = serde_json::to_value(MessageFlume { state: new_state })?;
        data.message_flume_update(message_id, flume_json).await?;

        // 3. update version in place with accumulated components
        let content = match &message.latest_version.message_type {
            MessageType::DefaultMarkdown(m) => m,
            _ => {
                return Err(Error::Internal(
                    "somehow message became not DefaultMarkdown?".to_string(),
                ))
            }
        };

        let payload = MessageType::DefaultMarkdown(MessageDefaultMarkdown {
            content: None,
            metadata: content.metadata.clone(),
            reply_id: content.reply_id,
            attachments: vec![],
            embeds: vec![],
            components: Components::default(),
        });

        data.message_update_in_place(
            channel_id,
            version_id,
            crate::types::DbMessageUpdate {
                attachment_ids: vec![],
                author_id,
                embeds: vec![],
                components: flume.content.components.inner,
                message_type: payload,
                created_at: Some(Time::now_utc().into()),
                mentions: Mentions::default(),
            },
        )
        .await?;

        // 4. get updated message
        let message = self.get(channel_id, message_id, None).await?;

        // 5. broadcast
        self.state.broadcast(MessageSync::MessageUpdate {
            message: message.clone(),
        })?;

        Ok(message)
    }

    /// attempt to lookup a flume, validating and returning errors if needed
    async fn flume_lookup(
        &self,
        channel_id: ChannelId,
        message_id: MessageId,
    ) -> Result<RefMut<'_, MessageId, Flume>> {
        let message = self.get(channel_id, message_id, None).await?;

        if message.flume.is_none() {
            return Err(ApiError::from_code(ErrorCode::MessageDoesntHaveFlume).into());
        };

        self.flumes
            .get_mut(&message_id)
            .ok_or_else(|| ApiError::from_code(ErrorCode::FlumeCommitted))
            .map_err(Error::from)
    }

    fn spawn_autocommit_timer(
        &self,
        channel_id: ChannelId,
        message_id: MessageId,
    ) -> JoinHandle<Result<()>> {
        let services = self.state.services();
        tokio::spawn(async move {
            tokio::time::sleep(FLUME_AUTOCOMMIT).await;

            let had = services.messages.flumes.remove(&message_id);
            debug!(
                "expire flume for {message_id}, had {:?}",
                had.as_ref().map(|(_, f)| &f.content)
            );

            if let Some((_, flume)) = had {
                let _ = services
                    .messages
                    .flume_commit_inner(channel_id, message_id, flume, FlumeState::Autocommitted)
                    .await;
            }

            Result::Ok(())
        })
    }

    /// resolves components' media and makes them canonical
    async fn resolve_media_and_make_canonical(
        &self,
        components: &Components<Thin>,
    ) -> Result<Components<components::Canonical>> {
        let mut media_ids = Vec::new();
        components.collect_media_refs(&mut media_ids);

        // deduplicate media ids
        let media_ids: Vec<_> = media_ids
            .into_iter()
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();

        let mut media_cache = HashMap::new();
        let mut media_futs = FuturesUnordered::new();
        for media_id in &media_ids {
            media_futs.push(async { (*media_id, self.state.data().media_select(*media_id).await) });
        }
        while let Some((media_id, result)) = media_futs.next().await {
            if let Ok(media) = result {
                media_cache.insert(media_id, media);
            }
        }

        let canonical = components.clone().into_canonical(|media_id: MediaId| {
            media_cache.get(&media_id).cloned().ok_or_else(|| {
                error!(media_id = ?media_id, "media not found in cache");
                Error::BadStatic("media not found in cache")
            })
        })?;

        Ok(canonical)
    }

    /// get initial delta for sync
    pub async fn flume_initial(&self, flume: &Flume) -> Result<FlumeDelta> {
        let components_canonical = self
            .resolve_media_and_make_canonical(&flume.content.components)
            .await?;

        Ok(FlumeDelta {
            init: Some(components_canonical),
            append: vec![],
            replace: vec![],
            delete: vec![],
        })
    }
}
