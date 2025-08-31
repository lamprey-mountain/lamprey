select role.position from room_member
join role_member on role_member.user_id = room_member.user_id
join role on role_member.role_id = role.id and role.room_id = room_member.room_id
where room_member.room_id = $1 and room_member.user_id = $2
