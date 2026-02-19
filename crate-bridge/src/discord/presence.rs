use std::sync::Arc;

use anyhow::Result;
use serenity::all::{ActivityType, OnlineStatus, Presence};
use tracing::{info, trace};

use crate::bridge_common::Globals;
use crate::db::Data;

pub async fn process_presence_update(globals: Arc<Globals>, presence: Presence) -> Result<()> {
    let Some(puppet) = globals
        .get_puppet("discord", &presence.user.id.to_string())
        .await?
    else {
        trace!("no puppet found for discord user {}", presence.user.id);
        return Ok(());
    };

    let status = match presence.status {
        OnlineStatus::Online => common::v1::types::presence::Status::Online,
        OnlineStatus::Idle => common::v1::types::presence::Status::Away,
        OnlineStatus::DoNotDisturb => common::v1::types::presence::Status::Busy,
        OnlineStatus::Invisible | OnlineStatus::Offline => {
            common::v1::types::presence::Status::Offline
        }
        _ => common::v1::types::presence::Status::Online,
    };

    let activities = presence
        .activities
        .iter()
        .filter(|a| a.kind == ActivityType::Custom)
        .filter_map(|a| a.state.clone())
        .map(|text| common::v1::types::presence::Activity::Custom {
            text,
            clear_at: None,
        })
        .collect();

    let ly_presence = common::v1::types::presence::Presence { status, activities };

    let ly = globals.lamprey_handle().await?;
    ly.user_set_presence(puppet.id.into(), &ly_presence).await?;
    info!("updated lamprey presence for {}", presence.user.id);
    Ok(())
}
