create table message (
    chat_id text not null,
    discord_id text not null,
    chat_thread_id text not null,
    discord_channel_id text not null,
    primary key (chat_id, discord_id)
);
