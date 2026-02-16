create table room_ban (
    room_id uuid,
    user_id uuid,
    reason text,
    created_at timestamp not null,
    expires_at timestamp,
    primary key (room_id, user_id),
    foreign key (room_id) references room(id),
    foreign key (user_id) references usr(id)
);
