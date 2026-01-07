use std::default::Default;
use std::sync::Arc;

use axum::{
    body::Body,
    extract::{multipart, FromRequest, Path, Request, State},
    http::StatusCode,
    response::IntoResponse,
};
use common::v1::types::{
    self, media::MediaRef, EmbedCreate, MediaCreate, MediaCreateSource, MessageCreate, WebhookId,
};
use serde::{Deserialize, Serialize};
use url::Url;
use utoipa::ToSchema;
use validator::Validate;

use crate::{
    error::{Error, Result},
    ServerState,
};

#[derive(Debug, Serialize, Deserialize, Validate, ToSchema)]
struct DiscordEmbed {
    #[schema(min_length = 1, max_length = 256)]
    #[validate(length(min = 1, max = 256))]
    title: Option<String>,

    #[schema(min_length = 1, max_length = 4096)]
    #[validate(length(min = 1, max = 4096))]
    description: Option<String>,

    // TODO: validate length
    url: Option<Url>,
    color: Option<u32>,

    #[validate(nested)]
    author: Option<DiscordEmbedAuthor>,
}

#[derive(Debug, Serialize, Deserialize, Validate, ToSchema)]
struct DiscordEmbedAuthor {
    #[schema(min_length = 1, max_length = 256)]
    #[validate(length(min = 1, max = 256))]
    name: String,

    // TODO: validate length
    url: Option<Url>,
}

#[derive(Debug, Serialize, Deserialize, Validate, ToSchema)]
pub struct DiscordWebhookExecuteBody {
    #[validate(length(min = 1, max = 8192))]
    content: Option<String>,

    #[schema(required = false, min_length = 0, max_length = 32)]
    #[validate(length(min = 0, max = 32), nested)]
    embeds: Option<Vec<DiscordEmbed>>,
}

fn convert_embed(embed: DiscordEmbed) -> EmbedCreate {
    EmbedCreate {
        title: embed.title,
        description: embed.description,
        url: embed.url,
        color: embed.color.map(|c| format!("#{:06x}", c)),
        author_name: embed.author.as_ref().map(|a| a.name.clone()),
        author_url: embed.author.and_then(|a| a.url),
        media: None,
        thumbnail: None,
        author_avatar: None,
    }
}

/// Webhook execute discord (WIP)
#[utoipa::path(
    post,
    path = "/webhook/{webhook_id}/{token}/discord",
    params(
        ("webhook_id", description = "Webhook id"),
        ("token", description = "Webhook token")
    ),
    request_body(content_type = "multipart/form-data", content = String),
    tags = ["webhook"],
    responses(
        (status = NO_CONTENT, description = "Execute webhook success"),
    )
)]
pub async fn webhook_execute_discord(
    Path((webhook_id, token)): Path<(WebhookId, String)>,
    State(s): State<Arc<ServerState>>,
    req: Request<Body>,
) -> Result<impl IntoResponse> {
    let webhook = s.data().webhook_get_with_token(webhook_id, &token).await?;
    let webhook_user_id: types::UserId = (*webhook.id).into();
    let mut multipart = multipart::Multipart::from_request(req, &s).await?;

    let mut message_create = MessageCreate::default();
    let mut file_fields = Vec::new();

    while let Some(field) = multipart.next_field().await? {
        let name = field.name().unwrap_or("").to_string();
        if name == "payload_json" {
            let data = field.bytes().await?;
            let discord_payload: DiscordWebhookExecuteBody = serde_json::from_slice(&data)?;
            discord_payload.validate()?;

            message_create.content = discord_payload.content;
            if let Some(embeds) = discord_payload.embeds {
                message_create.embeds = embeds.into_iter().map(convert_embed).collect();
            }
        } else if name.starts_with("files[") {
            // TODO: more robust parsing
            let filename = field.file_name().map(|s| s.to_string());
            let data = field.bytes().await?;
            file_fields.push((filename, data));
        }
    }

    let mut attachments = Vec::new();
    if !file_fields.is_empty() {
        for (filename, data) in file_fields {
            let size = data.len() as u64;
            if size > s.config.media_max_size {
                return Err(Error::TooBig);
            }
            let media_create = MediaCreate {
                source: MediaCreateSource::Upload {
                    size,
                    filename: filename.unwrap_or_else(|| "file".to_string()),
                },
                alt: None,
            };

            let media = s
                .services()
                .media
                .import_from_bytes(webhook_user_id, media_create, data.into())
                .await?;

            attachments.push(MediaRef { id: media.id });
        }
    }
    message_create.attachments = attachments;

    if message_create.content.is_none()
        && message_create.attachments.is_empty()
        && message_create.embeds.is_empty()
    {
        return Err(Error::BadRequest(
            "at least one of content, attachments, or embeds must be defined".to_string(),
        ));
    }

    let srv = s.services();
    let _message = srv
        .messages
        .create(
            webhook.channel_id,
            webhook_user_id,
            None,
            None,
            message_create,
        )
        .await?;

    // TODO: return message if ?wait=true
    Ok(StatusCode::NO_CONTENT)
}
