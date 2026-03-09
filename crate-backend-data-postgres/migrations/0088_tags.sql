create table tag (
    id uuid primary key,
    version_id uuid not null,
    room_id uuid not null,
    name text not null,
    description text,
    color text,
    is_archived boolean not null,
    foreign key (room_id) references room(id)
);

create table tag_apply_thread (
    thread_id uuid,
    tag_id uuid,
    primary key (thread_id, tag_id),
    foreign key (thread_id) references thread(id),
    foreign key (tag_id) references tag(id)
);

create table tag_apply_room (
    room_id uuid,
    tag_id uuid,
    primary key (room_id, tag_id),
    foreign key (room_id) references room(id),
    foreign key (tag_id) references tag(id)
);
