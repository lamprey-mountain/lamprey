create table message_attachment (
    message_id integer not null,
    lamprey_media_id text not null,
    discord_attachment_id text not null,
    foreign key (message_id) references message(id)
);
