create table metric_channel (
    ts timestamp not null,
    channel_id uuid not null references channel(id) on delete cascade,
    room_id uuid not null references room(id) on delete cascade,
    message_count bigint not null,
    media_count bigint not null,
    media_size bigint not null,
    primary key (ts, channel_id)
);

create table metric_room (
    ts timestamp not null,
    room_id uuid not null references room(id) on delete cascade,
    members bigint not null,
    members_join bigint not null,
    members_leave bigint not null,
    primary key (ts, room_id)
);

create table metric_invite (
    ts timestamp not null,
    room_id uuid not null references room(id) on delete cascade,
    origin_type text not null,
    origin_subject text not null,
    uses bigint not null,
    primary key (ts, room_id, origin_type, origin_subject)
);
