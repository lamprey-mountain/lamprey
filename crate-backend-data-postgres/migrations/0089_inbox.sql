create table inbox (
    user_id uuid not null,
    read_at timestamp,
    added_at timestamp not null,
    room_id uuid,
    thread_id uuid,
    message_id uuid,
    reason text,
    foreign key (user_id) references usr(id),
    foreign key (room_id) references room(id),
    foreign key (thread_id) references thread(id)
);
