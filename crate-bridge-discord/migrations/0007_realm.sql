create table realm (
    lamprey_room_id uuid not null primary key,
    discord_guild_id uuid not null,
    continuous boolean
);
