use std::fmt;

use anyhow::{Result, bail};
use common::v1::types::{Channel, ChannelCreate, ChannelId, ChannelPatch, ChannelType, RoomId};
use serde::Serialize;

use crate::auth;
use crate::config::LyConfig;

/// result of `ly channel create`, `ly channel get` and `ly channel update`
#[derive(Debug, Serialize)]
#[serde(transparent)]
pub struct ChannelResult(pub Channel);

/// result of `ly channel remove`
#[derive(Debug, Serialize)]
pub struct RemoveResult {
    pub channel_id: ChannelId,
}

impl fmt::Display for ChannelResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let channel = &self.0;
        let name = channel
            .name
            .as_deref()
            .or(channel.auto_name.as_deref())
            .unwrap_or("<unnamed>");

        write!(f, "{} ({:?}): {name}", channel.id, channel.ty)
    }
}

impl fmt::Display for RemoveResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "removed channel {}", self.channel_id)
    }
}

/// create a channel in a room
pub async fn create(
    config: &mut LyConfig,
    login: Option<&str>,
    room_id: RoomId,
    name: String,
    kind: ChannelType,
) -> Result<ChannelResult> {
    let http = auth::client(config, login).await?;
    let channel = http
        .channel_create_room(
            room_id,
            &ChannelCreate {
                name: Some(name),
                ty: kind,
                ..Default::default()
            },
        )
        .await?;

    Ok(ChannelResult(channel))
}

/// get channel details
pub async fn get(
    config: &mut LyConfig,
    login: Option<&str>,
    channel_id: ChannelId,
) -> Result<ChannelResult> {
    let http = auth::client(config, login).await?;
    let channel = http.channel_get(channel_id).await?;

    Ok(ChannelResult(channel))
}

/// update channel properties
pub async fn update(
    config: &mut LyConfig,
    login: Option<&str>,
    channel_id: ChannelId,
    name: Option<String>,
    description: Option<String>,
) -> Result<ChannelResult> {
    if name.is_none() && description.is_none() {
        bail!("no channel properties to update");
    }

    let http = auth::client(config, login).await?;
    let channel = http
        .channel_update(
            channel_id,
            &ChannelPatch {
                name: name.map(Some),
                description: description.map(Some),
                ..Default::default()
            },
        )
        .await?;

    Ok(ChannelResult(channel))
}

/// remove a channel
pub async fn remove(
    config: &mut LyConfig,
    login: Option<&str>,
    channel_id: ChannelId,
) -> Result<RemoveResult> {
    let http = auth::client(config, login).await?;
    http.channel_remove(channel_id).await?;

    Ok(RemoveResult { channel_id })
}
