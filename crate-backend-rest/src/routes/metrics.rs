use std::sync::Arc;

use axum::{extract::State, response::IntoResponse};
use prometheus::{Encoder, TextEncoder};

use crate::{
    metrics::{
        CHANNEL_COUNT_BROADCAST, CHANNEL_COUNT_CALENDAR, CHANNEL_COUNT_DM, CHANNEL_COUNT_GDM,
        CHANNEL_COUNT_TEXT, CHANNEL_COUNT_THREAD_PRIVATE, CHANNEL_COUNT_THREAD_PUBLIC,
        CHANNEL_COUNT_TOTAL, CHANNEL_COUNT_VOICE, ROOM_COUNT_PRIVATE, ROOM_COUNT_PUBLIC,
        ROOM_COUNT_TOTAL, USER_COUNT_BOT, USER_COUNT_GUEST, USER_COUNT_PUPPET,
        USER_COUNT_PUPPET_BOT, USER_COUNT_REGISTERED, USER_COUNT_TOTAL, USER_COUNT_WEBHOOK,
    },
    routes::util::Auth,
    types::{Permission, SERVER_ROOM_ID},
    Error, Result, ServerState,
};

pub async fn get_metrics(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let perms = s
        .services()
        .perms
        .for_room(auth.user.id, SERVER_ROOM_ID)
        .await?;
    perms.ensure(Permission::ServerMetrics)?;

    let metrics = s.data().get_metrics().await?;

    USER_COUNT_TOTAL.set(metrics.user_count_total);
    USER_COUNT_GUEST.set(metrics.user_count_guest);
    USER_COUNT_REGISTERED.set(metrics.user_count_registered);
    USER_COUNT_BOT.set(metrics.user_count_bot);
    USER_COUNT_WEBHOOK.set(metrics.user_count_webhook);
    USER_COUNT_PUPPET.set(metrics.user_count_puppet);
    USER_COUNT_PUPPET_BOT.set(metrics.user_count_puppet_bot);

    ROOM_COUNT_TOTAL.set(metrics.room_count_total);
    ROOM_COUNT_PRIVATE.set(metrics.room_count_private);
    ROOM_COUNT_PUBLIC.set(metrics.room_count_public);

    CHANNEL_COUNT_TOTAL.set(metrics.channel_count_total);
    CHANNEL_COUNT_TEXT.set(metrics.channel_count_text);
    CHANNEL_COUNT_VOICE.set(metrics.channel_count_voice);
    CHANNEL_COUNT_BROADCAST.set(metrics.channel_count_broadcast);
    CHANNEL_COUNT_CALENDAR.set(metrics.channel_count_calendar);
    CHANNEL_COUNT_THREAD_PUBLIC.set(metrics.channel_count_thread_public);
    CHANNEL_COUNT_THREAD_PRIVATE.set(metrics.channel_count_thread_private);
    CHANNEL_COUNT_DM.set(metrics.channel_count_dm);
    CHANNEL_COUNT_GDM.set(metrics.channel_count_gdm);

    let encoder = TextEncoder::new();
    let mut buffer = vec![];
    encoder
        .encode(&prometheus::gather(), &mut buffer)
        .map_err(|_| Error::BadStatic("failed to encode metrics"))?;

    Ok(buffer)
}
