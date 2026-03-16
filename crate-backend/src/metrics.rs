use once_cell::sync::Lazy;
use prometheus::{register_int_gauge, IntGauge, Opts};

macro_rules! register_gauge {
    ($NAME:ident, $help:expr) => {
        register_gauge!($NAME, $help, stringify!($NAME));
    };
    ($NAME:ident, $help:expr, $metric_name:expr) => {
        pub static $NAME: Lazy<IntGauge> = Lazy::new(|| {
            register_int_gauge!(Opts::new($metric_name, $help).namespace("lamprey"))
                .unwrap_or_else(|e| panic!("Failed to register gauge {}: {}", stringify!($NAME), e))
        });
    };
}

register_gauge!(
    USER_COUNT_TOTAL,
    "Total number of users",
    "user_count_total"
);
register_gauge!(
    USER_COUNT_GUEST,
    "Number of guest users",
    "user_count_guest"
);
register_gauge!(
    USER_COUNT_REGISTERED,
    "Number of registered users",
    "user_count_registered"
);
register_gauge!(USER_COUNT_BOT, "Number of bot users", "user_count_bot");
register_gauge!(
    USER_COUNT_WEBHOOK,
    "Number of webhook users",
    "user_count_webhook"
);
register_gauge!(
    USER_COUNT_PUPPET,
    "Number of puppet users",
    "user_count_puppet"
);
register_gauge!(
    USER_COUNT_PUPPET_BOT,
    "Number of puppet bot users",
    "user_count_puppet_bot"
);

register_gauge!(
    ROOM_COUNT_TOTAL,
    "Total number of rooms",
    "room_count_total"
);
register_gauge!(
    ROOM_COUNT_PRIVATE,
    "Number of private rooms",
    "room_count_private"
);
register_gauge!(
    ROOM_COUNT_PUBLIC,
    "Number of public rooms",
    "room_count_public"
);

register_gauge!(
    CHANNEL_COUNT_TOTAL,
    "Total number of channels",
    "channel_count_total"
);
register_gauge!(
    CHANNEL_COUNT_TEXT,
    "Number of text channels",
    "channel_count_text"
);
register_gauge!(
    CHANNEL_COUNT_VOICE,
    "Number of voice channels",
    "channel_count_voice"
);
register_gauge!(
    CHANNEL_COUNT_BROADCAST,
    "Number of broadcast channels",
    "channel_count_broadcast"
);
register_gauge!(
    CHANNEL_COUNT_CALENDAR,
    "Number of calendar channels",
    "channel_count_calendar"
);
register_gauge!(
    CHANNEL_COUNT_THREAD_PUBLIC,
    "Number of public thread channels",
    "channel_count_thread_public"
);
register_gauge!(
    CHANNEL_COUNT_THREAD_PRIVATE,
    "Number of private thread channels",
    "channel_count_thread_private"
);
register_gauge!(
    CHANNEL_COUNT_DM,
    "Number of DM channels",
    "channel_count_dm"
);
register_gauge!(
    CHANNEL_COUNT_GDM,
    "Number of GDM channels",
    "channel_count_gdm"
);
