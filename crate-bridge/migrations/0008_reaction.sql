create table reaction (
    portal_id text not null,
    source_platform text not null,
    lamprey_user_id text not null,
    discord_user_id text not null,
    lamprey_key text,
    discord_key text,
    foreign key (portal_id) references portal(id),
    foreign key (lamprey_user_id) references "user"(lamprey_id),
    foreign key (discord_user_id) references "user"(discord_id)
);
