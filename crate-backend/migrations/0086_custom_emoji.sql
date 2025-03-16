create table custom_emoji (
    id uuid primary key,
    name text not null,
    creator_id uuid,
    animated boolean not null,
    media_id uuid not null,
    owner text not null check (owner in ('User', 'Room')),
    room_id uuid,
    foreign key (creator_id) references usr(id) on delete set null (creator_id),
    foreign key (media_id) references media(id) on delete cascade,
    foreign key (room_id) references room(id) on delete cascade
);
