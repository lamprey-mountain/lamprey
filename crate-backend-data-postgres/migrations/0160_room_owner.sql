alter table room add column owner_id uuid;

alter table room
add constraint fk_room_owner
foreign key (id, owner_id) references room_member(room_id, user_id);

update room set owner_id = (
    select user_id
    from room_member
    where room_id = room.id
    order by joined_at
    limit 1
);
