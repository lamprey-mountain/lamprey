-- re-add accidentally removed channel_id and room_id
alter table inbox add column channel_id uuid;
alter table inbox add column room_id uuid;

update inbox set
    channel_id = (info->>'channel_id')::uuid,
    room_id = (info->>'room_id')::uuid;

create index inbox_channel_id_idx on inbox (user_id, channel_id);
create index inbox_room_id_idx on inbox (user_id, room_id);
create index inbox_user_id_idx on inbox (user_id);
