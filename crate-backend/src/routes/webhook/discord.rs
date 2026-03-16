use std::default::Default;
use std::sync::Arc;

use axum::{
    body::to_bytes,
    extract::{FromRequest, Multipart, Path, Query, Request, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use bytes::Bytes;
use common::v2::types::media::{MediaCreate, MediaCreateSource};
use common::{
    v1::types::{
        self, EmbedCreate, MessageAttachmentCreate, MessageAttachmentCreateType, MessageCreate,
        WebhookId,
    },
    v2::types::media::MediaReference,
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

    #[validate(nested)]
    footer: Option<DiscordEmbedFooter>,

    #[validate(nested)]
    image: Option<DiscordEmbedImage>,

    #[validate(nested)]
    thumbnail: Option<DiscordEmbedThumbnail>,

    // TODO: validate length
    timestamp: Option<String>,

    #[validate(length(max = 25))]
    fields: Option<Vec<DiscordEmbedField>>,
}

#[derive(Debug, Serialize, Deserialize, Validate, ToSchema)]
struct DiscordEmbedAuthor {
    #[schema(min_length = 1, max_length = 256)]
    #[validate(length(min = 1, max = 256))]
    name: String,

    // TODO: validate length
    url: Option<Url>,

    // TODO: validate length
    icon_url: Option<Url>,
}

#[derive(Debug, Serialize, Deserialize, Validate, ToSchema)]
struct DiscordEmbedFooter {
    #[schema(min_length = 1, max_length = 2048)]
    #[validate(length(min = 1, max = 2048))]
    text: String,

    // TODO: validate length
    icon_url: Option<Url>,
}

#[derive(Debug, Serialize, Deserialize, Validate, ToSchema)]
struct DiscordEmbedImage {
    // TODO: validate length
    url: Option<Url>,
}

#[derive(Debug, Serialize, Deserialize, Validate, ToSchema)]
struct DiscordEmbedThumbnail {
    // TODO: validate length
    url: Option<Url>,
}

#[derive(Debug, Serialize, Deserialize, Validate, ToSchema)]
struct DiscordEmbedField {
    #[schema(min_length = 1, max_length = 256)]
    #[validate(length(min = 1, max = 256))]
    name: String,

    #[schema(min_length = 1, max_length = 1024)]
    #[validate(length(min = 1, max = 1024))]
    value: String,

    #[serde(default)]
    inline: Option<bool>,
}

#[derive(Debug, Default, Serialize, Deserialize, Validate, ToSchema)]
pub struct DiscordAllowedMentions {
    #[serde(default)]
    parse: Option<Vec<String>>,

    #[serde(default)]
    users: Option<Vec<String>>,

    #[serde(default)]
    roles: Option<Vec<String>>,

    #[serde(default)]
    replied_user: Option<bool>,
}

impl DiscordAllowedMentions {
    fn into_parse_mentions(self) -> Result<self::types::ParseMentions> {
        let parse = self.parse.as_deref();

        // If parse contains "none", disable all mentions
        let has_none = parse.map_or(false, |p| p.iter().any(|s| s == "none"));
        let has_users = parse.map_or(false, |p| p.iter().any(|s| s == "users"));
        let has_roles = parse.map_or(false, |p| p.iter().any(|s| s == "roles"));
        let has_everyone = parse.map_or(false, |p| p.iter().any(|s| s == "everyone"));

        let users = if has_none {
            Some(vec![])
        } else if has_users || self.users.is_some() {
            let user_ids = self
                .users
                .unwrap_or_default()
                .into_iter()
                .filter_map(|id| id.parse::<uuid::Uuid>().ok())
                .map(types::UserId::from)
                .collect();
            Some(user_ids)
        } else {
            None
        };

        let roles = if has_none {
            Some(vec![])
        } else if has_roles || self.roles.is_some() {
            let role_ids = self
                .roles
                .unwrap_or_default()
                .into_iter()
                .filter_map(|id| id.parse::<uuid::Uuid>().ok())
                .map(types::RoleId::from)
                .collect();
            Some(role_ids)
        } else {
            None
        };

        let everyone = if has_none { false } else { has_everyone };

        Ok(self::types::ParseMentions {
            users,
            roles,
            everyone,
        })
    }
}

#[derive(Debug, Default, Serialize, Deserialize, ToSchema)]
pub struct DiscordPartialAttachment {
    id: Option<String>,
    filename: Option<String>,
    description: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize, Validate, ToSchema)]
pub struct DiscordWebhookExecuteBody {
    #[validate(length(max = 2000))]
    content: Option<String>,

    #[schema(required = false, max_length = 10)]
    #[validate(length(max = 10), nested)]
    embeds: Option<Vec<DiscordEmbed>>,

    #[validate(nested)]
    allowed_mentions: Option<DiscordAllowedMentions>,

    attachments: Option<Vec<DiscordPartialAttachment>>,
}

struct EmbedMediaUrls {
    image_url: Option<Url>,
    thumbnail_url: Option<Url>,
    author_avatar_url: Option<Url>,
}

fn extract_embed_media(embed: &DiscordEmbed) -> EmbedMediaUrls {
    EmbedMediaUrls {
        image_url: embed.image.as_ref().and_then(|i| i.url.clone()),
        thumbnail_url: embed.thumbnail.as_ref().and_then(|t| t.url.clone()),
        author_avatar_url: embed.author.as_ref().and_then(|a| a.icon_url.clone()),
    }
}

fn convert_embed_with_media(
    embed: DiscordEmbed,
    image_media: Option<MediaReference>,
    thumbnail_media: Option<MediaReference>,
    author_avatar_media: Option<MediaReference>,
) -> EmbedCreate {
    EmbedCreate {
        title: embed.title,
        description: embed.description,
        url: embed.url,
        color: embed.color.map(|c| format!("#{:06x}", c)),
        author_name: embed.author.as_ref().map(|a| a.name.clone()),
        author_url: embed.author.and_then(|a| a.url),
        media: image_media,
        thumbnail: thumbnail_media,
        author_avatar: author_avatar_media,
    }
}

struct ParsedAttachment {
    filename: String,
    data: Bytes,
    description: Option<String>,
}

struct ParsedEmbed {
    discord_embed: DiscordEmbed,
    media_urls: EmbedMediaUrls,
}

struct Parsed {
    message: MessageCreate,
    attachments: Vec<ParsedAttachment>,
    embeds: Vec<ParsedEmbed>,
}

#[derive(Debug, Default, Deserialize)]
pub struct WebhookQuery {
    #[serde(default)]
    wait: Option<bool>,
}

async fn parse_webhook_body(req: Request, s: &Arc<ServerState>) -> Result<Parsed> {
    let ct = req
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let mut message_create = MessageCreate::default();
    let mut file_attachments: Vec<(String, Bytes)> = Vec::new();
    let mut attachment_metadata: Vec<DiscordPartialAttachment> = Vec::new();
    let mut parsed_embeds: Vec<ParsedEmbed> = Vec::new();

    if ct.contains("application/json") {
        let body = req.into_body();
        let bytes = to_bytes(body, usize::MAX).await?;
        let payload: DiscordWebhookExecuteBody = serde_json::from_slice(&bytes)?;
        payload.validate()?;
        message_create.content = payload.content;
        if let Some(embeds) = payload.embeds {
            parsed_embeds = embeds
                .into_iter()
                .map(|e| {
                    let media_urls = extract_embed_media(&e);
                    ParsedEmbed {
                        discord_embed: e,
                        media_urls,
                    }
                })
                .collect();
        }
        if let Some(allowed_mentions) = payload.allowed_mentions {
            message_create.mentions = allowed_mentions.into_parse_mentions()?;
        }
        if let Some(attachments) = payload.attachments {
            attachment_metadata = attachments;
        }
    } else if ct.contains("multipart/form-data") {
        let multipart = Multipart::from_request(req, s).await?;
        let mut multipart = multipart;
        while let Some(field) = multipart.next_field().await? {
            let name = field.name().unwrap_or("").to_string();
            if name == "payload_json" {
                let data = field.bytes().await?;
                let payload: DiscordWebhookExecuteBody = serde_json::from_slice(&data)?;
                payload.validate()?;
                message_create.content = payload.content;
                if let Some(embeds) = payload.embeds {
                    parsed_embeds = embeds
                        .into_iter()
                        .map(|e| {
                            let media_urls = extract_embed_media(&e);
                            ParsedEmbed {
                                discord_embed: e,
                                media_urls,
                            }
                        })
                        .collect();
                }
                if let Some(allowed_mentions) = payload.allowed_mentions {
                    message_create.mentions = allowed_mentions.into_parse_mentions()?;
                }
                if let Some(attachments) = payload.attachments {
                    attachment_metadata = attachments;
                }
            } else if name.starts_with("files[") {
                // Extract index from files[0], files[1], etc.
                let id = name
                    .strip_prefix("files[")
                    .and_then(|s| s.strip_suffix(']'))
                    .unwrap_or("0")
                    .to_string();
                let data = field.bytes().await?;
                file_attachments.push((id, data));
            }
        }
    } else {
        return Err(Error::BadStatic(
            "content-type must be multipart/form-data or application/json",
        ));
    }

    // Correlate uploaded files with their metadata
    let mut attachments = Vec::new();
    for (id, data) in file_attachments {
        let metadata = attachment_metadata
            .iter()
            .find(|m| m.id.as_deref() == Some(&id));

        let filename = metadata
            .and_then(|m| m.filename.clone())
            .unwrap_or_else(|| {
                // Use original filename from multipart field
                id
            });

        let description = metadata.and_then(|m| m.description.clone());

        attachments.push(ParsedAttachment {
            filename,
            data,
            description,
        });
    }

    Ok(Parsed {
        message: message_create,
        attachments,
        embeds: parsed_embeds,
    })
}

/// Webhook execute discord
#[utoipa::path(
    post,
    path = "/webhook/{webhook_id}/{token}/discord",
    params(
        ("webhook_id", description = "Webhook id"),
        ("token", description = "Webhook token"),
        ("wait", description = "Wait for message to be sent and return it")
    ),
    request_body(content_type = "multipart/form-data", content = String),
    tags = ["webhook"],
    responses(
        (status = NO_CONTENT, description = "Execute webhook success"),
        (status = OK, description = "Execute webhook success with message")
    )
)]
pub async fn webhook_execute_discord(
    Path((webhook_id, token)): Path<(WebhookId, String)>,
    State(s): State<Arc<ServerState>>,
    Query(query): Query<WebhookQuery>,
    req: Request,
) -> Result<impl IntoResponse> {
    let webhook = s.data().webhook_get_with_token(webhook_id, &token).await?;
    let webhook_user_id: types::UserId = (*webhook.id).into();

    let Parsed {
        message: mut message_create,
        attachments: file_fields,
        embeds: parsed_embeds,
    } = parse_webhook_body(req, &s).await?;

    // Process file attachments
    let mut attachments = Vec::new();
    for attachment in file_fields {
        let size = attachment.data.len() as u64;
        if size > s.config.media_max_size {
            return Err(Error::TooBig);
        }
        let media_create = MediaCreate {
            strip_exif: false,
            source: MediaCreateSource::Upload {
                size: Some(size),
                filename: attachment.filename.clone(),
            },
            alt: attachment.description.clone(),
        };

        let media = s
            .services()
            .media
            .import_from_bytes(webhook_user_id, media_create, attachment.data.into())
            .await?;

        attachments.push(MessageAttachmentCreate {
            ty: MessageAttachmentCreateType::Media {
                media: MediaReference::Media { media_id: media.id },
                alt: Some(attachment.description),
                filename: Some(attachment.filename),
            },
            spoiler: false,
        });
    }
    message_create.attachments = attachments;

    // Process embed media URLs
    let mut embeds = Vec::new();
    for parsed_embed in parsed_embeds {
        let mut image_media = None;
        let mut thumbnail_media = None;
        let mut author_avatar_media = None;

        // Fetch image media
        if let Some(url) = parsed_embed.media_urls.image_url {
            let media_create = MediaCreate {
                strip_exif: false,
                source: MediaCreateSource::Upload {
                    size: None,
                    filename: url
                        .path_segments()
                        .and_then(|s| s.last())
                        .unwrap_or("image")
                        .to_string(),
                },
                alt: None,
            };
            if let Ok(media) = s
                .services()
                .media
                .import_from_url(webhook_user_id, media_create)
                .await
            {
                image_media = Some(MediaReference::Media { media_id: media.id });
            }
        }

        // Fetch thumbnail media
        if let Some(url) = parsed_embed.media_urls.thumbnail_url {
            let media_create = MediaCreate {
                strip_exif: false,
                source: MediaCreateSource::Upload {
                    size: None,
                    filename: url
                        .path_segments()
                        .and_then(|s| s.last())
                        .unwrap_or("thumbnail")
                        .to_string(),
                },
                alt: None,
            };
            if let Ok(media) = s
                .services()
                .media
                .import_from_url(webhook_user_id, media_create)
                .await
            {
                thumbnail_media = Some(MediaReference::Media { media_id: media.id });
            }
        }

        // Fetch author avatar media
        if let Some(url) = parsed_embed.media_urls.author_avatar_url {
            let media_create = MediaCreate {
                strip_exif: false,
                source: MediaCreateSource::Upload {
                    size: None,
                    filename: url
                        .path_segments()
                        .and_then(|s| s.last())
                        .unwrap_or("avatar")
                        .to_string(),
                },
                alt: None,
            };
            if let Ok(media) = s
                .services()
                .media
                .import_from_url(webhook_user_id, media_create)
                .await
            {
                author_avatar_media = Some(MediaReference::Media { media_id: media.id });
            }
        }

        let embed_create = convert_embed_with_media(
            parsed_embed.discord_embed,
            image_media,
            thumbnail_media,
            author_avatar_media,
        );
        embeds.push(embed_create);
    }
    message_create.embeds = embeds;

    if message_create.is_empty() {
        return Err(Error::BadRequest(
            "at least one of content, attachments, or embeds must be defined".to_string(),
        ));
    }

    let srv = s.services();
    let message = srv
        .messages
        .create_system(webhook.channel_id, webhook_user_id, None, message_create)
        .await?;

    if query.wait.unwrap_or(false) {
        Ok(Json(message).into_response())
    } else {
        Ok(StatusCode::NO_CONTENT.into_response())
    }
}
