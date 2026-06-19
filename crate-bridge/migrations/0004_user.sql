create table "user" (
    source_platform text not null,
    lamprey_id text not null,
    discord_id text not null,
    discord_avatar_url text,
    discord_banner_url text
);

create unique index user_lamprey_id on "user" (lamprey_id);
create unique index user_discord_id on "user" (discord_id);
