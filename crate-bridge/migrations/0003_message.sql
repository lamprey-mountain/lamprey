create table message (
    id text primary key,
    portal_id text not null,
    source_platform text not null,
    lamprey_message_id text,
    discord_message_id text,
    foreign key (portal_id) references portal(id)
);
