use std::io::{self, Write};

use anyhow::{Context, Result};
use common::v1::types::{ChannelId, RoomId};
use lamprey_sdk::syncer::Syncer;
use serde_json::Value;

use crate::auth;
use crate::config::LyConfig;

/// tail the event stream, printing matching sync events until interrupted
pub async fn tail(
    config: &mut LyConfig,
    login: Option<&str>,
    channels: &[ChannelId],
    rooms: &[RoomId],
    events: &[String],
    json: bool,
    verbose: bool,
) -> Result<()> {
    auth::check(config, login).await?;
    let login = config.login(login)?.clone();

    if verbose {
        eprintln!("connecting to {}", login.api_url);
    }

    let mut handle = Syncer::builder()
        .sync_url(login.api_url)
        .token(login.token)
        .build()?;
    handle.connect();

    let mut stdout = io::stdout();
    let mut connected = false;
    loop {
        let sync = match handle.poll_sync().await {
            Ok(sync) => sync,
            // the client fell behind; skip the missed events and keep tailing
            Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
            Err(err) => return Err(err).context("event stream closed"),
        };

        if !connected {
            if verbose {
                eprintln!("connected, tailing events");
            }
            connected = true;
        }

        let event = serde_json::to_value(&*sync)?;
        if !matches(&event, channels, rooms, events) {
            continue;
        }

        let line = if json {
            serde_json::to_string(&event)?
        } else {
            serde_json::to_string_pretty(&event)?
        };
        writeln!(stdout, "{line}")?;
        stdout.flush()?;
    }
}

/// whether an event passes the channel, room and event type filters
fn matches(event: &Value, channels: &[ChannelId], rooms: &[RoomId], events: &[String]) -> bool {
    if !events.is_empty() {
        let matches_type = event
            .get("type")
            .and_then(Value::as_str)
            .is_some_and(|ty| events.iter().any(|filter| filter.eq_ignore_ascii_case(ty)));
        if !matches_type {
            return false;
        }
    }

    if channels.is_empty() && rooms.is_empty() {
        return true;
    }

    let mut found_channels = Vec::new();
    let mut found_rooms = Vec::new();
    collect_ids(event, &mut found_channels, &mut found_rooms);

    let channel_matches =
        channels.is_empty() || found_channels.iter().any(|id| channels.contains(id));
    let room_matches = rooms.is_empty() || found_rooms.iter().any(|id| rooms.contains(id));

    channel_matches && room_matches
}

/// collect the channel and room ids referenced anywhere in an event
fn collect_ids(event: &Value, channels: &mut Vec<ChannelId>, rooms: &mut Vec<RoomId>) {
    match event {
        Value::Object(map) => {
            for (key, value) in map {
                match key.as_str() {
                    "channel_id" | "thread_id" => {
                        if let Some(id) = value.as_str().and_then(|id| id.parse().ok()) {
                            channels.push(id);
                        }
                    }
                    "room_id" => {
                        if let Some(id) = value.as_str().and_then(|id| id.parse().ok()) {
                            rooms.push(id);
                        }
                    }
                    "channel" | "thread" => {
                        if let Some(id) = nested_id(value).and_then(|id| id.parse().ok()) {
                            channels.push(id);
                        }
                    }
                    "room" => {
                        if let Some(id) = nested_id(value).and_then(|id| id.parse().ok()) {
                            rooms.push(id);
                        }
                    }
                    _ => {}
                }

                collect_ids(value, channels, rooms);
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_ids(item, channels, rooms);
            }
        }
        _ => {}
    }
}

/// the `id` field of a nested object, if it has one
fn nested_id(value: &Value) -> Option<&str> {
    value.get("id").and_then(Value::as_str)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const CHANNEL: &str = "00000000-0000-0000-0000-000000000001";
    const OTHER_CHANNEL: &str = "00000000-0000-0000-0000-000000000002";
    const ROOM: &str = "00000000-0000-0000-0000-000000000003";
    const MESSAGE: &str = "00000000-0000-0000-0000-000000000004";

    fn channel(id: &str) -> ChannelId {
        id.parse().unwrap()
    }

    fn room(id: &str) -> RoomId {
        id.parse().unwrap()
    }

    #[test]
    fn filters_by_event_type() {
        let event = json!({ "type": "MessageCreate" });

        assert!(matches(&event, &[], &[], &["messagecreate".to_string()]));
        assert!(!matches(&event, &[], &[], &["MessageDelete".to_string()]));
    }

    #[test]
    fn filters_by_nested_channel_and_room() {
        let event = json!({
            "type": "ChannelCreate",
            "channel": { "id": CHANNEL, "room_id": ROOM },
        });

        assert!(matches(&event, &[channel(CHANNEL)], &[], &[]));
        assert!(matches(&event, &[], &[room(ROOM)], &[]));
        assert!(!matches(&event, &[channel(OTHER_CHANNEL)], &[], &[]));
    }

    #[test]
    fn filters_by_message_channel() {
        let event = json!({
            "type": "MessageCreate",
            "message": { "id": MESSAGE, "channel_id": CHANNEL },
        });

        assert!(matches(&event, &[channel(CHANNEL)], &[], &[]));
        assert!(!matches(&event, &[channel(OTHER_CHANNEL)], &[], &[]));
    }
}
