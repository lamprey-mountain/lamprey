insert into room_member (user_id, room_id, membership, joined_at, mute, deaf)
select id, '00000000-0000-7000-0000-736572766572', 'Join', extract_timestamp_from_uuid_v7(id), false, false from usr
on conflict on constraint room_member_pkey do nothing;
