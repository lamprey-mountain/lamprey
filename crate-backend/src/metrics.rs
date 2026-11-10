use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;

use opentelemetry::{KeyValue, global};
use sqlx::PgPool;
use tracing::warn;

use crate::prelude::*;

/// register process metrics with the prometheus registry
pub fn register_process_metrics() {
    if let Some(registry) = kerosene_core::observability::prometheus_registry() {
        let collector = prometheus::process_collector::ProcessCollector::for_self();
        let _ = registry.register(Box::new(collector));
    }
}

/// register database pool metrics
pub fn register_db_pool_metrics(pool: PgPool) {
    let _ = global::meter("lamprey-backend")
        .u64_observable_gauge("db.pool.connections")
        .with_description("Database pool connections")
        .with_callback(move |observer| {
            let size = u64::from(pool.size());
            let idle = pool.num_idle() as u64;
            observer.observe(size, &[KeyValue::new("state", "total")]);
            observer.observe(idle, &[KeyValue::new("state", "idle")]);
            observer.observe(
                size.saturating_sub(idle),
                &[KeyValue::new("state", "in_use")],
            );
        })
        .build();
}

/// register sync connection metrics
pub fn register_connection_metrics(stats: Arc<kerosene_services::services::connections::Stats>) {
    let otel_stats = stats.clone();
    let _ = global::meter("lamprey-backend")
        .u64_observable_gauge("sync.connections.active")
        .with_description("Number of active sync connections")
        .with_callback(move |observer| {
            observer.observe(otel_stats.connections.load(Ordering::Relaxed) as u64, &[]);
        })
        .build();

    let otel_stats = stats;
    let _ = global::meter("lamprey-backend")
        .u64_observable_counter("sync.connections.total")
        .with_description("Total number of sync connections opened")
        .with_callback(move |observer| {
            observer.observe(
                otel_stats.connections_total.load(Ordering::Relaxed) as u64,
                &[],
            );
        })
        .build();
}

/// register database entity counts, refreshed periodically
pub fn register_instance_metrics(globals: Globals) {
    let meter = global::meter("lamprey-backend");
    let user_count_total = meter
        .u64_gauge("user.count.total")
        .with_description("Total number of users")
        .build();
    let user_count_guest = meter
        .u64_gauge("user.count.guest")
        .with_description("Number of guest users")
        .build();
    let user_count_registered = meter
        .u64_gauge("user.count.registered")
        .with_description("Number of registered users")
        .build();
    let user_count_bot = meter
        .u64_gauge("user.count.bot")
        .with_description("Number of bot users")
        .build();
    let user_count_webhook = meter
        .u64_gauge("user.count.webhook")
        .with_description("Number of webhook users")
        .build();
    let user_count_puppet = meter
        .u64_gauge("user.count.puppet")
        .with_description("Number of puppet users")
        .build();
    let user_count_puppet_bot = meter
        .u64_gauge("user.count.puppet_bot")
        .with_description("Number of puppet bot users")
        .build();

    let room_count_total = meter
        .u64_gauge("room.count.total")
        .with_description("Total number of rooms")
        .build();
    let room_count_private = meter
        .u64_gauge("room.count.private")
        .with_description("Number of private rooms")
        .build();
    let room_count_public = meter
        .u64_gauge("room.count.public")
        .with_description("Number of public rooms")
        .build();

    let channel_count_total = meter
        .u64_gauge("channel.count.total")
        .with_description("Total number of channels")
        .build();
    let channel_count_text = meter
        .u64_gauge("channel.count.text")
        .with_description("Number of text channels")
        .build();
    let channel_count_voice = meter
        .u64_gauge("channel.count.voice")
        .with_description("Number of voice channels")
        .build();
    let channel_count_broadcast = meter
        .u64_gauge("channel.count.broadcast")
        .with_description("Number of broadcast channels")
        .build();
    let channel_count_calendar = meter
        .u64_gauge("channel.count.calendar")
        .with_description("Number of calendar channels")
        .build();
    let channel_count_thread_public = meter
        .u64_gauge("channel.count.thread_public")
        .with_description("Number of public thread channels")
        .build();
    let channel_count_thread_private = meter
        .u64_gauge("channel.count.thread_private")
        .with_description("Number of private thread channels")
        .build();
    let channel_count_dm = meter
        .u64_gauge("channel.count.dm")
        .with_description("Number of DM channels")
        .build();
    let channel_count_gdm = meter
        .u64_gauge("channel.count.gdm")
        .with_description("Number of GDM channels")
        .build();

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(60));
        loop {
            interval.tick().await;

            let metrics = async {
                let mut data = globals.begin_read().await?;
                data.get_metrics().await
            }
            .await;

            let metrics = match metrics {
                Ok(metrics) => metrics,
                Err(err) => {
                    warn!("failed to collect instance metrics: {err}");
                    continue;
                }
            };

            user_count_total.record(metrics.user_count_total.max(0) as u64, &[]);
            user_count_guest.record(metrics.user_count_guest.max(0) as u64, &[]);
            user_count_registered.record(metrics.user_count_registered.max(0) as u64, &[]);
            user_count_bot.record(metrics.user_count_bot.max(0) as u64, &[]);
            user_count_webhook.record(metrics.user_count_webhook.max(0) as u64, &[]);
            user_count_puppet.record(metrics.user_count_puppet.max(0) as u64, &[]);
            user_count_puppet_bot.record(metrics.user_count_puppet_bot.max(0) as u64, &[]);

            room_count_total.record(metrics.room_count_total.max(0) as u64, &[]);
            room_count_private.record(metrics.room_count_private.max(0) as u64, &[]);
            room_count_public.record(metrics.room_count_public.max(0) as u64, &[]);

            channel_count_total.record(metrics.channel_count_total.max(0) as u64, &[]);
            channel_count_text.record(metrics.channel_count_text.max(0) as u64, &[]);
            channel_count_voice.record(metrics.channel_count_voice.max(0) as u64, &[]);
            channel_count_broadcast.record(metrics.channel_count_broadcast.max(0) as u64, &[]);
            channel_count_calendar.record(metrics.channel_count_calendar.max(0) as u64, &[]);
            channel_count_thread_public
                .record(metrics.channel_count_thread_public.max(0) as u64, &[]);
            channel_count_thread_private
                .record(metrics.channel_count_thread_private.max(0) as u64, &[]);
            channel_count_dm.record(metrics.channel_count_dm.max(0) as u64, &[]);
            channel_count_gdm.record(metrics.channel_count_gdm.max(0) as u64, &[]);
        }
    });
}
