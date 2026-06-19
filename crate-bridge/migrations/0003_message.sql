create table message (
    id integer primary key autoincrement,
    portal_id integer not null,
    source_platform text not null,
    lamprey_message_id text,
    discord_message_id text,
    foreign key (portal_id) references portal(id)
);
