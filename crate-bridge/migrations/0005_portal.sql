create table portal (
    lamprey_thread_id text primary key not null,
    lamprey_room_id text not null,
    discord_guild_id text not null,
    discord_channel_id text not null,
    discord_thread_id text,
    discord_webhook text not null
);