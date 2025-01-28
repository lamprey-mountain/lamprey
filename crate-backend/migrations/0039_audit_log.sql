create table audit_log (
    id uuid primary key,
    room_id uuid,
    user_id uuid,
    reason text,
    payload jsonb,
    foreign key (room_id) references room(id),
    foreign key (user_id) references usr(id)
);
