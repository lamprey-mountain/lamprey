create table attachment (
    chat_id text not null,
    discord_id text not null,
    primary key (chat_id, discord_id)
);
