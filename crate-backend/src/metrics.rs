use once_cell::sync::Lazy;
use prometheus::{register_int_gauge, IntGauge, Opts};

macro_rules! register_gauge {
    ($NAME:ident, $help:expr) => {
        pub static $NAME: Lazy<IntGauge> = Lazy::new(|| {
            register_int_gauge!(Opts::new(stringify!($NAME), $help).namespace("lamprey"))
                .unwrap_or_else(|e| panic!("Failed to register gauge {}: {}", stringify!($NAME), e))
        });
    };
}

register_gauge!(USER_COUNT_TOTAL, "Total number of users");
register_gauge!(USER_COUNT_GUEST, "Number of guest users");
register_gauge!(USER_COUNT_REGISTERED, "Number of registered users");
register_gauge!(USER_COUNT_BOT, "Number of bot users");
register_gauge!(USER_COUNT_WEBHOOK, "Number of webhook users");
register_gauge!(USER_COUNT_PUPPET, "Number of puppet users");
register_gauge!(USER_COUNT_PUPPET_BOT, "Number of puppet bot users");

register_gauge!(ROOM_COUNT_TOTAL, "Total number of rooms");
register_gauge!(ROOM_COUNT_PRIVATE, "Number of private rooms");
register_gauge!(ROOM_COUNT_PUBLIC, "Number of public rooms");

register_gauge!(CHANNEL_COUNT_TOTAL, "Total number of channels");
register_gauge!(CHANNEL_COUNT_TEXT, "Number of text channels");
register_gauge!(CHANNEL_COUNT_VOICE, "Number of voice channels");
register_gauge!(CHANNEL_COUNT_BROADCAST, "Number of broadcast channels");
register_gauge!(CHANNEL_COUNT_CALENDAR, "Number of calendar channels");
register_gauge!(
    CHANNEL_COUNT_THREAD_PUBLIC,
    "Number of public thread channels"
);
register_gauge!(
    CHANNEL_COUNT_THREAD_PRIVATE,
    "Number of private thread channels"
);
register_gauge!(CHANNEL_COUNT_DM, "Number of DM channels");
register_gauge!(CHANNEL_COUNT_GDM, "Number of GDM channels");
